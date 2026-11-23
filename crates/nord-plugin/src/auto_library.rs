//! Headless program loading from the user's saved `.ns4b` sound library (`library_setting`), for
//! hosts that never open the editor (SoundCraft on Windows has no VST3 editor windows) and for a
//! reloaded project: the `library_program` parameter names a location, a worker thread decodes
//! that program with the piano and sample banks it references, and hands them to `process()`
//! through the same load queue the editor uses (`loader.rs`, D218).
//!
//! The audio thread only sends the wanted location (a `Copy` through a bounded channel: no
//! allocation, no lock) and later drains the ready values. Everything that reads or decodes runs
//! here. The WAVE stage is switched on once the OS update tables are installed (`wave_tables.rs`).

use crate::library_select::{label, location};
use crate::loader::{LoadJob, LoadSender};
use crate::shared_sounds::Shared;
use crossbeam_channel::{bounded, Receiver, Sender};
use nord_core::cbin::{CbinFile, Ns4BackupArchive, ProgramDecoded, SlotIndex};
use nord_core::piano::NpnoSound;
use nord_core::synth::SynthSampleSound;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Requests the audio thread may queue before the oldest are dropped (only the last one counts).
const REQUEST_CAPACITY: usize = 8;
/// How long a load waits for `process()` to drain the queue before it gives up.
const QUEUE_WAIT: Duration = Duration::from_secs(30);
/// How long the first load waits for the OS update tables to finish installing.
const WAVE_TABLES_WAIT: Duration = Duration::from_secs(5);

/// Decoded banks shared by every instance of this process (`shared_sounds.rs`), by
/// `<library path>|<entry>`.
static PIANOS: Shared<NpnoSound> = Shared::new();
static SAMPLES: Shared<SynthSampleSound> = Shared::new();

/// The plugin's end of the worker: sends wanted locations.
pub struct LibraryLink {
    tx: Sender<i32>,
    /// Every request value the worker finished with (loaded, skipped or failed), for
    /// [`LibraryLink::wait_handled`].
    done_rx: Receiver<i32>,
}

impl LibraryLink {
    /// Starts the worker for the library at `path`, or returns `None` when there is none (or the
    /// thread cannot start). Nothing is read until the first request.
    pub fn start(path: Option<PathBuf>, loads: LoadSender) -> Option<Self> {
        let Some(path) = path else {
            report("no sound library set (ns4 library set <backup.ns4b>)");
            return None;
        };
        let (tx, rx) = bounded(REQUEST_CAPACITY);
        let (done_tx, done_rx) = bounded(REQUEST_CAPACITY);
        let worker = Worker {
            path,
            loads,
            done: done_tx,
            archive: None,
            pianos: HashMap::new(),
            samples: HashMap::new(),
            wave_bank_sent: false,
        };
        std::thread::Builder::new()
            .name("ns4-library".into())
            .spawn(move || worker.run(rx))
            .ok()?;
        Some(Self { tx, done_rx })
    }

    /// Blocks until the worker has finished with a request for `program` (its jobs are queued, or
    /// it was skipped or failed), at most `timeout`. For an offline render (a bounce), whose
    /// first block comes right after `initialize`: the program must already be in. `false` on
    /// timeout. Never call it from the audio thread.
    pub fn wait_handled(&self, program: i32, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            match self.done_rx.recv_timeout(deadline - now) {
                Ok(v) if v == program => return true,
                Ok(_) => continue,
                Err(_) => return false,
            }
        }
    }

    /// Asks for parameter value `program` (0 = Off: nothing happens). Never blocks or allocates;
    /// `false` when the request queue is full (the worker is behind; the next request wins).
    pub fn request(&self, program: i32) -> bool {
        self.tx.try_send(program).is_ok()
    }
}

/// Where the worker's messages go besides stderr (a DAW shows neither): `plugin.log` next to
/// the saved settings (`%APPDATA%\ns4\plugin.log` on Windows), appended, never rotated.
pub fn log_path() -> Option<PathBuf> {
    Some(
        nord_core::user_config::default_config_path()?
            .parent()?
            .join("plugin.log"),
    )
}

/// Prints `msg` on stderr and appends it to [`log_path`] (silently when that fails).
pub(crate) fn report(msg: &str) {
    eprintln!("nd matrix 1: {msg}");
    if let Some(p) = log_path() {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(p)
        {
            let _ = writeln!(f, "{msg}");
        }
    }
}

/// Makes this process ignore the saved library (`NS4_LIBRARY=none`): tests, and tools that
/// host the plugin without wanting a multi-second load.
pub fn ignore_saved_library() {
    std::env::set_var(
        nord_core::library_setting::LIBRARY_ENV,
        nord_core::library_setting::NO_LIBRARY,
    );
}

/// The saved library of this user: `NS4_LIBRARY`, else the path `ns4 library set` stored.
pub fn saved_library() -> Option<PathBuf> {
    let env = std::env::var(nord_core::library_setting::LIBRARY_ENV).ok();
    let config = nord_core::user_config::default_config_path();
    nord_core::library_setting::effective_path(env.as_deref(), config.as_deref())
}

/// A sound a program needs, by archive entry; `None` when the layer is off.
type Wanted = Option<String>;

/// The indexed archive and the program entry at each flat location (`bank * 64 + location`).
type Indexed = (Arc<Ns4BackupArchive>, Vec<(u16, String)>);

struct Worker {
    path: PathBuf,
    loads: LoadSender,
    /// Reports each handled request value (`LibraryLink::wait_handled`).
    done: Sender<i32>,
    /// The library, indexed on the first request.
    archive: Option<Indexed>,
    /// Banks of the current program, kept alive here (the process-wide `PIANOS` / `SAMPLES`
    /// hold only weak references; a factory piano is hundreds of megabytes).
    pianos: HashMap<String, Arc<NpnoSound>>,
    samples: HashMap<String, Arc<SynthSampleSound>>,
    wave_bank_sent: bool,
}

impl Worker {
    fn run(mut self, rx: Receiver<i32>) {
        while let Ok(mut want) = rx.recv() {
            // Only the newest request matters when the host scrubbed through several.
            while let Ok(later) = rx.try_recv() {
                want = later;
            }
            if let Some(flat) = location(want) {
                match self.load(flat) {
                    Ok(name) => report(&format!("program {} loaded: {name}", label(want))),
                    Err(e) => report(&format!("program {} not loaded ({e})", label(want))),
                }
            }
            // Nobody waiting (or a full report queue) is fine: the report is advisory.
            let _ = self.done.try_send(want);
        }
    }

    fn open(&mut self) -> Result<&Indexed, String> {
        if self.archive.is_none() {
            let (archive, slots) = index(&self.path)?;
            self.archive = Some((Arc::new(archive), slots));
        }
        self.archive
            .as_ref()
            .ok_or_else(|| "library not open".to_string())
    }

    /// Decodes the program at `flat` and its sounds, then queues them. Returns the program name.
    fn load(&mut self, flat: u16) -> Result<String, String> {
        let (archive, slots) = self.open()?.clone();
        let entry = slots
            .iter()
            .find(|(l, _)| *l == flat)
            .map(|(_, n)| n.clone())
            .ok_or_else(|| "no program at that location in the library".to_string())?;
        let prog = decode_program(&self.path, &archive, &entry)?;
        let name = prog.name.clone();
        let (pianos, samples) = wanted_sounds(&archive, &prog);

        let mut jobs: Vec<LoadJob> = Vec::with_capacity(7);
        jobs.push(LoadJob::Program(Box::new(prog)));
        for (slot_b, want) in [(false, &pianos[0]), (true, &pianos[1])] {
            let sound = match want {
                Some(e) => self.piano(&archive, e),
                None => None,
            };
            jobs.push(LoadJob::Piano { slot_b, sound });
        }
        for (layer, want) in samples.iter().enumerate() {
            let sound = match want {
                Some(e) => self.sample(&archive, e),
                None => None,
            };
            jobs.push(LoadJob::Sample { layer, sound });
        }
        // Keep only what this program uses (the engine holds its own `Arc`s).
        self.pianos
            .retain(|k, _| pianos.iter().flatten().any(|w| w == k));
        self.samples
            .retain(|k, _| samples.iter().flatten().any(|w| w == k));

        if !self.wave_bank_sent && wave_tables_ready() {
            nord_core::synth::prepare_wave_bank();
            jobs.push(LoadJob::WaveBank(true));
            self.wave_bank_sent = true;
        }
        for job in jobs {
            self.push(job)?;
        }
        Ok(name)
    }

    fn piano(&mut self, archive: &Ns4BackupArchive, entry: &str) -> Option<Arc<NpnoSound>> {
        if let Some(s) = self.pianos.get(entry) {
            return Some(Arc::clone(s));
        }
        let key = format!("{}|{entry}", self.path.display());
        let decoded = PIANOS.get_or_decode(&key, || {
            let mut sound = read_entry(&self.path, archive, entry)
                .and_then(|b| NpnoSound::from_npno_bytes(&b))?;
            if sound.name.is_empty() {
                sound.name = short_name(entry).to_string();
            }
            Ok(sound)
        });
        match decoded {
            Ok(sound) => {
                self.pianos.insert(entry.to_string(), Arc::clone(&sound));
                Some(sound)
            }
            Err(e) => {
                report(&format!("piano '{entry}' not loaded ({e})"));
                None
            }
        }
    }

    fn sample(&mut self, archive: &Ns4BackupArchive, entry: &str) -> Option<Arc<SynthSampleSound>> {
        if let Some(s) = self.samples.get(entry) {
            return Some(Arc::clone(s));
        }
        let key = format!("{}|{entry}", self.path.display());
        let decoded = SAMPLES.get_or_decode(&key, || {
            let mut sound = read_entry(&self.path, archive, entry)
                .and_then(|b| SynthSampleSound::from_nsmp4_bytes(&b).map_err(|e| e.to_string()))?;
            if sound.name.is_empty() {
                sound.name = short_name(entry).to_string();
            }
            Ok(sound)
        });
        match decoded {
            Ok(sound) => {
                self.samples.insert(entry.to_string(), Arc::clone(&sound));
                Some(sound)
            }
            Err(e) => {
                report(&format!("sample bank '{entry}' not loaded ({e})"));
                None
            }
        }
    }

    /// Queues one job, waiting for `process()` to make room (the queue is drained only while
    /// the host runs the plugin).
    fn push(&self, mut job: LoadJob) -> Result<(), String> {
        let deadline = Instant::now() + QUEUE_WAIT;
        loop {
            match self.loads.push(job) {
                Ok(()) => return Ok(()),
                Err(back) => {
                    if Instant::now() > deadline {
                        return Err("the plugin is not processing audio".to_string());
                    }
                    job = back;
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        }
    }
}

/// Indexes the archive: the entry at each flat location (`bank * 64 + location`).
fn index(path: &Path) -> Result<(Ns4BackupArchive, Vec<(u16, String)>), String> {
    let archive = Ns4BackupArchive::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut reader =
        BufReader::new(File::open(path).map_err(|e| format!("{}: {e}", path.display()))?);
    let slots = SlotIndex::read(&archive, &mut reader)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .programs
        .into_iter()
        .filter(|(b, l, _)| *b < 8 && *l < 64)
        .map(|(b, l, n)| (b * 64 + l, n))
        .collect();
    Ok((archive, slots))
}

fn read_entry(path: &Path, archive: &Ns4BackupArchive, entry: &str) -> Result<Vec<u8>, String> {
    let mut r = BufReader::new(File::open(path).map_err(|e| format!("{}: {e}", path.display()))?);
    archive.read_entry(&mut r, entry).map_err(|e| e.to_string())
}

/// The program entry decoded, named after its file when the stored name is the generic one
/// (the same rule as the editor's library).
fn decode_program(
    path: &Path,
    archive: &Ns4BackupArchive,
    entry: &str,
) -> Result<ProgramDecoded, String> {
    let bytes = read_entry(path, archive, entry)?;
    let mut prog = CbinFile::parse(&bytes)
        .and_then(|c| c.decode_program())
        .map_err(|e| e.to_string())?;
    let generic = prog.name.is_empty()
        || prog.name == "Nord Stage 4 Program"
        || prog.name.ends_with(" Program");
    if generic {
        prog.name = short_name(entry).to_string();
    }
    Ok(prog)
}

/// The piano (A, B) and sample (A, B, C) entries `prog` needs: a layer that is enabled, or is in
/// a scene of the program, or when the program has no scene masks at all (the editor's rule).
fn wanted_sounds(archive: &Ns4BackupArchive, prog: &ProgramDecoded) -> ([Wanted; 2], [Wanted; 3]) {
    let refs = archive.resolve_program_sounds(prog);
    let mask = prog.global.scene_1_mask | prog.global.scene_2_mask;
    let needed = |enabled: bool, bit: u8| enabled || mask == 0 || mask & (1 << bit) != 0;
    let pick =
        |enabled: bool, bit: u8, entry: Option<String>| entry.filter(|_| needed(enabled, bit));
    (
        [
            pick(prog.piano.layer_a.enabled, 2, refs.piano_a),
            pick(prog.piano.layer_b.enabled, 3, refs.piano_b),
        ],
        [
            pick(prog.synth.layer_a.enabled, 4, refs.synth_a),
            pick(prog.synth.layer_b.enabled, 5, refs.synth_b),
            pick(prog.synth.layer_c.enabled, 6, refs.synth_c),
        ],
    )
}

fn short_name(entry: &str) -> &str {
    Path::new(entry)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Program")
}

/// Whether the OS update tables are in: waits a little for the install thread
/// (`wave_tables::install_saved_in_background`) when a file is configured, so the first program
/// already plays its WAVE sounds from the real tables.
fn wave_tables_ready() -> bool {
    if nord_core::synth::wave_tables_installed() {
        return true;
    }
    let env = std::env::var(crate::wave_tables::WAVE_TABLES_ENV).ok();
    let config = nord_core::user_config::default_config_path();
    if crate::wave_tables::saved_path(env.as_deref(), config.as_deref()).is_none() {
        return false;
    }
    let deadline = Instant::now() + WAVE_TABLES_WAIT;
    while Instant::now() < deadline {
        if nord_core::synth::wave_tables_installed() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::loader::load_queue;

    /// A stored PKZIP archive (uncompressed members), enough for `Ns4BackupArchive`.
    pub(crate) fn stored_zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut central = Vec::new();
        for (name, data) in files {
            let offset = out.len() as u32;
            let (n, size) = (name.len() as u16, data.len() as u32);
            let crc = crc32(data);
            out.extend_from_slice(&[0x50, 0x4B, 0x03, 0x04, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&size.to_le_bytes());
            out.extend_from_slice(&size.to_le_bytes());
            out.extend_from_slice(&n.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(data);
            central
                .extend_from_slice(&[0x50, 0x4B, 0x01, 0x02, 20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            central.extend_from_slice(&crc.to_le_bytes());
            central.extend_from_slice(&size.to_le_bytes());
            central.extend_from_slice(&size.to_le_bytes());
            central.extend_from_slice(&n.to_le_bytes());
            central.extend_from_slice(&[0; 12]);
            central.extend_from_slice(&offset.to_le_bytes());
            central.extend_from_slice(name.as_bytes());
        }
        let (cd_offset, cd_size, count) =
            (out.len() as u32, central.len() as u32, files.len() as u16);
        out.extend_from_slice(&central);
        out.extend_from_slice(&[0x50, 0x4B, 0x05, 0x06, 0, 0, 0, 0]);
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&cd_size.to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }

    fn crc32(data: &[u8]) -> u32 {
        let mut c = 0xFFFF_FFFFu32;
        for b in data {
            c ^= u32::from(*b);
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    (c >> 1) ^ 0xEDB8_8320
                } else {
                    c >> 1
                };
            }
        }
        !c
    }

    /// A backup with one organ program at A:11 (bank 0, location 0) and one at A:12.
    pub(crate) fn two_program_backup(name: &str) -> PathBuf {
        let organ = |name: &str, bars: [u8; 9]| {
            let mut p = ProgramDecoded {
                name: name.into(),
                ..Default::default()
            };
            p.organ.drawbars_a = bars;
            p
        };
        let first = organ("First", [8, 8, 8, 0, 0, 0, 0, 0, 0]);
        let second = organ("Second", [0, 0, 0, 0, 0, 0, 0, 0, 8]);
        let a = first.to_cbin_file(0, 0).expect("encode").to_bytes();
        let b = second.to_cbin_file(0, 1).expect("encode").to_bytes();
        let path =
            std::env::temp_dir().join(format!("nd-autolib-{name}-{}.ns4b", std::process::id()));
        std::fs::write(
            &path,
            stored_zip(&[("Programs/First.ns4p", &a), ("Programs/Second.ns4p", &b)]),
        )
        .unwrap();
        path
    }

    fn drain_until(rx: &crate::loader::LoadReceiver, n: usize) -> Vec<LoadJob> {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut out = Vec::new();
        while out.len() < n && Instant::now() < deadline {
            match rx.pop() {
                Some(j) => out.push(j),
                None => std::thread::sleep(Duration::from_millis(10)),
            }
        }
        out
    }

    #[test]
    fn a_request_decodes_the_program_at_that_location_with_empty_sound_slots() {
        let path = two_program_backup("load");
        let (tx, rx) = load_queue(16);
        let link = LibraryLink::start(Some(path.clone()), tx).expect("worker");
        assert!(link.request(2));
        let jobs = drain_until(&rx, 6);
        let _ = std::fs::remove_file(&path);
        assert_eq!(jobs.len(), 6, "program + 2 pianos + 3 samples");
        match &jobs[0] {
            LoadJob::Program(p) => {
                assert_eq!(p.name, "Second");
                assert_eq!(p.organ.drawbars_a[8], 8);
            }
            other => panic!("first job is not the program: {other:?}"),
        }
        assert!(matches!(
            jobs[1],
            LoadJob::Piano {
                slot_b: false,
                sound: None
            }
        ));
        assert!(matches!(
            jobs[2],
            LoadJob::Piano {
                slot_b: true,
                sound: None
            }
        ));
        assert!(matches!(
            jobs[5],
            LoadJob::Sample {
                layer: 2,
                sound: None
            }
        ));
    }

    #[test]
    fn wait_handled_returns_once_the_jobs_are_queued_and_times_out_otherwise() {
        let path = two_program_backup("wait");
        let (tx, rx) = load_queue(16);
        let link = LibraryLink::start(Some(path.clone()), tx).expect("worker");
        assert!(
            !link.wait_handled(1, Duration::from_millis(50)),
            "nothing requested"
        );
        assert!(link.request(1));
        assert!(link.wait_handled(1, Duration::from_secs(10)));
        let _ = std::fs::remove_file(&path);
        assert_eq!(
            rx.pop().map(|j| matches!(j, LoadJob::Program(_))),
            Some(true)
        );
        // Off is handled too (nothing queued), so an offline render never waits on it.
        assert!(link.request(0));
        assert!(link.wait_handled(0, Duration::from_secs(10)));
    }

    #[test]
    fn off_and_missing_locations_queue_nothing_and_the_newest_request_wins() {
        let path = two_program_backup("skip");
        let (tx, rx) = load_queue(16);
        let link = LibraryLink::start(Some(path.clone()), tx).expect("worker");
        assert!(link.request(0));
        assert!(link.request(300));
        std::thread::sleep(Duration::from_millis(300));
        assert!(rx.is_empty(), "Off and an empty location load nothing");
        assert!(link.request(2));
        assert!(link.request(1));
        let jobs = drain_until(&rx, 6);
        let _ = std::fs::remove_file(&path);
        match jobs.first() {
            Some(LoadJob::Program(p)) => assert_eq!(p.name, "First"),
            other => panic!("{other:?}"),
        }
        // The worker may have served both or only the newest; either way the last is "First".
        let names: Vec<String> = jobs
            .iter()
            .filter_map(|j| match j {
                LoadJob::Program(p) => Some(p.name.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(names.last().map(String::as_str), Some("First"));
    }

    #[test]
    fn no_library_means_no_worker_and_a_bad_file_is_reported_not_fatal() {
        let (tx, rx) = load_queue(4);
        assert!(LibraryLink::start(None, tx.clone()).is_none());
        let bad = std::env::temp_dir().join(format!("nd-autolib-bad-{}.ns4b", std::process::id()));
        std::fs::write(&bad, b"nope").unwrap();
        let link = LibraryLink::start(Some(bad.clone()), tx).expect("worker");
        assert!(link.request(1));
        std::thread::sleep(Duration::from_millis(300));
        let _ = std::fs::remove_file(&bad);
        assert!(rx.is_empty());
        assert!(index(Path::new("definitely-missing.ns4b")).is_err());
    }

    #[test]
    fn ignoring_the_saved_library_leaves_no_path() {
        ignore_saved_library();
        assert_eq!(saved_library(), None);
    }

    #[test]
    fn the_log_sits_next_to_the_settings() {
        let p = log_path();
        if let Some(p) = p {
            assert_eq!(p.file_name().and_then(|s| s.to_str()), Some("plugin.log"));
            assert_eq!(
                p.parent(),
                nord_core::user_config::default_config_path()
                    .as_deref()
                    .and_then(Path::parent)
            );
        }
    }

    #[test]
    fn wanted_sounds_follow_the_layer_enables_and_scene_masks() {
        let path = two_program_backup("wanted");
        let (archive, slots) = index(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0].0, 0);
        assert_eq!(slots[1].0, 1);
        let prog = ProgramDecoded::default();
        let (p, s) = wanted_sounds(&archive, &prog);
        assert!(p.iter().all(Option::is_none) && s.iter().all(Option::is_none));
        assert_eq!(short_name("Programs/Grand Piano.ns4p"), "Grand Piano");
    }
}

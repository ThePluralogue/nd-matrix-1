//! Test fixtures shared by the `auto_library` submodules.

use crate::loader::{LoadJob, LoadReceiver};
use nord_core::cbin::{stored_archive, ProgramDecoded};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// A backup on disk with one organ program at A:11 (bank 0, location 0) and one at A:12.
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
    let path = std::env::temp_dir().join(format!("nd-autolib-{name}-{}.ns4b", std::process::id()));
    let bytes = stored_archive(&[("Programs/First.ns4p", &a), ("Programs/Second.ns4p", &b)])
        .expect("archive");
    std::fs::write(&path, bytes).unwrap();
    path
}

/// Pops up to `n` jobs, waiting at most 10 s for them.
pub(crate) fn drain_until(rx: &LoadReceiver, n: usize) -> Vec<LoadJob> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::load_queue;

    #[test]
    fn the_fixture_is_a_readable_backup_and_the_drain_stops_at_the_count() {
        let path = two_program_backup("support");
        let archive = nord_core::cbin::Ns4BackupArchive::open(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(archive.entries().len(), 2);
        let (tx, rx) = load_queue(4);
        tx.push(LoadJob::WaveBank(true)).unwrap();
        tx.push(LoadJob::WaveBank(false)).unwrap();
        assert_eq!(drain_until(&rx, 1).len(), 1);
        assert_eq!(drain_until(&rx, 1).len(), 1);
    }
}

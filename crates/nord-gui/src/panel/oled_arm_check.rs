//! Pixel check of our OLED painters against frames the real firmware drew in the ARM harness.
//!
//! The ARM frames are firmware output (never committed): produce them with
//! `python tools/arm_screens.py press <CODE> <SRAM> --state target/oled/state.bin --key 0x01,0,3
//! --out target/oled/icons/press` (home, display 0) and `--key 0x01,1,6 --out target/oled/syn/press`
//! (Synth waveform page, display 1), then run
//! `NS4_WAVE_TABLES=<bank0_CODE.bin or the update file> NS4_OLED_FRAMES=target/oled
//!  cargo test -p nord-gui --lib oled_arm_check -- --nocapture`.
//! Without both variables the frame tests print `SKIPPED` and return (the data is local only).
//!
//! Our shapes are rasterised on a 128 x 64 rectangle at one screen pixel per display pixel: a
//! pixel is lit when the last shape covering its centre is a rectangle filled with the OLED
//! foreground colour. Text lines are not shapes here because the firmware fonts are installed.

use super::bay_program::oled::{compose, Summary};
use super::bay_program::oled_paint;
use super::bay_synth::oled_fw;
use crate::panel::prelude::*;
use crate::panel::ui_state::ProgramUi;
use egui::epaint::Shape;
use egui::Pos2;
use std::path::PathBuf;

const W: usize = 128;
const H: usize = 64;
const FG: Color32 = Color32::from_rgb(150, 190, 230);

/// A 128 x 64 monochrome frame, row-major.
type Frame = Vec<bool>;

fn paint_shape(frame: &mut Frame, origin: Pos2, shape: &Shape) {
    match shape {
        Shape::Vec(v) => v.iter().for_each(|s| paint_shape(frame, origin, s)),
        Shape::Rect(r) => {
            let lit = r.fill == FG;
            for y in 0..H {
                for x in 0..W {
                    let c = origin + vec2(x as f32 + 0.5, y as f32 + 0.5);
                    if r.rect.contains(c) {
                        frame[y * W + x] = lit;
                    }
                }
            }
        }
        Shape::LineSegment { points, stroke } if points[0].y == points[1].y => {
            // a horizontal rule: the row nearest to the line, lit
            let y = (points[0].y - origin.y).floor();
            let (x0, x1) = (points[0].x.min(points[1].x), points[0].x.max(points[1].x));
            if (0.0..H as f32).contains(&y) && stroke.color == FG {
                for x in 0..W {
                    let cx = origin.x + x as f32 + 0.5;
                    if cx >= x0 - 0.5 && cx <= x1 + 0.5 {
                        frame[y as usize * W + x] = true;
                    }
                }
            }
        }
        _ => {}
    }
}

/// Runs `body` with a painter and a 128 x 64 rectangle and rasterises what it painted.
fn raster(body: impl FnOnce(&Painter, Rect)) -> Frame {
    let ctx = egui::Context::default();
    let mut body = Some(body);
    let mut origin = Pos2::ZERO;
    let out = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(W as f32, H as f32), Sense::hover());
            origin = rect.min;
            if let Some(f) = body.take() {
                f(&ui.painter().clone(), rect);
            }
        });
    });
    let mut frame = vec![false; W * H];
    for s in &out.shapes {
        paint_shape(&mut frame, origin, &s.shape);
    }
    frame
}

/// Parses the harness dump (`#` lit, `.` dark, 64 lines of 128).
fn parse_dump(text: &str) -> Option<Frame> {
    let rows: Vec<&str> = text.lines().filter(|l| l.len() == W).collect();
    (rows.len() == H).then(|| {
        rows.iter()
            .flat_map(|r| r.chars().map(|c| c == '#'))
            .collect()
    })
}

fn dump(f: &Frame) -> String {
    (0..H)
        .map(|y| {
            (0..W)
                .map(|x| if f[y * W + x] { '#' } else { '.' })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Number of differing pixels inside `region` = `(x0, y0, x1, y1)` (end exclusive).
fn diff(a: &Frame, b: &Frame, (x0, y0, x1, y1): (usize, usize, usize, usize)) -> usize {
    (y0..y1)
        .flat_map(|y| (x0..x1).map(move |x| (x, y)))
        .filter(|&(x, y)| a[y * W + x] != b[y * W + x])
        .count()
}

/// Side by side view of the lines that differ (ours | ARM).
fn show_diff(ours: &Frame, arm: &Frame, rows: std::ops::Range<usize>) -> String {
    rows.map(|y| {
        let row = |f: &Frame| {
            (0..W)
                .map(|x| if f[y * W + x] { '#' } else { '.' })
                .collect::<String>()
        };
        format!("{y:2} {}\n   {}", row(ours), row(arm))
    })
    .collect::<Vec<_>>()
    .join("\n")
}

/// Fonts and frames, or `None` (and a `SKIPPED` line) without the local data.
fn local_data(name: &str) -> Option<Frame> {
    let dir = PathBuf::from(std::env::var("NS4_OLED_FRAMES").ok()?);
    if std::env::var("NS4_WAVE_TABLES").is_err() {
        eprintln!("SKIPPED {name}: NS4_WAVE_TABLES not set");
        return None;
    }
    if nord_core::oled_font::install_from_env().ok()? == 0 {
        return None;
    }
    let file = if name.starts_with("home") {
        "icons/press/press_d0.txt"
    } else {
        "syn/press/press_d1.txt"
    };
    let text = std::fs::read_to_string(dir.join(file)).ok().or_else(|| {
        eprintln!("SKIPPED {name}: no {file} under {}", dir.display());
        None
    })?;
    parse_dump(&text)
}

#[test]
fn the_home_screen_matches_the_arm_frame() {
    let Some(arm) = local_data("home") else {
        eprintln!("SKIPPED home: NS4_OLED_FRAMES / NS4_WAVE_TABLES");
        return;
    };
    let ui = ProgramUi {
        title: "<empty>".into(),
        ..Default::default()
    };
    // the harness program: an organ B3, no piano sound, a synth "Pure Saw"; the home view
    let sum = Summary {
        organ: "B3".into(),
        piano: String::new(),
        synth: "Pure Saw".into(),
        layers_on: [true, false, true, false, true, false, false],
        ..Default::default()
    };
    let text = compose(&ui, &sum);
    let ours = raster(|p, rect| oled_paint::paint(p, rect, &text, false));
    let regions = [
        ("title A:11", (0, 0, W, 17)),
        ("name <empty>", (0, 17, W, 33)),
        ("rule", (0, 33, W, 35)),
        ("organ icon", (0, 36, 9, 45)),
        ("piano row", (0, 45, W, 55)),
        ("synth icon", (0, 55, 9, 64)),
    ];
    let mut bad = Vec::new();
    for (n, r) in regions {
        let d = diff(&ours, &arm, r);
        eprintln!("home {n}: {d} differing pixels");
        if d > 0 {
            bad.push((n, d));
        }
    }
    if !bad.is_empty() {
        eprintln!("{}", show_diff(&ours, &arm, 0..64));
    }
    assert!(bad.is_empty(), "differs from the ARM frame: {bad:?}");
}

#[test]
fn the_synth_waveform_page_matches_the_arm_frame() {
    let Some(arm) = local_data("synth") else {
        eprintln!("SKIPPED synth: NS4_OLED_FRAMES / NS4_WAVE_TABLES");
        return;
    };
    use crate::panel::bay_synth::oled_types::{Graphic, Icon, OledView, Soft};
    let soft = |value: &str, label: &'static str| {
        Some(Soft {
            value: value.into(),
            label,
            dim: false,
        })
    };
    let view = OledView {
        title: "OSC WAVEFORM",
        layer: 'A',
        graphic: Graphic::Wave {
            name: "Pure Saw".into(),
            ctrl: String::new(),
            icon: Icon::Saw,
        },
        soft: [
            soft("Analog", "TYPE"),
            soft("Pure", "CAT"),
            soft("3 (7)", "WAVE"),
        ],
    };
    let ours = raster(|p, rect| {
        oled_fw::paint(p, rect, &view, "OSC WAVEFORM");
    });
    let regions = [
        ("title bar", (0, 0, W, 11)),
        ("wave name", (0, 11, 86, 26)),
        ("picture", (86, 26, W, 48)),
        ("values", (0, 48, W, 56)),
        ("tabs", (0, 56, W, 64)),
    ];
    let mut bad = Vec::new();
    for (n, r) in regions {
        let d = diff(&ours, &arm, r);
        eprintln!("synth {n}: {d} differing pixels");
        if d > 0 {
            bad.push((n, d));
        }
    }
    if !bad.is_empty() {
        eprintln!("{}", show_diff(&ours, &arm, 0..64));
    }
    assert!(bad.is_empty(), "differs from the ARM frame: {bad:?}");
}

#[test]
fn the_rasteriser_and_the_dump_parser_agree_with_each_other() {
    let f = raster(|p, rect| {
        p.rect_filled(
            Rect::from_min_size(rect.min + vec2(3.0, 2.0), vec2(2.0, 1.0)),
            0.0,
            FG,
        );
    });
    assert!(f[2 * W + 3] && f[2 * W + 4] && !f[2 * W + 5] && !f[3 * W + 3]);
    assert_eq!(parse_dump(&dump(&f)), Some(f));
    assert_eq!(parse_dump("short"), None);
    assert_eq!(
        diff(&vec![false; W * H], &vec![true; W * H], (0, 0, 2, 3)),
        6
    );
}

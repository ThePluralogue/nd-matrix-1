//! The text size option (issue #592) in the editor frame: the bar, the plate and the Studio
//! Settings dialog at 100 %, 150 % and 200 %. Headless: egui builds the real shapes, the checks
//! read the painted text rectangles (no GPU, no golden bitmap; the numbers below are the golden).

use super::draw::{draw_editor, size_for_width, EditorState, DEFAULT_SIZE, PLATE_H, PLATE_W};
use super::index::ParamIndex;
use super::notes::note_queue;
use super::sink::test_support::Recorder;
use crate::loader::load_queue;
use crate::params::Stage4Params;
use nih_plug_egui::egui::{self, Rect, Shape};
use nih_plug_egui::EguiState;
use nord_gui::debug_dialog::DebugTab;
use nord_gui::text_scale::{self, TextScale};
use std::sync::Arc;

fn editor(percent: u32) -> EditorState {
    let params = Arc::new(Stage4Params::default());
    let index = Arc::new(ParamIndex::new(params.as_ref()).with_program_hold(params));
    let (tx, _notes) = note_queue(8);
    let (load_tx, _load_rx) = load_queue(8);
    let mut st = EditorState::new(index, tx, load_tx, Default::default());
    st.text_scale = TextScale::new(percent);
    st
}

fn window(_percent: u32) -> egui::Vec2 {
    // The editor keeps the plate aspect; the default width is wide enough for the bar at 200 %.
    let (w, h) = size_for_width(DEFAULT_SIZE.0 as f32);
    egui::vec2(w as f32, h as f32)
}

fn frame(
    ctx: &egui::Context,
    st: &mut EditorState,
    size: egui::Vec2,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let egui_state = EguiState::from_size(size.x as u32, size.y as u32);
    let modifiers = events
        .iter()
        .find_map(|e| match e {
            egui::Event::Key { modifiers, .. } => Some(*modifiers),
            _ => None,
        })
        .unwrap_or_default();
    let input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, size)),
        modifiers,
        events,
        ..Default::default()
    };
    ctx.run(input, |ctx| {
        draw_editor(ctx, &egui_state, st, Recorder::default())
    })
}

struct Label {
    text: String,
    rect: Rect,
    clip: Rect,
}

/// Every painted text with its rectangle (galley rect at its position) and clip rectangle.
fn labels(out: &egui::FullOutput) -> Vec<Label> {
    let mut v = Vec::new();
    for s in &out.shapes {
        if let Shape::Text(t) = &s.shape {
            v.push(Label {
                text: t.galley.text().to_string(),
                rect: t.galley.rect.translate(t.pos.to_vec2()),
                clip: s.clip_rect,
            });
        }
    }
    v
}

fn settled(percent: u32) -> (egui::Context, EditorState, egui::FullOutput) {
    let ctx = egui::Context::default();
    let mut st = editor(percent);
    let size = window(percent);
    let mut out = frame(&ctx, &mut st, size, vec![]);
    for _ in 0..4 {
        out = frame(&ctx, &mut st, size, vec![]);
    }
    (ctx, st, out)
}

/// Labels that belong to the bar (above the plate).
fn bar_labels(out: &egui::FullOutput, bar_h: f32) -> Vec<Label> {
    labels(out)
        .into_iter()
        .filter(|l| l.rect.center().y < bar_h)
        .collect()
}

#[test]
fn bar_text_stays_whole_and_apart_at_every_size() {
    for percent in [100, 125, 150, 175, 200] {
        let (_ctx, _st, out) = settled(percent);
        let bar_h = text_scale::bar_height();
        let size = window(percent);
        let screen = Rect::from_min_size(egui::Pos2::ZERO, size);
        let bar = bar_labels(&out, bar_h);
        assert!(
            bar.len() >= 10,
            "{percent} %: only {} bar labels",
            bar.len()
        );
        for l in &bar {
            assert!(
                screen.contains_rect(l.rect),
                "{percent} %: {:?} sticks out {:?}",
                l.text,
                l.rect
            );
            assert!(
                l.clip.expand(0.5).contains_rect(l.rect),
                "{percent} %: {:?} is clipped by {:?} ({:?})",
                l.text,
                l.clip,
                l.rect
            );
            assert!(
                l.rect.height() <= bar_h,
                "{percent} %: {:?} taller than the bar",
                l.text
            );
        }
        for (i, a) in bar.iter().enumerate() {
            for b in &bar[i + 1..] {
                assert!(
                    !a.rect.shrink(0.25).intersects(b.rect.shrink(0.25)),
                    "{percent} %: {:?} overlaps {:?}",
                    a.text,
                    b.text
                );
            }
        }
    }
}

/// Golden: the height of the `DEBUG` pill text is the 9 pt bold pill font times the factor, to the
/// nearest tenth of a point, at each size. Measured with egui's bundled fonts (deterministic on
/// every platform).
#[test]
fn pill_text_grows_with_the_chosen_size() {
    let mut heights = Vec::new();
    for percent in [100, 150, 200] {
        let (_ctx, _st, out) = settled(percent);
        let debug = bar_labels(&out, text_scale::bar_height())
            .into_iter()
            .find(|l| l.text == "DEBUG")
            .unwrap_or_else(|| panic!("{percent} %: no DEBUG pill"));
        heights.push((percent, debug.rect.height()));
    }
    // EMPIRICAL: measured with this test on egui 0.31.1 bundled fonts, 2026-11-24 (text rows are
    // pixel-rounded, hence whole numbers); a font or style change shows up here first.
    assert_eq!(heights, [(100, 10.0), (150, 16.0), (200, 21.0)]);
    let base = heights[0].1;
    for (percent, h) in &heights {
        let want = base * *percent as f32 / 100.0;
        assert!(
            (h - want).abs() <= 1.5,
            "{percent} %: DEBUG text {h:.2} pt high, want about {want:.2} ({heights:?})"
        );
    }
    assert!(
        heights[2].1 > heights[1].1 && heights[1].1 > heights[0].1,
        "{heights:?}"
    );
}

#[test]
fn the_window_keeps_the_plate_aspect_below_the_taller_bar() {
    for percent in [100u32, 150, 200] {
        let _ = settled(percent); // sets the factor of this thread
        let (w, h) = size_for_width(1600.0);
        let bar = text_scale::bar_height();
        assert_eq!(
            h,
            (1600.0 * PLATE_H / PLATE_W + bar).round() as u32,
            "{percent} %"
        );
        assert_eq!(w, 1600);
    }
}

#[test]
fn the_plate_is_magnified_by_the_size_and_starts_at_the_top_left() {
    for percent in [100u32, 150, 200] {
        let (ctx, st, _out) = settled(percent);
        let f = percent as f32 / 100.0;
        assert!((st.panel.baked_zoom.base - f).abs() < 1e-4);
        assert!(
            (st.panel.baked_zoom.zoom - f).abs() < 1e-3,
            "{percent} %: {:?}",
            st.panel.baked_zoom
        );
        let plate: Rect = ctx
            .data(|d| d.get_temp(egui::Id::new("ns4_baked_plate_rect")))
            .expect("plate rect");
        let view_w = window(percent).x;
        // The visible part always fills the window width; the whole plate is f times wider.
        assert!(
            (plate.width() - view_w).abs() < 2.0 || percent == 100,
            "{plate:?}"
        );
        assert!(plate.left().abs() < 1.0, "{percent} %: starts at {plate:?}");
    }
}

#[test]
fn the_dialog_fits_the_editor_at_every_size() {
    for percent in [100, 150, 200] {
        let ctx = egui::Context::default();
        let mut st = editor(percent);
        let size = window(percent);
        let screen = Rect::from_min_size(egui::Pos2::ZERO, size);
        for tab in DebugTab::ALL {
            st.debug_dialog.open_tab(tab);
            for _ in 0..3 {
                let _ = frame(&ctx, &mut st, size, vec![]);
            }
            let rect = ctx
                .memory(|m| m.area_rect(egui::Id::new("ns4_shadcn_debug_embedded")))
                .unwrap_or_else(|| panic!("{percent} % {tab:?}: no dialog window"));
            assert!(
                screen.contains_rect(rect),
                "{percent} % {tab:?}: {rect:?} sticks out of {screen:?}"
            );
        }
    }
}

fn ctrl(key: egui::Key) -> Vec<egui::Event> {
    vec![egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::COMMAND,
    }]
}

#[test]
fn ctrl_plus_minus_zero_change_the_size_and_save_it() {
    let dir = std::env::temp_dir().join(format!("ns4-ts-editor-{}", std::process::id()));
    let cfg = dir.join("ns4").join("config.json");
    let _ = std::fs::remove_dir_all(&dir);
    let ctx = egui::Context::default();
    let mut st = editor(100);
    st.config_path = Some(cfg.clone());
    let size = window(100);
    let _ = frame(&ctx, &mut st, size, vec![]);
    let _ = frame(&ctx, &mut st, size, ctrl(egui::Key::Equals));
    let _ = frame(&ctx, &mut st, size, ctrl(egui::Key::Plus));
    assert_eq!(st.text_scale.percent(), 150);
    assert_eq!(text_scale::load(Some(&cfg)).percent(), 150, "saved");
    let _ = frame(&ctx, &mut st, size, ctrl(egui::Key::Minus));
    assert_eq!(st.text_scale.percent(), 125);
    let _ = frame(&ctx, &mut st, size, ctrl(egui::Key::Num0));
    assert_eq!(st.text_scale.percent(), 100);
    assert_eq!(text_scale::load(Some(&cfg)).percent(), 100);
    // Without Ctrl the same keys do nothing to the text size.
    let plain = vec![egui::Event::Key {
        key: egui::Key::Plus,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }];
    let _ = frame(&ctx, &mut st, size, plain);
    assert_eq!(st.text_scale.percent(), 100);
    let _ = std::fs::remove_dir_all(&dir);
}

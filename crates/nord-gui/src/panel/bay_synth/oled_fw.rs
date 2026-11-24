//! The Synth `OSC WAVEFORM` page laid out on the real 128 x 64 grid with the firmware fonts.
//!
//! VERIFIED (ARM frame of display 1, `tools/arm_screens.py press ... --key 0x01,0,6`, research
//! note `arm-oled-render.md` s16.4 and its follow-up on the Synth page): inverted bar rows 0 to 10
//! with the centred title (8 px font), the wave name in the 11 px font at y = 14, the wave picture
//! in the box x = 87..127, y = 26..47 (a falling ramp with its vertical edge on the left for a
//! saw), the three values in the 5 px font at y = 49 and the three inverted labels at y = 57..63
//! (glyphs `@`, `B`, `C` of the 7 px wide-glyph font = TYPE, CAT, WAVE). EMPIRICAL / ASSUMED: the
//! caption (`DETUNE: 7.2`) placement under the name at y = 28 (a unit photo), the picture of the
//! other waves (only the saw was captured; the others reuse our own wave sketches).
//!
//! Only drawn when the firmware fonts are loaded and the page is the plain waveform page; every
//! other page and the fallback keep the monospace painter of `oled.rs`.

use super::graphics::wave_points_n;
use super::oled_types::{Graphic, Icon, OledView};
use crate::panel::bay_program::oled_text::draw_with;
use crate::panel::prelude::*;
use egui::Pos2;
use nord_core::oled_font::{font, Font};

/// UI font indices (VERIFIED: `arm-oled-render.md` s3 and s16.4).
const F_BAR: usize = 3;
const F_NAME: usize = 6;
const F_SMALL: usize = 8;
const F_GLYPH: usize = 10;
/// Wave picture box (VERIFIED frame): left, top, width, height in display pixels.
const PIC: (i32, i32, i32, i32) = (87, 26, 41, 22);
/// Centres of the three columns (VERIFIED frame: boxes x 7..29, 55..71, 99..121).
const COLS: [i32; 3] = [18, 63, 110];

/// Display pixels to screen pixels on `disp`.
struct Grid {
    disp: Rect,
    cell: f32,
}

impl Grid {
    fn x(&self, v: i32) -> f32 {
        (self.disp.left() + v as f32 * self.cell).round()
    }
    fn y(&self, v: i32) -> f32 {
        (self.disp.top() + v as f32 * self.cell).round()
    }
    fn at(&self, x: i32, y: i32) -> Pos2 {
        pos2(self.x(x), self.y(y))
    }
    /// Filled block of `w` x `h` display pixels with its top-left at `(x, y)`.
    fn block(&self, p: &Painter, (x, y, w, h): (i32, i32, i32, i32), color: Color32) {
        p.rect_filled(
            Rect::from_min_max(self.at(x, y), self.at(x + w, y + h)),
            0.0,
            color,
        );
    }
}

/// Pixels of the line from `a` to `b` (Bresenham), both ends included.
pub(super) fn line_pixels(a: (i32, i32), b: (i32, i32)) -> Vec<(i32, i32)> {
    let (mut x, mut y) = a;
    let (dx, dy) = ((b.0 - a.0).abs(), -(b.1 - a.1).abs());
    let (sx, sy) = (
        if a.0 < b.0 { 1 } else { -1 },
        if a.1 < b.1 { 1 } else { -1 },
    );
    let mut err = dx + dy;
    let mut out = vec![(x, y)];
    while (x, y) != b {
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
        out.push((x, y));
    }
    out
}

/// Wave picture pixels inside a `w` x `h` box (`(0, 0)` = top-left).
///
/// A saw is one falling ramp whose vertical edge is the left side, like the captured frame; a
/// multi saw shows two; the other waves use our sketch over two periods.
pub(super) fn picture_pixels(icon: Icon, w: i32, h: i32) -> Vec<(i32, i32)> {
    let teeth = match icon {
        Icon::Saw => Some(1.0_f32),
        Icon::Multi => Some(2.0),
        _ => None,
    };
    let pts: Vec<(f32, f32)> = match teeth {
        // falling ramps; the phase is held just below 1.0 so the last column is the ramp bottom
        Some(n) => {
            let mut v = vec![(0.0, 0.0)]; // the vertical edge of the first tooth
            v.extend((0..w).map(|i| {
                let x = i as f32 / (w - 1).max(1) as f32;
                (x, 1.0 - (x * n).min(n - 1e-3).fract())
            }));
            v
        }
        None => wave_points_n(icon, w as usize, 2.0),
    };
    let px = |p: (f32, f32)| {
        (
            (p.0 * (w - 1) as f32).round() as i32,
            ((1.0 - p.1) * (h - 1) as f32).round() as i32,
        )
    };
    let mut out = Vec::new();
    for pair in pts.windows(2) {
        out.extend(line_pixels(px(pair[0]), px(pair[1])));
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// Glyph of the 7 px wide-glyph font that spells a soft label (VERIFIED: research s8 / s16.4).
fn label_glyph(label: &str) -> Option<char> {
    match label {
        "TYPE" => Some('@'),
        "CAT" => Some('B'),
        "WAVE" => Some('C'),
        _ => None,
    }
}

/// `ANALOG` -> `Analog`: the firmware prints these values in mixed case (VERIFIED frame:
/// `Analog`, `Pure`); words of one or two letters (`FM`) and strings with other characters stay.
pub(super) fn mixed_case(s: &str) -> String {
    if !s.chars().all(|c| c.is_ascii_uppercase() || c == ' ') {
        return s.to_string();
    }
    s.split(' ')
        .map(|w| match w.len() {
            0..=2 => w.to_string(),
            _ => format!("{}{}", &w[..1], w[1..].to_ascii_lowercase()),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Paints the waveform page when it applies; `false` leaves the display to the fallback painter.
pub(super) fn paint(p: &Painter, disp: Rect, v: &OledView, title: &str) -> bool {
    let Graphic::Wave { name, ctrl, icon } = &v.graphic else {
        return false;
    };
    if v.title != "OSC WAVEFORM" || matches!(icon, Icon::Sample) {
        return false;
    }
    let (Some(bar), Some(big), Some(small)) = (font(F_BAR), font(F_NAME), font(F_SMALL)) else {
        return false;
    };
    let g = Grid {
        disp,
        cell: disp.width() / 128.0,
    };
    let text = |f: &Font, at: Pos2, a: Align2, s: &str, size: f32, c: Color32| {
        draw_with(p, Some(f), at, a, s, size, g.cell, c);
    };
    p.rect_filled(disp, 2.0, OLED_BG);
    // title bar: lit rows 0..10, unlit title centred at y = 2
    g.block(p, (0, 0, 128, 11), FG_OLED);
    text(&bar, g.at(64, 2), Align2::CENTER_TOP, title, 8.0, OLED_BG);
    text(&big, g.at(1, 14), Align2::LEFT_TOP, name, 11.0, FG_OLED);
    if !ctrl.is_empty() {
        text(&small, g.at(1, 28), Align2::LEFT_TOP, ctrl, 5.0, FG_OLED);
    }
    let (px, py, pw, ph) = PIC;
    for (x, y) in picture_pixels(*icon, pw, ph) {
        g.block(p, (px + x, py + y, 1, 1), FG_OLED);
    }
    for (i, s) in v.soft.iter().enumerate() {
        let Some(s) = s else { continue };
        let color = if s.dim {
            FG_OLED.gamma_multiply(0.4)
        } else {
            FG_OLED
        };
        text(
            &small,
            g.at(COLS[i], 49),
            Align2::CENTER_TOP,
            &mixed_case(&s.value),
            5.0,
            color,
        );
        // The glyph already is the whole tab: lit rounded box with the word cut out (VERIFIED
        // frame rows 57..63), so it is drawn lit, not on top of a block.
        match label_glyph(s.label).zip(font(F_GLYPH)) {
            Some((c, f)) => {
                let w = f.text_width(&c.to_string()) as i32;
                let at = g.at((COLS[i] - w / 2).clamp(0, 128 - w), 57);
                draw_with(
                    p,
                    Some(&f),
                    at,
                    Align2::LEFT_TOP,
                    &c.to_string(),
                    7.0,
                    g.cell,
                    color,
                );
            }
            None => {
                let w = small.text_width(s.label) as i32 + 4;
                let left = (COLS[i] - w / 2).clamp(0, 128 - w);
                g.block(p, (left, 57, w, 7), color);
                text(
                    &small,
                    g.at(left + 2, 59),
                    Align2::LEFT_TOP,
                    s.label,
                    5.0,
                    OLED_BG,
                );
            }
        }
    }
    true
}

/// OLED pixel colour (same as `graphics::FG`).
const FG_OLED: Color32 = super::graphics::FG;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_covers_both_ends_and_a_vertical_is_solid() {
        let l = line_pixels((0, 0), (3, 0));
        assert_eq!(l, [(0, 0), (1, 0), (2, 0), (3, 0)]);
        let v = line_pixels((2, 5), (2, 1));
        assert_eq!(v.len(), 5);
        assert!(v.iter().all(|p| p.0 == 2));
        let d = line_pixels((0, 0), (4, 2));
        assert_eq!((d[0], *d.last().unwrap()), ((0, 0), (4, 2)));
    }

    #[test]
    fn a_saw_is_a_vertical_edge_on_the_left_and_a_ramp_falling_to_the_right() {
        let px = picture_pixels(Icon::Saw, 41, 22);
        // the whole left column is lit (the captured frame's vertical line)
        assert!((0..22).all(|y| px.contains(&(0, y))));
        // one pixel per column from there on, falling from the top to the bottom row
        let col = |x: i32| {
            px.iter()
                .filter(|p| p.0 == x)
                .map(|p| p.1)
                .collect::<Vec<_>>()
        };
        assert!(col(1).first().is_some_and(|&y| y <= 1));
        assert_eq!(col(40).last(), Some(&21));
        assert!(col(20).len() <= 2);
    }

    #[test]
    fn every_icon_stays_inside_the_picture_box() {
        for icon in [
            Icon::Sine,
            Icon::Triangle,
            Icon::Saw,
            Icon::Pulse(0.3),
            Icon::Multi,
            Icon::Noise,
            Icon::Fm,
            Icon::Table,
        ] {
            let px = picture_pixels(icon, 41, 22);
            assert!(!px.is_empty());
            assert!(px
                .iter()
                .all(|&(x, y)| (0..41).contains(&x) && (0..22).contains(&y)));
        }
    }

    #[test]
    fn values_are_mixed_case_unless_they_hold_digits_or_short_words() {
        assert_eq!(mixed_case("ANALOG"), "Analog");
        assert_eq!(mixed_case("SUB OSC"), "Sub Osc");
        assert_eq!(mixed_case("FM"), "FM");
        assert_eq!(mixed_case("3 (7)"), "3 (7)");
        assert_eq!(mixed_case("Bells/Tines"), "Bells/Tines");
    }

    #[test]
    fn labels_map_to_the_firmware_glyphs() {
        assert_eq!(label_glyph("TYPE"), Some('@'));
        assert_eq!(label_glyph("CAT"), Some('B'));
        assert_eq!(label_glyph("WAVE"), Some('C'));
        assert_eq!(label_glyph("FREQ"), None);
    }

    #[test]
    fn without_firmware_fonts_the_page_is_left_to_the_fallback() {
        use crate::panel::support::shapes_painted;
        let v = OledView {
            title: "OSC WAVEFORM",
            layer: 'A',
            graphic: Graphic::Wave {
                name: "Pure Saw".into(),
                ctrl: String::new(),
                icon: Icon::Saw,
            },
            soft: [None, None, None],
        };
        let mut drew = true;
        shapes_painted(|_ui, p, rect, _s, _a| {
            drew = paint(p, rect, &v, "OSC WAVEFORM");
        });
        assert!(!drew || nord_core::oled_font::installed());
    }
}

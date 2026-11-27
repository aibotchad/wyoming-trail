//! Bar graphs for the stats page. One series to a graph, so one colour; the
//! title says what is measured and every bar's figure is a tap away.

use super::draw::{font, Column};
use super::page::{Action, Bar, DayColumn, Hotspot};
use super::App;

/// Series colour: blue stepped for the dark page.
const SERIES: &str = "#338ced";
const SERIES_LIT: &str = "#7cb6f5";
const TRACK: &str = "#262017";
const AXIS: &str = "#6a5f4c";
const INK: &str = "#ede6d6";
const MUTED: &str = "#b0a48c";
const TITLE: &str = "#e8b04a";
/// Bars are no thicker than this, with their far end rounded.
const THICK: f64 = 18.0;
const ROUND: f64 = 4.0;
const PLOT_HEIGHT: f64 = 130.0;

impl App {
    /// A bar from its baseline, rounded at the end the data reaches: the top
    /// of a column (`upward`) or the right of a horizontal bar.
    fn mark(&self, (x, y, w, h): (f64, f64, f64, f64), upward: bool, colour: &str) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let r = ROUND.min(w / 2.0).min(h / 2.0);
        let ctx = &self.ctx;
        ctx.set_fill_style_str(colour);
        ctx.begin_path();
        if upward {
            ctx.move_to(x, y + h);
            ctx.line_to(x, y + r);
            let _ = ctx.arc_to(x, y, x + r, y, r);
            ctx.line_to(x + w - r, y);
            let _ = ctx.arc_to(x + w, y, x + w, y + r, r);
            ctx.line_to(x + w, y + h);
        } else {
            ctx.move_to(x, y);
            ctx.line_to(x + w - r, y);
            let _ = ctx.arc_to(x + w, y, x + w, y + r, r);
            ctx.line_to(x + w, y + h - r);
            let _ = ctx.arc_to(x + w, y + h, x + w - r, y + h, r);
            ctx.line_to(x, y + h);
        }
        ctx.close_path();
        ctx.fill();
    }

    fn label(&self, text: &str, x: f64, baseline: f64, max: f64, colour: &str) {
        if self.on_screen(baseline - 20.0, 26.0) {
            self.ctx.set_fill_style_str(colour);
            let _ = self.ctx.fill_text_with_max_width(text, x, baseline, max);
        }
    }

    pub(super) fn bars(&mut self, title: &str, rows: &[Bar], col: &mut Column) {
        col.y += 6.0;
        self.ctx.set_font(&font(23.0));
        col.y += self.paragraph(title, col.x, col.y, col.w, 23.0, TITLE) + 6.0;
        self.ctx.set_font(&font(19.0));
        // Names to the left, figures to the right, bars between.
        let name_w = (col.w * 0.3).clamp(90.0, 190.0);
        let figure_w = (col.w * 0.24).clamp(80.0, 150.0);
        let track = (col.w - name_w - figure_w - 8.0).max(30.0);
        for row in rows {
            let y = col.y - self.scroll;
            let x = col.x + name_w;
            self.label(&row.label, col.x, y + 15.0, name_w - 8.0, MUTED);
            if self.on_screen(y, THICK) {
                self.ctx.set_fill_style_str(TRACK);
                self.ctx.fill_rect(x, y, track, THICK);
                let share = (row.value / row.max.max(1e-9)).clamp(0.0, 1.0);
                self.mark((x, y, track * share, THICK), false, SERIES);
                self.ctx.set_fill_style_str(AXIS);
                self.ctx.fill_rect(x - 1.0, y - 2.0, 1.0, THICK + 4.0);
            }
            self.label(&row.figure, x + track + 8.0, y + 15.0, figure_w, INK);
            col.y += THICK + 10.0;
        }
        col.y += 10.0;
    }

    pub(super) fn columns(&mut self, title: &str, data: &[DayColumn], col: &mut Column) {
        col.y += 6.0;
        self.ctx.set_font(&font(23.0));
        col.y += self.paragraph(title, col.x, col.y, col.w, 23.0, TITLE) + 6.0;
        // Only as many recent days as stay wide enough to tap.
        let fit = ((col.w / 14.0) as usize).max(1);
        let data = &data[data.len().saturating_sub(fit)..];
        let top = col.y + 22.0 - self.scroll;
        let base = top + PLOT_HEIGHT;
        let most = data.iter().map(|d| d.value).fold(0.0, f64::max);
        let slot = (col.w / data.len().max(1) as f64).min(48.0);
        let thick = (slot - 2.0).clamp(2.0, 24.0);
        let peak = data.iter().position(|d| d.value >= most && most > 0.0);
        // Every day numbered if there's room, else every so many.
        let every = (30.0 / slot).ceil().max(1.0) as u32;
        self.ctx.set_font(&font(17.0));
        for (i, d) in data.iter().enumerate() {
            let x = col.x + slot * i as f64 + (slot - thick) / 2.0;
            let h = if most > 0.0 {
                PLOT_HEIGHT * (d.value / most).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let hovered = self.pointer.is_some_and(|(px, py)| {
                (x..=x + thick).contains(&px) && (top..=base).contains(&py)
            });
            if self.on_screen(top, PLOT_HEIGHT) {
                let colour = if hovered { SERIES_LIT } else { SERIES };
                self.mark((x, base - h, thick, h), true, colour);
            }
            // Figures on the highest and the latest only; the rest on a tap.
            if hovered || Some(i) == peak || i + 1 == data.len() {
                let tw = self.ctx.measure_text(&d.figure).map_or(0.0, |m| m.width());
                let at = (x + thick / 2.0 - tw / 2.0)
                    .min(col.x + col.w - tw)
                    .max(col.x);
                self.label(&d.figure, at, base - h - 6.0, col.w, INK);
            }
            if d.day % every == 0 || every == 1 {
                self.label(
                    &d.day.to_string(),
                    x,
                    base + 18.0,
                    slot * every as f64,
                    MUTED,
                );
            }
            self.layout.hotspots.push(Hotspot {
                x: col.x + slot * i as f64,
                y: top + self.scroll,
                w: slot,
                h: PLOT_HEIGHT + 22.0,
                button: None,
                action: Action::Reveal(format!("Day {}: {}", d.day, d.figure)),
            });
        }
        if self.on_screen(base, 2.0) {
            self.ctx.set_fill_style_str(AXIS);
            self.ctx.fill_rect(col.x, base, col.w, 1.0);
        }
        col.y += PLOT_HEIGHT + 52.0;
        self.ctx.set_font(&font(16.0));
        let hint = if self.touch {
            "By day. Tap a column for its figure."
        } else {
            "By day. Click a column for its figure."
        };
        col.y += self.paragraph(hint, col.x, col.y, col.w, 16.0, MUTED) + 8.0;
    }
}

//! Nine-slice panels cut from `img/ui.png` (Simple RPG GUI, CC0): corners
//! keep their size while edges and middle stretch to fit.

use super::App;

/// A resizable piece of the UI sheet: its rectangle and corner size.
#[derive(Clone, Copy)]
pub(super) struct Skin {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    corner: f64,
    /// Flat colour used until the sheet has loaded.
    fallback: &'static str,
}

const fn skin(x: f64, y: f64, w: f64, h: f64, corner: f64, fallback: &'static str) -> Skin {
    Skin {
        x,
        y,
        w,
        h,
        corner,
        fallback,
    }
}

/// Wooden window frame with a gem in each corner.
pub(super) const WINDOW: Skin = skin(7.0, 6.0, 179.0, 94.0, 18.0, "#4d3020");
/// Rounded dark wood panel.
pub(super) const PANEL: Skin = skin(11.0, 103.0, 173.0, 88.0, 14.0, "#4d3020");
/// Dark slot with notched corners: a button at rest.
pub(super) const SLOT: Skin = skin(367.0, 59.0, 32.0, 32.0, 9.0, "#4d3020");
/// Tan tile: a button under the pointer.
pub(super) const TAN: Skin = skin(330.0, 59.0, 32.0, 32.0, 7.0, "#cb9c83");
/// Parchment: the selected button.
pub(super) const PARCHMENT: Skin = skin(13.0, 200.0, 168.0, 66.0, 8.0, "#efe4b0");
/// Green bar: progress made.
pub(super) const GREEN: Skin = skin(337.0, 9.0, 66.0, 21.0, 6.0, "#b5e61d");

impl App {
    /// Draw `skin` over the screen rectangle (x, y, w, h). With `middle`
    /// false only the border is drawn, to frame something already there.
    pub(super) fn nine(&mut self, skin: Skin, (x, y, w, h): (f64, f64, f64, f64), middle: bool) {
        if w <= 0.0 || h <= 0.0 || !self.on_screen(y, h) {
            return;
        }
        let Some(sheet) = self.image("ui.png") else {
            if middle {
                self.ctx.set_fill_style_str(skin.fallback);
                self.ctx.fill_rect(x, y, w, h);
            }
            return;
        };
        // Corners shrink if the target is smaller than two of them.
        let dc = skin.corner.min(w / 2.0).min(h / 2.0);
        let c = skin.corner;
        // (source offset, source length, target offset, target length)
        let cuts = |from: f64, len: f64, to: f64, size: f64| {
            [
                (from, c, to, dc),
                (from + c, len - 2.0 * c, to + dc, size - 2.0 * dc),
                (from + len - c, c, to + size - dc, dc),
            ]
        };
        self.ctx.set_image_smoothing_enabled(false);
        for (row, (sy, sh, dy, dh)) in cuts(skin.y, skin.h, y, h).into_iter().enumerate() {
            for (col, (sx, sw, dx, dw)) in cuts(skin.x, skin.w, x, w).into_iter().enumerate() {
                if dw <= 0.0 || dh <= 0.0 || (!middle && row == 1 && col == 1) {
                    continue;
                }
                let _ = self
                    .ctx
                    .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                        &sheet, sx, sy, sw, sh, dx, dy, dw, dh,
                    );
            }
        }
        self.ctx.set_image_smoothing_enabled(true);
    }

    /// A window frame just outside a picture, map or field.
    pub(super) fn frame(&mut self, (x, y, w, h): (f64, f64, f64, f64)) {
        const OUT: f64 = 10.0;
        self.nine(
            WINDOW,
            (x - OUT, y - OUT, w + OUT * 2.0, h + OUT * 2.0),
            false,
        );
    }
}

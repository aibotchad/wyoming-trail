//! The drum circle that ends a mission: tap along, and the groove brings out
//! whoever has been summoned. It can't be lost.

use crate::core::defs::{FinaleDef, FinaleScene};

use super::draw::{font, Column};
use super::page::{Action, Hotspot};
use super::App;

/// A beat, and how close a tap must be to count double.
const BEAT_MS: f64 = 620.0;
const ON_BEAT_MS: f64 = 140.0;
/// Strikes' worth of groove it takes.
const FULL: u32 = 18;

/// How the circle is going.
#[derive(Default)]
pub(super) struct Circle {
    groove: u32,
    /// When the drum was last struck, and whether it was on the beat.
    struck: f64,
    sweet: bool,
}

/// How far `now` is from the nearest beat, in milliseconds.
fn off_beat(now: f64) -> f64 {
    let into = now.rem_euclid(BEAT_MS);
    into.min(BEAT_MS - into)
}

impl Circle {
    pub fn strike(&mut self, now: f64) {
        self.sweet = off_beat(now) <= ON_BEAT_MS;
        self.groove = (self.groove + if self.sweet { 2 } else { 1 }).min(FULL);
        self.struck = now;
    }

    pub fn complete(&self) -> bool {
        self.groove >= FULL
    }

    /// 0..1 of the way there.
    fn share(&self) -> f64 {
        f64::from(self.groove) / f64::from(FULL)
    }
}

type Area = (f64, f64, f64, f64);

impl App {
    pub(super) fn finale(&mut self, scene: &FinaleDef, col: &mut Column, view_height: f64) {
        let w = col.w.min(720.0);
        let h = (w * 0.74).min(view_height * 0.6).max(220.0);
        col.y += 6.0;
        let area = (col.x + (col.w - w) / 2.0, col.y - self.scroll, w, h);
        if self.on_screen(area.1, h) {
            self.layout.animated = true;
            self.ctx.save();
            self.ctx.begin_path();
            self.ctx.rect(area.0, area.1, w, h);
            self.ctx.clip();
            match scene.scene {
                FinaleScene::Canyon => self.canyon(area),
                FinaleScene::Condo => self.condo(area),
            }
            self.guests(scene, area);
            self.drummers(scene, area);
            self.groove_meter(area);
            self.ctx.restore();
            self.frame(area);
        }
        self.layout.hotspots.push(Hotspot {
            x: area.0,
            y: col.y,
            w,
            h,
            button: None,
            action: Action::Drum,
        });
        col.y += h + 20.0;
    }

    fn fill(&self, colour: &str, x: f64, y: f64, w: f64, h: f64) {
        self.ctx.set_fill_style_str(colour);
        self.ctx.fill_rect(x, y, w, h);
    }

    fn disc(&self, colour: &str, x: f64, y: f64, r: f64) {
        self.ctx.set_fill_style_str(colour);
        self.ctx.begin_path();
        let _ = self.ctx.arc(x, y, r, 0.0, std::f64::consts::TAU);
        self.ctx.fill();
    }

    fn shape(&self, colour: &str, points: &[(f64, f64)]) {
        self.ctx.set_fill_style_str(colour);
        self.ctx.begin_path();
        for (i, &(x, y)) in points.iter().enumerate() {
            if i == 0 {
                self.ctx.move_to(x, y);
            } else {
                self.ctx.line_to(x, y);
            }
        }
        self.ctx.close_path();
        self.ctx.fill();
    }

    /// A full moon over the canyon walls, and the river below.
    fn canyon(&mut self, (x, y, w, h): Area) {
        for (i, colour) in ["#0b1030", "#121a44", "#1d2656", "#2a3266"]
            .iter()
            .enumerate()
        {
            self.fill(colour, x, y + h * 0.15 * i as f64, w, h * 0.16 + 1.0);
        }
        for i in 0..60_u32 {
            let seed = i.wrapping_mul(2_654_435_761);
            let sx = f64::from(seed >> 8 & 0xFFF) / 4096.0 * w;
            let sy = f64::from(seed >> 20) / 4096.0 * h * 0.45;
            self.fill("#e8ecff", x + sx, y + sy, 2.0, 2.0);
        }
        // The moon climbs as the groove builds.
        let rise = self.circle.share();
        let (mx, my) = (x + w * 0.72, y + h * (0.34 - 0.18 * rise));
        let r = h * 0.11;
        self.disc("rgba(240,244,210,0.16)", mx, my, r * 1.7);
        self.disc("#f4f1d4", mx, my, r);
        self.disc("#dcd8b8", mx - r * 0.3, my - r * 0.15, r * 0.22);
        self.disc("#dcd8b8", mx + r * 0.35, my + r * 0.3, r * 0.15);
        let at = |fx: f64, fy: f64| (x + w * fx, y + h * fy);
        self.shape(
            "#171427",
            &[
                at(0.0, 0.18),
                at(0.1, 0.2),
                at(0.22, 0.34),
                at(0.34, 0.55),
                at(0.38, 0.75),
                at(0.0, 0.8),
            ],
        );
        self.shape(
            "#141122",
            &[
                at(1.0, 0.24),
                at(0.9, 0.27),
                at(0.8, 0.45),
                at(0.66, 0.6),
                at(0.62, 0.75),
                at(1.0, 0.8),
            ],
        );
        self.fill("#26301f", x, y + h * 0.66, w, h * 0.34);
        self.fill("#1f3a5c", x, y + h * 0.62, w, h * 0.06);
        self.fire(x + w * 0.5, y + h * 0.84, h * 0.09);
    }

    /// Log walls, a window on the Tetons, and everything collected.
    fn condo(&mut self, (x, y, w, h): Area) {
        self.fill("#6b4426", x, y, w, h);
        let log = (h * 0.075).max(10.0);
        for i in 0..(h * 0.68 / log) as u32 + 1 {
            self.fill("#53321b", x, y + log * f64::from(i), w, 3.0);
        }
        // The window, with its moon.
        let (wx, wy, ww, wh) = (x + w * 0.33, y + h * 0.07, w * 0.34, h * 0.3);
        self.fill("#2a1a0e", wx - 5.0, wy - 5.0, ww + 10.0, wh + 10.0);
        self.fill("#1a2350", wx, wy, ww, wh);
        self.disc("#f4f1d4", wx + ww * 0.8, wy + wh * 0.28, wh * 0.14);
        let at = |fx: f64, fy: f64| (wx + ww * fx, wy + wh * fy);
        self.shape(
            "#8d9ab8",
            &[
                at(0.0, 1.0),
                at(0.2, 0.5),
                at(0.32, 0.68),
                at(0.5, 0.25),
                at(0.64, 0.62),
                at(0.78, 0.48),
                at(1.0, 1.0),
            ],
        );
        self.fill("#2a1a0e", wx + ww * 0.49, wy, 3.0, wh);
        // The collection, along a shelf.
        let owned: Vec<String> = self
            .game
            .collection()
            .iter()
            .filter_map(|(_, d)| d.sprite.clone())
            .collect();
        let shelf = y + h * 0.5;
        self.fill("#2a1a0e", x + w * 0.04, shelf, w * 0.92, 5.0);
        let size = (w * 0.12).min(h * 0.13);
        for (i, name) in owned.iter().enumerate() {
            let sx = x + w * 0.06 + (size + w * 0.06) * i as f64;
            self.sprite(name, 0, sx, shelf - size, size / 32.0, false);
        }
        if owned.is_empty() {
            self.ctx.set_font(&font(18.0));
            self.ctx.set_fill_style_str("#d9bf98");
            let _ = self
                .ctx
                .fill_text("(the shelf is very bare)", x + w * 0.06, shelf - 8.0);
        }
        self.fill("#3d2a4a", x, y + h * 0.68, w, h * 0.32);
        self.fill("#7a3d52", x + w * 0.16, y + h * 0.74, w * 0.68, h * 0.2);
        self.fire(x + w * 0.5, y + h * 0.82, h * 0.07);
    }

    /// A campfire that flickers, brighter as the groove builds.
    fn fire(&self, x: f64, base: f64, size: f64) {
        let t = js_sys::Date::now() / 90.0;
        let big = size * (1.0 + 0.5 * self.circle.share());
        self.disc("rgba(255,150,50,0.18)", x, base - big * 0.5, big * 2.2);
        self.fill("#4a2c18", x - size, base - 4.0, size * 2.0, 8.0);
        for (i, colour) in ["#d8421c", "#f58a1f", "#ffd447"].iter().enumerate() {
            let k = 1.0 - 0.27 * i as f64;
            let lean = (t + i as f64 * 2.1).sin() * big * 0.18;
            let tall = big * k * (1.5 + 0.25 * (t * 1.7 + i as f64).sin());
            self.shape(
                colour,
                &[
                    (x - big * 0.7 * k, base),
                    (x + lean, base - tall),
                    (x + big * 0.7 * k, base),
                ],
            );
        }
    }

    fn guests(&mut self, scene: &FinaleDef, (x, y, w, h): Area) {
        let share = self.circle.share();
        if share <= 0.0 {
            return;
        }
        let tick = super::scene::anim_tick(js_sys::Date::now()) / 3;
        self.ctx.set_global_alpha(share * share);
        for (i, name) in scene.guests.iter().enumerate() {
            let Some(s) = self.game.content.sprites.get(name).copied() else {
                continue;
            };
            let k = (h * 0.3 / f64::from(s.h)).max(1.0);
            let wide = f64::from(s.w) * k;
            let (gx, gy) = match scene.scene {
                // Up out of the water.
                FinaleScene::Canyon => (x + w * 0.5 - wide / 2.0, y + h * (0.62 - 0.28 * share)),
                // In at the sides, drawn nearer by the beat.
                FinaleScene::Condo => {
                    let edge = if i == 0 { 0.05 } else { 0.95 };
                    let toward = edge + (0.5 - edge) * 0.35 * share;
                    (x + w * toward - wide / 2.0, y + h * 0.5)
                }
            };
            let frame = if s.frames > 3 { tick } else { 1 };
            self.sprite(name, frame, gx, gy, k, false);
        }
        self.ctx.set_global_alpha(1.0);
    }

    /// The circle itself: each bobs on the beat, with a drum in front.
    fn drummers(&mut self, scene: &FinaleDef, (x, y, w, h): Area) {
        let now = js_sys::Date::now();
        let beat = off_beat(now) < ON_BEAT_MS;
        let struck = now - self.circle.struck < 150.0;
        let k = (h * 0.24 / 36.0).max(1.0);
        let n = scene.drummers.len().max(1);
        for (i, name) in scene.drummers.iter().enumerate() {
            let spread = i as f64 / (n.max(2) - 1) as f64;
            // Nearer the edges stand higher: they are round the far side.
            let px = x + w * (0.1 + 0.8 * spread);
            let curve = (spread - 0.5).abs() * 2.0;
            let foot = y + h * (0.93 - 0.11 * curve);
            let bob = if beat == i.is_multiple_of(2) {
                4.0
            } else {
                0.0
            };
            self.sprite(name, 1, px - 16.0 * k, foot - 36.0 * k - bob, k, false);
            let toward = if spread < 0.5 { 1.0 } else { -1.0 };
            let (dx, dy) = (px + toward * 12.0 * k, foot - 8.0 * k);
            self.fill("#6b3f1f", dx - 6.0 * k, dy - 3.0 * k, 12.0 * k, 10.0 * k);
            self.fill("#c9a06a", dx - 7.0 * k, dy - 5.0 * k, 14.0 * k, 3.0 * k);
            if struck {
                self.fill("#fff1a8", dx - 9.0 * k, dy - 11.0 * k, 3.0, 8.0);
                self.fill("#fff1a8", dx + 8.0 * k, dy - 11.0 * k, 3.0, 8.0);
            }
        }
    }

    /// The beat to tap to, and how far the groove has come.
    fn groove_meter(&self, (x, y, w, h): Area) {
        let now = js_sys::Date::now();
        self.fill("rgba(12,10,8,0.7)", x, y, w, 40.0);
        self.fill("#14110d", x + 14.0, y + 28.0, w - 28.0, 7.0);
        self.fill(
            "#ff8ad0",
            x + 14.0,
            y + 28.0,
            (w - 28.0) * self.circle.share(),
            7.0,
        );
        self.ctx.set_font(&font(20.0));
        self.ctx.set_fill_style_str("#ede6d6");
        let said = if self.circle.complete() {
            "THE GROOVE IS COMPLETE"
        } else if now - self.circle.struck < 500.0 && self.circle.sweet {
            "RIGHT ON THE BEAT!"
        } else {
            "GROOVE"
        };
        let _ = self
            .ctx
            .fill_text_with_max_width(said, x + 14.0, y + 22.0, w - 90.0);
        // A ring that closes on the dot as each beat arrives.
        let to_go = 1.0 - now.rem_euclid(BEAT_MS) / BEAT_MS;
        let (bx, by) = (x + w - 34.0, y + h - 34.0);
        self.disc("rgba(12,10,8,0.7)", bx, by, 27.0);
        self.disc(
            if off_beat(now) < ON_BEAT_MS {
                "#ffd447"
            } else {
                "#b0a48c"
            },
            bx,
            by,
            9.0,
        );
        self.ctx.set_stroke_style_str("#ffd447");
        self.ctx.set_line_width(3.0);
        self.ctx.begin_path();
        let _ = self
            .ctx
            .arc(bx, by, 9.0 + 16.0 * to_go, 0.0, std::f64::consts::TAU);
        self.ctx.stroke();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_the_beat_counts_double_and_the_groove_tops_out() {
        let mut c = Circle::default();
        c.strike(BEAT_MS * 10.0 + 40.0);
        assert_eq!(c.groove, 2);
        c.strike(BEAT_MS * 10.0 + BEAT_MS / 2.0);
        assert_eq!(c.groove, 3);
        // Any tapping at all gets there in the end.
        for i in 0..FULL {
            c.strike(f64::from(i) * 333.0);
        }
        assert!(c.complete());
        assert_eq!(c.groove, FULL);
        assert!((c.share() - 1.0).abs() < 1e-9);
    }
}

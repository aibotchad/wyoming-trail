//! Sprite drawing and the travel animation: a side-scrolling view of the
//! drive with progress bars, plus the facts about the leg being shown.

use crate::core::defs::{LayerDef, OverlayDef, SeasonDef, TerrainLook};
use crate::core::game::Game;
use crate::core::state::{EnRoute, GameState};

use super::draw::{font, Column};
use super::ui::{GREEN, SLOT as TRACK};
use super::App;

/// How long a day's drive takes to watch: a moment to get going, and a little
/// for each mile, so long hauls take longer on screen without dragging.
const DRIVE_MS: f64 = 2_800.0;
const MS_PER_MILE: f64 = 72.0;
/// Miles over which one kind of country fades into the next.
const FADE_MILES: f64 = 5.0;
/// Scenery distance covered by one day's drive, in sprite-sheet pixels.
const SCROLL_PER_DAY: f64 = 520.0;
/// Spacing of roadside objects, in sprite-sheet pixels.
const SLOT: f64 = 46.0;

/// Milliseconds per animation frame.
const FRAME_MS: f64 = 110.0;
/// Frame numbers wrap here: a multiple of every strip length up to ten.
const FRAME_WRAP: u64 = 2520 * 1000;

/// The animation frame number at `now_ms` (milliseconds since 1970). That is
/// far too big for a `u32` as it stands, so it wraps.
pub(super) fn anim_tick(now_ms: f64) -> u32 {
    ((now_ms / FRAME_MS) as u64 % FRAME_WRAP) as u32
}

/// Milliseconds spent watching `miles` go by.
fn watching(miles: f64) -> f64 {
    DRIVE_MS + MS_PER_MILE * miles.max(0.0)
}

/// How far the scenery has scrolled, `days` into the trip. It follows the
/// trip alone, so it holds still whenever the vehicle does.
fn scroll_at(days: f64) -> f64 {
    days * SCROLL_PER_DAY
}

/// Miles that were left when the day began.
fn left_before(leg: &EnRoute) -> f64 {
    leg.remaining_miles
}

/// A finished day's drive, being replayed.
pub(super) struct Drive {
    pub from: String,
    pub to: String,
    pub highway: String,
    total: f64,
    /// Miles of the leg covered before and after the day.
    start: f64,
    end: f64,
    /// Calendar day before and after.
    pub day: u32,
    end_day: u32,
    began: f64,
    /// How long it takes to watch, in milliseconds.
    lasts: f64,
    road: usize,
}

impl Drive {
    /// What changed between `before` and the game after `drive_day`.
    pub fn between(before: &GameState, after: &Game) -> Option<Self> {
        let leg = before.en_route.as_ref()?;
        let road = after.content.map.road(leg.road)?;
        // Arriving clears `en_route`, leaving nothing to go.
        let left = after
            .state
            .en_route
            .as_ref()
            .map_or(0.0, |e| e.remaining_miles);
        Some(Drive {
            from: leg.from.clone(),
            to: leg.destination.clone(),
            highway: road.highway.clone(),
            total: leg.total_miles,
            start: leg.total_miles - leg.remaining_miles,
            end: leg.total_miles - left,
            day: before.day,
            end_day: after.state.day,
            began: js_sys::Date::now(),
            lasts: watching(left_before(leg) - left),
            road: leg.road,
        })
    }

    /// 0..1 through the animation.
    fn elapsed(&self) -> f64 {
        ((js_sys::Date::now() - self.began) / self.lasts).clamp(0.0, 1.0)
    }

    pub fn finished(&self) -> bool {
        self.elapsed() >= 1.0
    }
}

/// Where and how the drive view is being drawn.
struct Shot {
    area: (f64, f64, f64, f64),
    scale: f64,
    offset: f64,
    frame: u32,
}

/// The leg on screen right now: on the move, or pulled over.
pub(super) struct Leg {
    pub from: String,
    pub to: String,
    /// The road being driven.
    road: usize,
    total: f64,
    /// Fraction of the leg covered.
    pub done: f64,
    pub moving: bool,
    /// Scenery scroll position.
    scroll: f64,
    /// Animation frame counter for the vehicle's wheels.
    pub frame: u32,
    /// Trip day, which decides the weather.
    day: u32,
}

/// Distance between traffic slots, in sprite-sheet pixels.
const TRAFFIC_SLOT: f64 = 190.0;

impl App {
    pub(super) fn current_leg(&self) -> Option<Leg> {
        let now = js_sys::Date::now();
        let tick = anim_tick(now);
        if let Some(d) = &self.drive {
            let t = d.elapsed();
            // Steady, so one day runs smoothly into the next.
            let miles = d.start + (d.end - d.start) * t;
            let days = f64::from(d.end_day - d.day);
            return Some(Leg {
                from: d.from.clone(),
                to: d.to.clone(),
                road: d.road,
                total: d.total,
                done: miles / d.total.max(1.0),
                moving: true,
                scroll: scroll_at(f64::from(d.day) + days * t),
                frame: tick,
                day: d.day,
            });
        }
        let e = self.game.state.en_route.as_ref()?;
        // Not being replayed means pulled over.
        Some(Leg {
            from: e.from.clone(),
            to: e.destination.clone(),
            road: e.road,
            total: e.total_miles,
            done: 1.0 - e.remaining_miles / e.total_miles.max(1.0),
            moving: false,
            scroll: scroll_at(f64::from(self.game.state.day)),
            frame: tick,
            day: self.game.state.day,
        })
    }

    pub(super) fn vehicle_sprite(&self) -> String {
        let content = &self.game.content;
        let id = &self.game.state.truck.id;
        let found = content.vehicles.iter().find(|v| v.id == *id);
        found
            .or(content.vehicles.first())
            .map_or_else(String::new, |v| v.sprite.clone())
    }

    /// Rows from the top of a sprite's cell to what it stands on.
    pub(super) fn feet(&self, name: &str) -> f64 {
        let found = self.game.content.sprites.get(name);
        found.map_or(0.0, |s| f64::from(s.feet()))
    }

    /// Draw one frame of a sprite with its top-left at screen (`x`, `y`).
    pub(super) fn sprite(
        &mut self,
        name: &str,
        frame: u32,
        x: f64,
        y: f64,
        scale: f64,
        mirrored: bool,
    ) {
        let Some(s) = self.game.content.sprites.get(name).copied() else {
            return;
        };
        let (w, h) = (f64::from(s.w) * scale, f64::from(s.h) * scale);
        if !self.on_screen(y, h) {
            return;
        }
        let Some(sheet) = self.image("sprites.png") else {
            return;
        };
        let sx = f64::from(s.x + s.w * (frame % s.frames.max(1)));
        let ctx = &self.ctx;
        ctx.save();
        ctx.set_image_smoothing_enabled(false); // keep the pixels square
        let left = if mirrored {
            let _ = ctx.translate(x + w, 0.0);
            let _ = ctx.scale(-1.0, 1.0);
            0.0
        } else {
            x
        };
        let _ = ctx.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
            &sheet,
            sx,
            f64::from(s.y),
            f64::from(s.w),
            f64::from(s.h),
            left,
            y,
            w,
            h,
        );
        ctx.restore();
    }

    /// Tile one background layer across the scene, its bottom on `base`.
    fn backdrop(&mut self, image: &str, layer: &LayerDef, area: (f64, f64, f64, f64), offset: f64) {
        let (x, base, w, h) = area;
        let Some(img) = self.image(&format!("bg_{image}.png")) else {
            return;
        };
        let natural_h = f64::from(img.natural_height().max(1));
        let tile_h = h * layer.height;
        let tile_w = (f64::from(img.natural_width()) * tile_h / natural_h)
            .floor()
            .max(1.0);
        let drift = layer.drift * js_sys::Date::now() / 1000.0;
        let mut left = x - (offset * layer.parallax + drift).rem_euclid(tile_w);
        self.ctx.set_image_smoothing_enabled(false);
        while left < x + w {
            // A pixel of overlap hides rounding seams.
            let _ = self.ctx.draw_image_with_html_image_element_and_dw_and_dh(
                &img,
                left.floor(),
                base - tile_h,
                tile_w + 1.0,
                tile_h,
            );
            left += tile_w;
        }
        self.ctx.set_image_smoothing_enabled(true);
    }

    /// Rain, snow, smoke and the rest, over the whole view.
    fn weather_overlay(&self, o: &OverlayDef, (x, y, w, h): (f64, f64, f64, f64)) {
        let t = (js_sys::Date::now() % 100_000.0) / 1000.0;
        self.ctx.set_fill_style_str(&o.tint);
        self.ctx.fill_rect(x, y, w, h);
        self.ctx.set_fill_style_str(&o.color);
        for i in 0..o.count {
            // Scattered start points from a cheap hash.
            let seed = i.wrapping_add(3).wrapping_mul(2_654_435_761);
            let sx = f64::from(seed >> 8 & 0xFFF) / 4096.0 * w;
            let sy = f64::from(seed >> 20) / 4096.0 * h;
            let px = x + (sx + o.slant * t).rem_euclid(w);
            let py = y + (sy + o.fall * t).rem_euclid(h);
            self.ctx.fill_rect(px, py, o.width, o.height);
        }
    }

    pub(super) fn scene(&mut self, col: &mut Column, view_height: f64) {
        let Some(leg) = self.current_leg() else {
            return;
        };
        let w = col.w;
        let h = (w * 0.52).clamp(150.0, (view_height * 0.52).max(160.0));
        col.y += 6.0; // room for the frame
                      // The picture and the progress bar under it both move.
        let top = col.y - self.scroll;
        self.layout.animated |= self.on_screen(top, h + 80.0);
        if self.on_screen(top, h) {
            self.road_view((col.x, top, w, h), &leg);
        }
        col.y += h + 18.0;
        self.progress(col, &leg);
    }

    /// The side-on view of the drive, filling (x, y, w, h).
    fn road_view(&mut self, (x, y, w, h): (f64, f64, f64, f64), leg: &Leg) {
        // Whole-number scale keeps the pixel art crisp.
        let scale = (h / 90.0).floor().max(2.0);
        let road_top = y + h * 0.73;
        let road_h = h * 0.2;
        let offset = leg.scroll * scale;
        let content = self.game.content.clone();
        let Some(road) = content.map.road(leg.road) else {
            return;
        };
        let Some(look) = content.scenery.look(road.terrain_at(leg.done)) else {
            return;
        };
        let month = self.game.date(leg.day).0;
        let Some(season) = content.scenery.season(month) else {
            return;
        };

        self.ctx.save();
        self.ctx.begin_path();
        self.ctx.rect(x, y, w, h);
        self.ctx.clip();

        let view = Shot {
            area: (x, y, w, h),
            scale,
            offset,
            frame: leg.frame,
        };
        self.land(look, season, &view);
        // Where the country changes, the next kind fades in over this one.
        let reach = FADE_MILES / leg.total.max(1.0);
        if let Some((coming, share)) = road.changing_to(leg.done, reach) {
            if let Some(next) = content.scenery.look(coming) {
                self.ctx.set_global_alpha(share);
                self.land(next, season, &view);
                self.ctx.set_global_alpha(1.0);
            }
        }

        // --- road ----------------------------------------------------------------
        self.ctx.set_fill_style_str("#4a4642");
        self.ctx.fill_rect(x, road_top, w, road_h);
        self.ctx.set_fill_style_str("#e8c547");
        let dash = 22.0 * scale;
        let mut dx = x - offset.rem_euclid(dash);
        while dx < x + w {
            self.ctx
                .fill_rect(dx, road_top + road_h * 0.48, dash * 0.5, 3.0);
            dx += dash;
        }

        // --- oncoming traffic in the far lane -------------------------------------
        // It keeps coming even while you are parked.
        let small = (scale - 1.0).max(1.0);
        let gap = TRAFFIC_SLOT * scale;
        let travel = offset * 1.4 + js_sys::Date::now() * 0.05 * scale;
        let first = (travel / gap).floor() as i64 - 1;
        for i in first..=first + (w / gap) as i64 + 2 {
            let traffic = &content.scenery.traffic;
            let at = i.rem_euclid(traffic.len().max(1) as i64) as usize;
            let Some(car) = traffic.get(at) else {
                continue;
            };
            if car.is_empty() || season.parked_traffic.contains(car) {
                continue;
            }
            let sx = x + i as f64 * gap - travel;
            let sy = road_top + road_h * 0.42 - 32.0 * small;
            self.sprite(car, 0, sx, sy, small, true);
        }

        // --- the player's vehicle, with dust while it moves ------------------------
        let car_x = x + w * 0.42 - 24.0 * scale;
        let car_y = road_top + road_h * 0.86 - 31.0 * scale;
        if leg.moving {
            self.sprite(
                season.sprite(&content.scenery.exhaust),
                leg.frame,
                car_x - 38.0 * scale,
                car_y,
                scale,
                false,
            );
        }
        // Parked, it idles: wheels still, body shivering now and then.
        let frame = if leg.moving {
            leg.frame
        } else {
            u32::from(leg.frame.is_multiple_of(8))
        };
        let name = self.vehicle_sprite();
        self.sprite(&name, frame, car_x, car_y, scale, false);

        // --- near brush, rushing past faster than the road ------------------------
        let near = offset * 1.7;
        let gap = 120.0 * scale;
        let first = (near / gap).floor() as i64 - 1;
        for i in first..=first + (w / gap) as i64 + 2 {
            let sx = x + i as f64 * gap - near + (i.rem_euclid(3) as f64) * 17.0 * scale;
            self.sprite(
                season.sprite(&content.scenery.brush),
                0,
                sx,
                y + h - 22.0 * scale,
                scale,
                false,
            );
        }
        if let Some(overlay) = self
            .game
            .weather_on(leg.day)
            .and_then(|w| w.overlay.clone())
        {
            self.weather_overlay(&overlay, (x, y, w, h));
        }
        self.ctx.restore();
        self.frame((x, y, w, h));
    }

    /// One kind of country: sky, distant layers, ground and what stands
    /// beside the road.
    fn land(&mut self, look: &TerrainLook, season: &SeasonDef, view: &Shot) {
        let (x, y, w, h) = view.area;
        let (scale, offset) = (view.scale, view.offset);
        let horizon = y + h * 0.68;
        let road_top = y + h * 0.73;
        let content = self.game.content.clone();

        // --- sky and parallax layers, far to near ------------------------------
        self.ctx.set_fill_style_str(&look.sky);
        self.ctx.fill_rect(x, y, w, h);
        if let Some(img) = look
            .sky_image
            .as_ref()
            .and_then(|n| self.image(&format!("bg_{n}.png")))
        {
            // One stretched copy, so there is only ever one moon.
            let (iw, ih) = (
                f64::from(img.natural_width().max(1)),
                f64::from(img.natural_height().max(1)),
            );
            let k = (w / iw).max((horizon - y) / ih);
            self.ctx.set_image_smoothing_enabled(false);
            let _ = self.ctx.draw_image_with_html_image_element_and_dw_and_dh(
                &img,
                x,
                y,
                iw * k,
                ih * k,
            );
            self.ctx.set_image_smoothing_enabled(true);
        }
        for l in &look.layers {
            // Clouds hang from the top; everything else stands on the horizon.
            let base = if l.from_top {
                y + h * l.height
            } else {
                horizon + 1.0
            };
            self.backdrop(season.layer(&l.image), l, (x, base, w, h), offset);
        }
        let ground = season.ground.as_ref().unwrap_or(&look.ground);
        self.ctx.set_fill_style_str(ground);
        self.ctx.fill_rect(x, horizon, w, y + h - horizon);
        if let Some(wash) = &season.ground_wash {
            self.ctx.set_fill_style_str(wash);
            self.ctx.fill_rect(x, horizon, w, y + h - horizon);
        }

        // --- things beside the road --------------------------------------------
        let spacing = SLOT * scale;
        let first = (offset / spacing).floor() as i64 - 2;
        let tick = view.frame / 3;
        for i in first..=first + (w / spacing) as i64 + 3 {
            let n = look.roadside.len().max(1) as i64;
            let at = (i * 5 + i.div_euclid(7)).rem_euclid(n) as usize;
            let Some(item) = look.roadside.get(at) else {
                continue;
            };
            // The season may swap it, or plant something in a gap.
            let item = season.sprite(item);
            if item.is_empty() {
                continue;
            }
            let Some(s) = content.sprites.get(item) else {
                continue;
            };
            // The big animals have bigger cells and stand farther off.
            let k = if s.h > 40 { scale * 0.6 } else { scale };
            let sx = x + i as f64 * spacing - offset - (f64::from(s.w) * k - 48.0 * scale) / 2.0;
            // Feet on the verge, just short of the pavement.
            let sy = road_top - 2.0 - f64::from(s.feet()) * k;
            let frame = tick.wrapping_add(i.unsigned_abs() as u32);
            self.sprite(item, frame, sx, sy, k, false);
        }
    }

    /// Progress along this road, and the distance still to the mission.
    fn progress(&mut self, col: &mut Column, leg: &Leg) {
        let (x, w) = (col.x, col.w);
        let name = |id: &str| {
            self.game
                .content
                .map
                .node(id)
                .map_or_else(|| id.to_string(), |n| n.name.clone())
        };
        let (from, to) = (name(&leg.from), name(&leg.to));
        let miles = leg.done * leg.total;

        // Room above the bar for the little vehicle riding on it.
        let bar_y = col.y - self.scroll + 28.0;
        self.nine(TRACK, (x, bar_y - 3.0, w, 18.0), true);
        if leg.done > 0.0 {
            self.nine(
                GREEN,
                (x + 2.0, bar_y - 1.0, (w - 4.0) * leg.done, 14.0),
                true,
            );
        }
        let marker = (x + w * leg.done - 24.0).min(x + w - 48.0).max(x);
        let name = self.vehicle_sprite();
        let frame = if leg.moving { leg.frame } else { 0 };
        let feet = self.feet(&name);
        self.sprite(&name, frame, marker, bar_y - 1.0 - feet, 1.0, false);
        col.y += 44.0;

        self.ctx.set_font(&font(21.0));
        let line = format!("{from} to {to}: {miles:.0} of {:.0} miles", leg.total);
        col.y += self.paragraph(&line, x, col.y, w, 21.0, "#ede6d6");
        if let Some(step) = self.game.current_step() {
            if let Some(beyond) = self.game.content.map.miles_to(&leg.to, &step.at) {
                let line = format!(
                    "{}: {:.0} miles by the shortest road this way",
                    step.task,
                    beyond + leg.total - miles
                );
                col.y += self.paragraph(&line, x, col.y, w, 21.0, "#ff8ad0");
            }
        }
        col.y += 10.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn animation_frames_advance_at_todays_clock_values() {
        let now = 1_795_700_000_000.0; // late 2026
        assert_eq!(anim_tick(now + FRAME_MS), anim_tick(now) + 1);
        // Room to add per-sprite offsets without overflowing.
        assert!(anim_tick(now) < u32::MAX / 2);
    }

    #[test]
    fn even_the_longest_road_is_quick_to_watch() {
        let map = crate::core::map_graph::MapGraph::load();
        let longest = map.roads.iter().map(|r| r.distance_miles);
        let longest = longest.fold(0.0, f64::max);
        assert!(watching(longest) < 15_000.0, "{longest} miles");
        assert!(watching(5.0) > 500.0, "short hops are still seen");
        assert!(scroll_at(2.0) > scroll_at(1.0));
    }
}

//! The poaching / foraging field: its controls, its clock and its drawing.
//! The rules are in `crate::core::gather`.

use std::collections::BTreeSet;

use crate::core::config::{GATHER_CARRY, GATHER_SECONDS, JUMP_SECONDS, MEAT_CARRY_LB};
use crate::core::defs::GatherMode;
use crate::core::gather::Session;

use super::draw::{font, Column};
use super::page::View;
use super::scene::anim_tick;
use super::App;

/// A press shorter than this is a tap; longer, and you walk toward it.
const TAP_MS: f64 = 220.0;
const TAP_SLOP: f64 = 14.0;
/// Ignore Space this long after a round ends, so a last shot doesn't also
/// dismiss the results.
const SETTLE_MS: f64 = 900.0;

/// A screen rectangle: (x, y, width, height).
type Area = (f64, f64, f64, f64);

/// Read-out text size for a field `w` wide.
fn text_size(w: f64) -> f64 {
    if w < 500.0 {
        19.0
    } else {
        26.0
    }
}

/// The two lines of score for a round.
fn tally(round: &Session) -> (String, String) {
    let bag = || format!("BAG {} / {} servings", round.food, GATHER_CARRY);
    match round.mode {
        GatherMode::Poach => (
            format!("MEAT SHOT {} lb   AMMO {}", round.meat_shot, round.ammo),
            format!("CARRYING {} / {} lb", round.carried(), MEAT_CARRY_LB),
        ),
        GatherMode::Forage => ("FORAGING".to_string(), bag()),
        GatherMode::Scavenge => ("DUMPSTER DIVING".to_string(), bag()),
    }
}

/// The moments of a dive seen from the alley.
#[derive(Clone, Copy)]
enum Outside {
    Waiting,
    /// 0..1 of the way in.
    Leaping(f64),
    Done,
}

/// Something to draw in the field, centred at `at` in field units.
struct Actor {
    name: String,
    frame: u32,
    at: (f64, f64),
    size: f64,
    mirrored: bool,
    /// Set for an armed poacher: which way the rifle points.
    armed: Option<&'static str>,
}

#[derive(Clone, Copy)]
struct Touch {
    /// Which finger.
    id: i32,
    began: f64,
    from: (f64, f64),
    at: (f64, f64),
}

#[derive(Default)]
pub(super) struct Controls {
    /// Steering keys held down.
    held: BTreeSet<String>,
    /// Fingers down on the field, oldest first.
    touches: Vec<Touch>,
    /// Time of the previous step.
    last: f64,
    ended: f64,
}

impl Controls {
    pub fn release(&mut self, key: &str) {
        self.held.remove(&key.to_lowercase());
    }

    /// Where the finger doing the steering is: the oldest one held down. Others
    /// can tap to shoot while it stays put.
    fn steering(&self, now: f64) -> Option<(f64, f64)> {
        let held = self.touches.iter().find(|t| now - t.began >= TAP_MS);
        held.map(|t| t.at)
    }

    fn press(&mut self, id: i32, at: (f64, f64), now: f64) {
        self.touches.retain(|t| t.id != id);
        self.touches.push(Touch {
            id,
            began: now,
            from: at,
            at,
        });
    }

    fn drag(&mut self, id: i32, to: (f64, f64)) -> bool {
        let found = self.touches.iter_mut().find(|t| t.id == id);
        found.map(|t| t.at = to).is_some()
    }

    /// Lift a finger. `None` if it wasn't on the field; otherwise whether it
    /// was a tap.
    fn lift(&mut self, id: i32, at: (f64, f64), now: f64) -> Option<bool> {
        let i = self.touches.iter().position(|t| t.id == id)?;
        let t = self.touches.remove(i);
        let still = (at.0 - t.from.0).hypot(at.1 - t.from.1) < TAP_SLOP;
        Some(now - t.began < TAP_MS && still)
    }

    /// Direction from the held keys.
    fn keys(&self) -> (f64, f64) {
        let any =
            |names: [&str; 2]| f64::from(u8::from(names.iter().any(|n| self.held.contains(*n))));
        (
            any(["arrowright", "d"]) - any(["arrowleft", "a"]),
            any(["arrowdown", "s"]) - any(["arrowup", "w"]),
        )
    }
}

impl App {
    pub(super) fn playing(&self) -> bool {
        self.view == View::Gather
            && self.round_started
            && self.round.as_ref().is_some_and(|r| !r.over())
    }

    /// A round set up but not yet started: the player gets to read first.
    fn waiting(&self) -> bool {
        self.view == View::Gather && !self.round_started && self.round.is_some()
    }

    /// Start the clock.
    pub(super) fn begin_round(&mut self) {
        if self.waiting() {
            self.round_started = true;
            self.controls = Controls {
                last: js_sys::Date::now(),
                ..Controls::default()
            };
        }
    }

    /// Screen position to field units.
    fn to_field(&self, (x, y): (f64, f64)) -> Option<(f64, f64)> {
        let (fx, fy, w, h) = self.layout.field?;
        let round = self.round.as_ref()?;
        Some(((x - fx) * round.width / w, (y - fy) * round.height / h))
    }

    /// A pointer went down. True if the field took it.
    pub(super) fn field_press(&mut self, id: i32, at: (f64, f64)) -> bool {
        let inside = self
            .layout
            .field
            .is_some_and(|(x, y, w, h)| (x..=x + w).contains(&at.0) && (y..=y + h).contains(&at.1));
        if !inside || at.1 >= self.layout.view_height {
            return false;
        }
        // The first tap only starts the round: it neither walks nor shoots.
        if self.waiting() {
            self.begin_round();
            self.refresh();
            self.render();
            return true;
        }
        if !self.playing() {
            return false;
        }
        self.controls.press(id, at, js_sys::Date::now());
        true
    }

    pub(super) fn field_drag(&mut self, id: i32, to: (f64, f64)) -> bool {
        self.controls.drag(id, to)
    }

    /// The browser took the finger away (a system gesture, say).
    pub(super) fn field_cancel(&mut self, id: i32) {
        self.controls.touches.retain(|t| t.id != id);
    }

    pub(super) fn field_release(&mut self, id: i32, at: (f64, f64)) -> bool {
        let Some(tap) = self.controls.lift(id, at, js_sys::Date::now()) else {
            return false;
        };
        if let (true, Some((x, y)), Some(round)) = (tap, self.to_field(at), self.round.as_mut()) {
            match round.mode {
                GatherMode::Poach => round.fire(Some((x, y))),
                GatherMode::Forage | GatherMode::Scavenge => round.go_to(x, y),
            }
        }
        true
    }

    /// A key went down. True if the field used it.
    pub(super) fn field_key(&mut self, key: &str, repeat: bool) -> bool {
        if self.view != View::Gather {
            return false;
        }
        if self.waiting() {
            let steer = matches!(
                key.to_lowercase().as_str(),
                "arrowup" | "arrowdown" | "arrowleft" | "arrowright" | "w" | "a" | "s" | "d"
            );
            if !(steer || key == " " || key == "Enter") {
                return false;
            }
            if !repeat {
                self.begin_round();
                if steer {
                    self.controls.held.insert(key.to_lowercase());
                }
                self.refresh();
                self.render();
            }
            return true;
        }
        if !self.playing() {
            return key == " " && js_sys::Date::now() - self.controls.ended < SETTLE_MS;
        }
        let name = key.to_lowercase();
        match name.as_str() {
            "arrowup" | "arrowdown" | "arrowleft" | "arrowright" | "w" | "a" | "s" | "d" => {
                self.controls.held.insert(name);
                true
            }
            " " => {
                if let (false, Some(round)) = (repeat, self.round.as_mut()) {
                    round.fire(None);
                    round.pick();
                }
                true
            }
            // Don't let a stray Enter end the round.
            "enter" => true,
            _ => false,
        }
    }

    /// Advance the round to now and redraw.
    pub(super) fn step_field(&mut self) {
        if !self.playing() {
            return;
        }
        let now = js_sys::Date::now();
        let dt = (now - self.controls.last) / 1000.0;
        self.controls.last = now;
        let (mut dx, mut dy) = self.controls.keys();
        // A held finger steers toward itself.
        if let Some(at) = self.controls.steering(now) {
            if let (Some((x, y)), Some(r)) = (self.to_field(at), self.round.as_ref()) {
                if (x - r.x).hypot(y - r.y) > 5.0 {
                    (dx, dy) = (x - r.x, y - r.y);
                }
            }
        }
        if let Some(round) = self.round.as_mut() {
            round.update(dt, dx, dy);
            if round.over() {
                self.controls = Controls {
                    ended: now,
                    ..Controls::default()
                };
                self.refresh();
            }
        }
        self.render();
    }

    pub(super) fn field(&mut self, col: &mut Column, view_height: f64) {
        // Borrowed out of the app while it is drawn; put back at the end.
        let Some(round) = self.round.take() else {
            return;
        };
        let aspect = round.width / round.height;
        let w = col.w.min((view_height * 0.66).max(200.0) * aspect);
        let h = w / aspect;
        col.y += 6.0; // room for the frame
        let area = (col.x + (col.w - w) / 2.0, col.y - self.scroll, w, h);
        self.layout.field = Some(area);

        self.ctx.save();
        self.ctx.begin_path();
        self.ctx.rect(area.0, area.1, w, h);
        self.ctx.clip();
        // A diver is seen outside the dumpster before and after.
        let outside = self.outside_view(&round);
        match outside {
            Some(view) => self.field_outside(area, view),
            None => {
                self.field_country(&round, area);
                self.field_cast(&round, area);
            }
        }
        let top = self.field_scores(&round, area);
        let bottom = self.field_message(&round, area);
        self.field_snapshot(&round, area, top);
        if !round.over() {
            self.field_flash(&round, area, top, bottom);
        }
        // What is left between the read-outs, for cards laid over the field.
        // Outside a dumpster they keep to the top, so it stays in view.
        let mut middle = (area.0, area.1 + top, w, h - top - bottom);
        if outside.is_some() {
            middle.3 = middle.3.min(h * 0.4);
        }
        if !self.round_started {
            self.start_card(round.mode, middle);
        }
        if round.over() {
            self.done_card(area, middle);
        }
        self.ctx.restore();
        self.frame(area);
        col.y += h + 18.0;
        self.round = Some(round);
    }

    /// Ground, photo and the season's wash.
    fn field_country(&mut self, round: &Session, (x, y, w, h): Area) {
        let content = self.game.content.clone();
        if let Some(look) = content.scenery.look(self.game.local_terrain()) {
            self.ctx.set_fill_style_str(&look.field_ground);
        }
        self.ctx.fill_rect(x, y, w, h);
        let photo = round
            .backdrop
            .as_ref()
            .and_then(|f| Some((self.image(&format!("{}.jpg", f.id))?, f.horizon, f.sky)));
        if let Some((img, horizon, sky)) = photo {
            // Crop the photo so its skyline lands where the walking stops.
            let (iw, ih) = (
                f64::from(img.natural_width()),
                f64::from(img.natural_height()),
            );
            let sh = ih
                .min((1.0 - horizon) * ih / (1.0 - sky))
                .min(horizon * ih / sky)
                .min(iw * h / w);
            let sw = sh * w / h;
            let _ = self
                .ctx
                .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                    &img,
                    (iw - sw) / 2.0,
                    horizon * ih - sky * sh,
                    sw,
                    sh,
                    x,
                    y,
                    w,
                    h,
                );
        }
        // Snow over the country, from the skyline down.
        let season = self.game.season(self.game.state.day);
        if let Some(wash) = season.and_then(|s| s.field_wash.as_ref()) {
            if round.mode != GatherMode::Scavenge {
                self.ctx.set_fill_style_str(wash);
                let sky = round.top / round.height * h;
                self.ctx.fill_rect(x, y + sky, w, h - sky);
            }
        }
    }

    /// The animals or finds, as they stand now.
    fn things(round: &Session, tick: u32) -> Vec<Actor> {
        let covers = round.backdrop.as_ref().map_or(&[][..], |f| &f.covers);
        let each = round.things.iter().enumerate();
        each.filter_map(|(i, t)| {
            let kind = round.kinds.get(t.kind)?;
            // A buried find shows as a pile; which pile goes by where it lies.
            let pile = (t.x * 7.0 + t.y * 13.0) as usize % covers.len().max(1);
            let name = match (&kind.flee, t.fleeing) {
                _ if t.hidden => covers.get(pile).unwrap_or(&kind.sprite).clone(),
                (Some(f), true) => f.clone(),
                _ => kind.sprite.clone(),
            };
            let frame = if t.vx != 0.0 {
                tick.wrapping_add(i as u32)
            } else {
                0
            };
            Some(Actor {
                name,
                frame,
                at: (t.x, t.y),
                size: kind.size,
                mirrored: t.vx < 0.0,
                armed: None,
            })
        })
        .collect()
    }

    /// The player: one strip per direction, stepping left, middle, right.
    fn player(&self, round: &Session, tick: u32) -> Actor {
        let (fx, fy) = round.face;
        let way = match (fx.abs() >= fy.abs(), fx >= 0.0, fy >= 0.0) {
            (true, true, _) => "right",
            (true, false, _) => "left",
            (false, _, true) => "down",
            (false, _, false) => "up",
        };
        let female = self.game.state.female;
        let job = self.game.job();
        let name = job.map_or_else(String::new, |j| j.hero_sprite(female, way));
        let look = job
            .and_then(|j| self.game.content.look(j.look(female)))
            .cloned()
            .unwrap_or_default();
        let frame = match look.aim {
            Some(aim) if round.aiming() => aim,
            _ if round.moving && !look.walk.is_empty() => {
                look.walk[tick as usize % look.walk.len()]
            }
            _ => look.stand,
        };
        Actor {
            name,
            frame,
            at: (round.x, round.y),
            size: 1.0,
            mirrored: false,
            // A rifle is drawn in for art that doesn't carry one.
            armed: (round.mode == GatherMode::Poach && !look.armed).then_some(way),
        }
    }

    /// Is this moment of a round seen from outside the dumpster?
    fn outside_view(&self, round: &Session) -> Option<Outside> {
        if round.mode != GatherMode::Scavenge {
            return None;
        }
        if !self.round_started {
            return Some(Outside::Waiting);
        }
        if round.over() {
            return Some(Outside::Done);
        }
        let t = (GATHER_SECONDS - round.time_left) / JUMP_SECONDS;
        (t < 1.0).then_some(Outside::Leaping(t.max(0.0)))
    }

    /// Behind the store in the snow: the dumpster, and the diver beside it or
    /// on the way in.
    fn field_outside(&mut self, (x, y, w, h): Area, view: Outside) {
        let ground = y + h * 0.66;
        self.ctx.set_fill_style_str("#8a4e3e");
        self.ctx.fill_rect(x, y, w, ground - y);
        self.ctx.set_fill_style_str("#6c3c32");
        let course = h * 0.045;
        let mut row = y;
        let mut odd = false;
        while row < ground {
            self.ctx.fill_rect(x, row, w, 2.0);
            let mut brick = x + if odd { course } else { 0.0 };
            while brick < x + w {
                self.ctx.fill_rect(brick, row, 2.0, course);
                brick += course * 2.0;
            }
            row += course;
            odd = !odd;
        }
        self.ctx.set_fill_style_str("#e0e8f2");
        self.ctx.fill_rect(x, ground, w, y + h - ground);

        // Full going in; shut and picked over afterwards.
        let bin = match view {
            Outside::Done => "dumpster_empty",
            _ => "dumpster_full",
        };
        let Some(art) = self.game.content.sprites.get(bin).copied() else {
            return;
        };
        let k = (w * 0.58 / f64::from(art.w)).min(h * 0.46 / f64::from(art.h));
        let (bw, bh) = (f64::from(art.w) * k, f64::from(art.h) * k);
        let base = y + h * 0.84;
        let (bx, by) = (x + w * 0.62 - bw / 2.0, base - bh);

        let female = self.game.state.female;
        let Some(job) = self.game.job() else {
            return;
        };
        let (hero, side) = (job.hero_sprite(female, "right"), k * 0.8);
        let start = (x + w * 0.1, base - 34.0 * side);
        let (at, behind) = match view {
            Outside::Leaping(t) => {
                // Up, over the rim and out of sight inside.
                let end = (bx + bw / 2.0 - 16.0 * side, by + bh * 0.3);
                let arc = (t * std::f64::consts::PI).sin() * h * 0.34;
                let along = (
                    start.0 + (end.0 - start.0) * t,
                    start.1 + (end.1 - start.1) * t - arc,
                );
                (along, t > 0.62)
            }
            _ => (start, false),
        };
        if behind {
            self.sprite(&hero, 1, at.0, at.1, side, false);
        }
        self.sprite(bin, 0, bx, by, k, false);
        if !behind {
            self.sprite(&hero, 1, at.0, at.1, side, false);
        }
    }

    /// Everything standing in the field, nearer things drawn later, and the
    /// shots in the air.
    fn field_cast(&mut self, round: &Session, (x, y, w, _): Area) {
        let k = w / round.width;
        let tick = anim_tick(js_sys::Date::now());
        let mut cast = Self::things(round, tick);
        cast.push(self.player(round, tick));
        let sprites = &self.game.content.sprites;
        let ground = |a: &Actor| {
            sprites.get(&a.name).map_or(a.at.1, |s| {
                a.at.1 + (f64::from(s.feet()) - f64::from(s.h) / 2.0) * a.size
            })
        };
        cast.sort_by(|a, b| ground(a).total_cmp(&ground(b)));
        for actor in cast {
            let Some(s) = self.game.content.sprites.get(&actor.name).copied() else {
                continue;
            };
            let (sw, sh) = (f64::from(s.w) * actor.size, f64::from(s.h) * actor.size);
            let (cx, cy) = actor.at;
            let corner = (x + (cx - sw / 2.0) * k, y + (cy - sh / 2.0) * k);
            // Facing away, the rifle is on the far side of the body.
            if actor.armed == Some("up") {
                self.rifle("up", (x + cx * k, y + cy * k), k);
            }
            self.sprite(
                &actor.name,
                actor.frame,
                corner.0,
                corner.1,
                actor.size * k,
                actor.mirrored,
            );
            if let Some(way) = actor.armed.filter(|w| *w != "up") {
                self.rifle(way, (x + cx * k, y + cy * k), k);
            }
        }
        self.ctx.set_fill_style_str("#fff1a8");
        for s in &round.shots {
            self.ctx
                .fill_rect(x + s.x * k - 2.0, y + s.y * k - 2.0, 5.0, 5.0);
        }
    }

    /// The tally and the clock across the top; returns the strip's height.
    fn field_scores(&self, round: &Session, (x, y, w, _): Area) -> f64 {
        // Everything is inset to stay clear of the frame.
        let size = text_size(w);
        let top_h = size * 2.0 + 30.0;
        let left = round.time_left.max(0.0);
        let urgent = left <= 10.0;
        self.ctx.set_fill_style_str("rgba(20,17,13,0.78)");
        self.ctx.fill_rect(x, y, w, top_h);

        // The clock: big, top right, red for the last ten seconds.
        let big = size * 1.7;
        let secs = left.ceil() as u32;
        let clock = format!("{}:{:02}", secs / 60, secs % 60);
        self.ctx.set_font(&font(big));
        let clock_w = self
            .ctx
            .measure_text(&clock)
            .map(|m| m.width())
            .unwrap_or(0.0);
        self.ctx
            .set_fill_style_str(if urgent { "#ff5a3c" } else { "#ffffff" });
        let _ = self.ctx.fill_text(&clock, x + w - 16.0 - clock_w, y + big);

        let (first, second) = tally(round);
        let room = w - clock_w - 44.0;
        self.ctx.set_font(&font(size));
        self.ctx.set_fill_style_str("#e8b04a");
        let _ = self
            .ctx
            .fill_text_with_max_width(&first, x + 16.0, y + size + 6.0, room);
        let full = round.mode == GatherMode::Poach && round.meat_shot >= MEAT_CARRY_LB;
        self.ctx
            .set_fill_style_str(if full { "#ff8ad0" } else { "#ede6d6" });
        let _ = self
            .ctx
            .fill_text_with_max_width(&second, x + 16.0, y + size * 2.0 + 10.0, room);

        // Time running out, as a shrinking bar.
        let bar_y = y + top_h - 10.0;
        self.ctx.set_fill_style_str("#14110d");
        self.ctx.fill_rect(x + 14.0, bar_y, w - 28.0, 7.0);
        self.ctx
            .set_fill_style_str(if urgent { "#ff5a3c" } else { "#b5e61d" });
        let share = (left / GATHER_SECONDS).clamp(0.0, 1.0);
        self.ctx.fill_rect(x + 14.0, bar_y, (w - 28.0) * share, 7.0);
        top_h
    }

    /// News of the last thing that happened, along the bottom; returns the
    /// strip's height.
    fn field_message(&self, round: &Session, (x, y, w, h): Area) -> f64 {
        let small = text_size(w) - 3.0;
        self.ctx.set_font(&font(small));
        let lines = self.wrap(&round.message, w - 32.0);
        let box_h = lines.len() as f64 * small * 1.1 + 18.0;
        self.ctx.set_fill_style_str("rgba(20,17,13,0.75)");
        self.ctx.fill_rect(x, y + h - box_h, w, box_h);
        self.ctx.set_fill_style_str("#ede6d6");
        for (i, line) in lines.iter().enumerate() {
            let by = y + h - box_h + small + i as f64 * small * 1.1;
            let _ = self
                .ctx
                .fill_text_with_max_width(line, x + 16.0, by - 3.0, w - 32.0);
        }
        box_h
    }

    /// What you just picked, as it really looks.
    fn field_snapshot(&mut self, round: &Session, (x, y, w, h): Area, top: f64) {
        let kind = round.picked.and_then(|(k, _)| round.kinds.get(k));
        let Some((kind, photo)) = kind.and_then(|k| Some((k, k.photo.as_ref()?))) else {
            return;
        };
        let side = (w * 0.3).min(h * 0.45);
        let at = (x + w - side - 14.0, y + top + 8.0);
        self.cover(photo, (at.0, at.1, side, side));
        self.ctx
            .set_stroke_style_str(if kind.poison { "#ff5a3c" } else { "#b5e61d" });
        self.ctx.set_line_width(4.0);
        self.ctx.stroke_rect(at.0, at.1, side, side);
    }

    /// The shout about a dumpster find: what it is and whether to eat it. It
    /// keeps to the half of the field the diver isn't in.
    fn field_flash(&mut self, round: &Session, (x, y, w, h): Area, top: f64, bottom: f64) {
        let Some(flash) = round.flash else {
            return;
        };
        let Some(kind) = round.kinds.get(flash.kind) else {
            return;
        };
        let good = !kind.poison && kind.food > 0;
        let (title, name, verdict) = match (good, flash.taken) {
            (true, false) => (
                "YOU CAN EAT THIS!",
                kind.name.clone(),
                "Tap it to take it".to_string(),
            ),
            (true, true) => (
                "GREAT JOB!",
                format!("You got {}", kind.name),
                format!("You can eat this: +{} servings", kind.food),
            ),
            (false, false) => (
                "DON'T EAT THAT",
                kind.name.clone(),
                "Leave it where it is".to_string(),
            ),
            (false, true) => (
                "OH NO",
                format!("You grabbed the {}", kind.name.to_lowercase()),
                "That is not dinner".to_string(),
            ),
        };
        let tall = (h * 0.3).clamp(96.0, 150.0);
        let high = round.y > round.height * 0.55;
        let at = if high {
            y + top + 8.0
        } else {
            y + h - bottom - tall - 8.0
        };
        let (cx, cw) = (x + 14.0, w - 28.0);
        // It blinks between two borders.
        let blink = ((flash.left * 8.0) as u32).is_multiple_of(2);
        let border = match (good, blink) {
            (true, true) => "#b5e61d",
            (true, false) => "#fff1a8",
            (false, true) => "#ff5a3c",
            (false, false) => "#ffd447",
        };
        self.ctx.set_fill_style_str(border);
        self.ctx
            .fill_rect(cx - 5.0, at - 5.0, cw + 10.0, tall + 10.0);
        self.ctx
            .set_fill_style_str(if good { "#14321a" } else { "#3a1410" });
        self.ctx.fill_rect(cx, at, cw, tall);

        // The find itself, large, on the left.
        let side = (tall - 20.0).min(cw * 0.3);
        if let Some(art) = self.game.content.sprites.get(&kind.sprite).copied() {
            let [ax, ay, aw, ah] = art.art.map(f64::from);
            let k = (side / aw).min(side / ah);
            let left = cx + 10.0 + (side - aw * k) / 2.0 - ax * k;
            let up = at + (tall - ah * k) / 2.0 - ay * k;
            self.sprite(&kind.sprite, 0, left, up, k, false);
        }
        let (tx, tw) = (cx + side + 22.0, cw - side - 30.0);
        let big = (tall * 0.27).min(36.0);
        self.ctx.set_font(&font(big));
        self.ctx.set_fill_style_str(border);
        // Struck twice for a bolder face.
        for shift in [0.0, 1.5] {
            let _ = self
                .ctx
                .fill_text_with_max_width(title, tx + shift, at + tall * 0.38, tw);
        }
        self.ctx.set_font(&font(big * 0.72));
        self.ctx.set_fill_style_str("#ffffff");
        let _ = self
            .ctx
            .fill_text_with_max_width(&name, tx, at + tall * 0.63, tw);
        self.ctx.set_fill_style_str("#ede6d6");
        let _ = self
            .ctx
            .fill_text_with_max_width(&verdict, tx, at + tall * 0.85, tw);
    }

    fn done_card(&self, (x, y, w, h): Area, (mx, my, mw, mh): Area) {
        self.ctx.set_fill_style_str("rgba(20,17,13,0.6)");
        self.ctx.fill_rect(mx, my, mw, mh);
        self.ctx.set_font(&font(text_size(w) * 2.0));
        self.ctx.set_fill_style_str("#ff8ad0");
        self.ctx.set_text_align("center");
        let _ = self.ctx.fill_text("DONE!", x + w / 2.0, y + h / 2.0);
        self.ctx.set_text_align("left");
    }

    /// How to play, over the field, until the round is started.
    fn start_card(&self, mode: GatherMode, (x, y, w, h): (f64, f64, f64, f64)) {
        let title = if self.touch {
            "TAP HERE TO START"
        } else {
            "CLICK OR PRESS SPACE TO START"
        };
        let title = match (mode, self.touch) {
            (GatherMode::Scavenge, true) => "TAP HERE TO JUMP IN",
            (GatherMode::Scavenge, false) => "CLICK OR PRESS SPACE TO JUMP IN",
            _ => title,
        };
        let how: [&str; 3] = match (mode, self.touch) {
            (GatherMode::Poach, true) => [
                "Hold a finger down to walk to it.",
                "Tap an animal to shoot at it.",
                "Aim a little ahead of runners.",
            ],
            (GatherMode::Poach, false) => [
                "Walk: arrows, WASD or hold the mouse.",
                "Shoot: click a target, or Space.",
                "Aim a little ahead of runners.",
            ],
            (GatherMode::Forage, true) => [
                "Tap a plant to walk over and pick it.",
                "Check the field guide below first.",
                "Leave the death camas alone.",
            ],
            (GatherMode::Scavenge, true) => [
                "Tap a pile to wade over and dig.",
                "Tap what you find to take it.",
                "Leave the mystery burrito where it is.",
            ],
            (GatherMode::Scavenge, false) => [
                "Click a pile to dig; click again to take.",
                "Or walk with arrows and press Space.",
                "Leave the mystery burrito where it is.",
            ],
            (GatherMode::Forage, false) => [
                "Click a plant to walk over and pick it.",
                "Or walk with arrows and press Space.",
                "Leave the death camas alone.",
            ],
        };
        self.ctx.set_fill_style_str("rgba(20,17,13,0.78)");
        self.ctx.fill_rect(x, y, w, h);
        self.ctx.set_text_align("center");
        let mid = x + w / 2.0;
        let widest: f64 = if w < 500.0 { 28.0 } else { 36.0 };
        // The title and three lines take about 4.2 title heights.
        let big = widest.min(h / 4.2);
        let small = big * 0.68;
        let top = y + (h - big - small * 4.0).max(0.0) / 2.0 + big;
        self.ctx.set_font(&font(big));
        self.ctx.set_fill_style_str("#ff8ad0");
        let _ = self.ctx.fill_text_with_max_width(title, mid, top, w - 24.0);
        self.ctx.set_font(&font(small));
        self.ctx.set_fill_style_str("#ede6d6");
        for (i, line) in how.iter().enumerate() {
            let at = top + small * (1.5 + 1.2 * i as f64);
            let _ = self.ctx.fill_text_with_max_width(line, mid, at, w - 24.0);
        }
        self.ctx.set_text_align("left");
    }

    /// A rifle held by someone centred at screen point (`x`, `y`).
    fn rifle(&self, way: &str, (x, y): (f64, f64), k: f64) {
        // (stock x, y, w, h) then (barrel x, y, w, h), in field units.
        let (stock, barrel) = match way {
            "right" => ([0.0, 5.0, 7.0, 3.0], [7.0, 5.0, 14.0, 2.0]),
            "left" => ([-7.0, 5.0, 7.0, 3.0], [-21.0, 5.0, 14.0, 2.0]),
            "down" => ([4.0, 2.0, 3.0, 6.0], [4.5, 8.0, 2.0, 9.0]),
            _ => ([5.0, -6.0, 3.0, 7.0], [5.5, -19.0, 2.0, 13.0]),
        };
        for (colour, [rx, ry, rw, rh]) in [("#6b4423", stock), ("#26262b", barrel)] {
            self.ctx.set_fill_style_str(colour);
            self.ctx.fill_rect(x + rx * k, y + ry * k, rw * k, rh * k);
        }
    }

    /// Fill a rectangle with the middle of `img/<name>.jpg`.
    fn cover(&mut self, name: &str, (x, y, w, h): (f64, f64, f64, f64)) {
        if !self.on_screen(y, h) {
            return;
        }
        let Some(img) = self.image(&format!("{name}.jpg")) else {
            self.ctx.set_fill_style_str("#262017");
            self.ctx.fill_rect(x, y, w, h);
            return;
        };
        let (iw, ih) = (
            f64::from(img.natural_width()),
            f64::from(img.natural_height()),
        );
        let sw = iw.min(ih * w / h);
        let sh = sw * h / w;
        let _ = self
            .ctx
            .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                &img,
                (iw - sw) / 2.0,
                (ih - sh) / 2.0,
                sw,
                sh,
                x,
                y,
                w,
                h,
            );
    }

    /// A field-guide entry with a photo of the species.
    pub(super) fn species(&mut self, photo: &str, sprite: &str, note: &str, col: &mut Column) {
        let side = if col.w < 420.0 { 110.0 } else { 150.0 };
        col.y += 10.0; // room for the frame
        let y = col.y - self.scroll;
        self.cover(photo, (col.x, y, side, side));
        self.frame((col.x, y, side, side));
        // The little picture to look for in the field.
        if let Some(s) = self.game.content.sprites.get(sprite).copied() {
            let [ax, ay, aw, ah] = s.art.map(f64::from);
            let k = 40.0 / ah;
            let at = (col.x + side - aw * k - 4.0, y + side - 44.0);
            self.ctx.set_fill_style_str("rgba(20,17,13,0.7)");
            self.ctx.fill_rect(at.0 - 2.0, at.1, aw * k + 4.0, 44.0);
            self.sprite(sprite, 0, at.0 - ax * k, at.1 + 2.0 - ay * k, k, false);
        }
        let (tx, tw) = (col.x + side + 20.0, col.w - side - 20.0);
        self.ctx.set_font(&font(20.0));
        let mut text_h = self.paragraph(note, tx, col.y, tw, 20.0, "#ede6d6");
        if let Some(c) = self.game.content.credits.iter().find(|c| c.id == photo) {
            let line = format!("Photo: {} · {}", c.author, c.license);
            self.ctx.set_font(&font(15.0));
            text_h += self.paragraph(&line, tx, col.y + text_h, tw, 15.0, "#b0a48c");
        }
        col.y += text_h.max(side) + 22.0;
    }

    /// One field-guide row: picture and name.
    pub(super) fn specimen(&mut self, sprite: &str, frame: u32, note: &str, col: &mut Column) {
        let y = col.y - self.scroll;
        if let Some(s) = self.game.content.sprites.get(sprite).copied() {
            // Fit what can be seen, not the padded cell, into a 64×52 slot.
            let [ax, ay, aw, ah] = s.art.map(f64::from);
            let k = (64.0 / aw).min(52.0 / ah).min(3.0);
            let left = col.x + (64.0 - aw * k) / 2.0 - ax * k;
            let top = y + (52.0 - ah * k) / 2.0 - ay * k;
            self.sprite(sprite, frame, left, top, k, false);
        }
        self.ctx.set_font(&font(21.0));
        let h = self.paragraph(
            note,
            col.x + 76.0,
            col.y + 10.0,
            col.w - 76.0,
            21.0,
            "#ede6d6",
        );
        col.y += (h + 14.0).max(60.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_finger_can_tap_while_the_first_keeps_steering() {
        let mut c = Controls::default();
        c.press(1, (10.0, 10.0), 0.0);
        assert_eq!(c.steering(50.0), None, "too soon to tell from a tap");
        assert_eq!(c.steering(TAP_MS), Some((10.0, 10.0)));
        c.press(2, (200.0, 90.0), 1000.0);
        assert_eq!(c.lift(2, (201.0, 90.0), 1050.0), Some(true));
        assert!(c.drag(1, (40.0, 10.0)));
        assert_eq!(c.steering(1100.0), Some((40.0, 10.0)));
        assert_eq!(c.lift(1, (40.0, 10.0), 1200.0), Some(false));
        assert_eq!(c.lift(1, (40.0, 10.0), 1200.0), None);
    }
}

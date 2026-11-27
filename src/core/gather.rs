//! Poaching and foraging: a short real-time round in a small field. Pure and
//! seeded; the web layer only feeds it input and draws what is here.

use crate::core::config::*;
use crate::core::defs::{FieldDef, GatherDef, GatherMode};
use crate::core::rng::Rng;

/// Field shapes (width, height) in field units: wide screens and tall phones.
pub const LANDSCAPE: (f64, f64) = (320.0, 180.0);
pub const PORTRAIT: (f64, f64) = (220.0, 260.0);

/// How far from the edges things stay.
const MARGIN: f64 = 14.0;
/// How far off the field an animal wanders before it is gone.
const GONE: f64 = 50.0;

/// An animal or plant in the field. (`x`, `y`) is its centre.
#[derive(Debug, Clone)]
pub struct Thing {
    /// Index into [`Session::kinds`].
    pub kind: usize,
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub fleeing: bool,
    /// Buried in a pile, so nobody knows what it is yet.
    pub hidden: bool,
    /// Seconds before something worthless, once dug up, sinks away again.
    pub fade: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct Shot {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
}

/// A dumpster find being announced.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Flash {
    /// Index into [`Session::kinds`].
    pub kind: usize,
    /// In the bag, as against just uncovered.
    pub taken: bool,
    /// Seconds it has left on screen.
    pub left: f64,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub mode: GatherMode,
    /// The trip day it set out on.
    pub day: u32,
    /// Photo backdrop (`img/<id>.jpg`), where in it the skyline is, and where
    /// the skyline falls in the field.
    pub backdrop: Option<FieldDef>,
    /// Nothing walks above this height.
    pub top: f64,
    /// The plant just picked and how long its photo stays up.
    pub picked: Option<(usize, f64)>,
    pub flash: Option<Flash>,
    /// Field size.
    pub width: f64,
    pub height: f64,
    pub kinds: Vec<GatherDef>,
    pub things: Vec<Thing>,
    pub shots: Vec<Shot>,
    /// The gatherer.
    pub x: f64,
    pub y: f64,
    /// Unit vector the gatherer faces.
    pub face: (f64, f64),
    pub moving: bool,
    /// Where a tap told the gatherer to walk.
    goal: Option<(f64, f64)>,
    pub time_left: f64,
    /// Rounds left.
    pub ammo: u32,
    pub fired: u32,
    /// Food servings that will make it home.
    pub food: u32,
    /// Pounds of meat shot, including what can't be carried.
    pub meat_shot: u32,
    /// Poisonous plants picked.
    pub poison: u32,
    /// Feedback for the last thing that happened.
    pub message: String,
    cooldown: f64,
    spawn_in: f64,
    rng: Rng,
}

fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

impl Session {
    pub fn new(
        mode: GatherMode,
        (width, height): (f64, f64),
        all: &[GatherDef],
        ammo: u32,
        mut rng: Rng,
    ) -> Self {
        let kinds: Vec<GatherDef> = all.iter().filter(|k| k.mode == mode).cloned().collect();
        rng.unit();
        let mut s = Session {
            mode,
            day: 0,
            backdrop: None,
            top: 0.0,
            picked: None,
            flash: None,
            width,
            height,
            kinds,
            things: Vec::new(),
            shots: Vec::new(),
            x: width / 2.0,
            y: height * 0.7,
            face: (1.0, 0.0),
            moving: false,
            goal: None,
            time_left: GATHER_SECONDS,
            ammo,
            fired: 0,
            food: 0,
            meat_shot: 0,
            poison: 0,
            message: match mode {
                GatherMode::Poach => "Line them up and shoot.".into(),
                GatherMode::Forage => "Pick the good ones. Leave the death camas.".into(),
                GatherMode::Scavenge => {
                    "You're in. Dig through the piles; take only what's good.".into()
                }
            },
            cooldown: 0.0,
            spawn_in: 0.0,
            rng,
        };
        match mode {
            GatherMode::Poach => (0..2).for_each(|_| s.spawn_animal(true)),
            GatherMode::Forage | GatherMode::Scavenge => {
                (0..PLANT_PATCH).for_each(|_| s.spawn_plant());
            }
        }
        s
    }

    /// Put the round in front of a photo: everything moves below its skyline.
    pub fn set_backdrop(&mut self, field: FieldDef) {
        self.top = self.height * field.sky;
        self.backdrop = Some(field);
        self.y = self.y.max(self.top + MARGIN);
        let count = self.things.len();
        self.things.clear();
        match self.mode {
            GatherMode::Poach => (0..count).for_each(|_| self.spawn_animal(true)),
            GatherMode::Forage | GatherMode::Scavenge => {
                (0..PLANT_PATCH).for_each(|_| self.spawn_plant());
            }
        }
    }

    pub fn over(&self) -> bool {
        self.time_left <= 0.0
            || match self.mode {
                // A full pack doesn't stop anyone shooting.
                GatherMode::Poach => self.ammo == 0 && self.shots.is_empty(),
                GatherMode::Forage | GatherMode::Scavenge => self.food >= GATHER_CARRY,
            }
    }

    /// Pounds of meat being carried back.
    pub fn carried(&self) -> u32 {
        self.meat_shot.min(MEAT_CARRY_LB)
    }

    /// True just after a shot, while the gun is still up.
    pub fn aiming(&self) -> bool {
        self.cooldown > 0.0
    }

    /// A kind index, chosen by weight.
    fn pick_kind(&mut self) -> Option<usize> {
        let total: u32 = self.kinds.iter().map(|k| k.weight).sum();
        let mut roll = (self.rng.unit() * f64::from(total)) as u32;
        self.kinds.iter().position(|k| {
            if roll < k.weight {
                true
            } else {
                roll -= k.weight;
                false
            }
        })
    }

    fn between(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.rng.unit() * (hi - lo)
    }

    fn spawn_animal(&mut self, on_field: bool) {
        let Some(kind) = self.pick_kind() else {
            return;
        };
        let rightward = self.rng.chance(0.5);
        let x = if on_field {
            self.between(MARGIN, self.width - MARGIN)
        } else if rightward {
            -GONE / 2.0
        } else {
            self.width + GONE / 2.0
        };
        let y = self.between(self.top + MARGIN, self.height - FIELD_FLOOR);
        let speed = self.kinds[kind].speed;
        let vy = self.between(-0.2, 0.2) * speed;
        self.things.push(Thing {
            kind,
            x,
            y,
            vx: if rightward { speed } else { -speed },
            vy,
            fleeing: false,
            hidden: false,
            fade: 0.0,
        });
    }

    fn spawn_plant(&mut self) {
        let Some(kind) = self.pick_kind() else {
            return;
        };
        // Try for a spot clear of other plants and of the gatherer's feet.
        for _ in 0..30 {
            let at = (
                self.between(PLANT_EDGE, self.width - PLANT_EDGE),
                self.between(self.top + MARGIN, self.height - FIELD_FLOOR),
            );
            let clear = distance(at, (self.x, self.y)) > PLANT_CLEARANCE
                && self
                    .things
                    .iter()
                    .all(|t| distance(at, (t.x, t.y)) > PLANT_SPACING);
            if clear {
                self.things.push(Thing {
                    kind,
                    x: at.0,
                    y: at.1,
                    vx: 0.0,
                    vy: 0.0,
                    fleeing: false,
                    hidden: self.mode == GatherMode::Scavenge,
                    fade: 0.0,
                });
                return;
            }
        }
    }

    /// Walk to a spot (a tap). Foragers pick what they find on arrival.
    pub fn go_to(&mut self, x: f64, y: f64) {
        let plant = match self.mode {
            GatherMode::Forage | GatherMode::Scavenge => self
                .things
                .iter()
                .map(|t| (distance((x, y), (t.x, t.y)), (t.x, t.y)))
                .filter(|(d, _)| *d <= TAP_SNAP)
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .map(|(_, at)| at),
            GatherMode::Poach => None,
        };
        let (x, y) = plant.unwrap_or((x, y));
        self.goal = Some(self.walkable(x, y));
    }

    /// The nearest spot the gatherer can stand on.
    fn walkable(&self, x: f64, y: f64) -> (f64, f64) {
        (
            x.clamp(MARGIN, self.width - MARGIN),
            y.clamp(self.top + MARGIN, self.height - WALK_FLOOR),
        )
    }

    /// Shoot toward a point, or straight ahead.
    pub fn fire(&mut self, at: Option<(f64, f64)>) {
        if self.mode != GatherMode::Poach || self.over() || self.cooldown > 0.0 || self.ammo == 0 {
            return;
        }
        if let Some((tx, ty)) = at {
            let d = distance((tx, ty), (self.x, self.y));
            if d > 1.0 {
                self.face = ((tx - self.x) / d, (ty - self.y) / d);
            }
        }
        self.ammo -= 1;
        self.fired += 1;
        self.cooldown = SHOT_COOLDOWN;
        self.shots.push(Shot {
            x: self.x,
            y: self.y,
            vx: self.face.0 * BULLET_SPEED,
            vy: self.face.1 * BULLET_SPEED,
        });
        // The bang scares everything.
        for t in self.things.iter_mut().filter(|t| !t.fleeing) {
            t.fleeing = true;
            t.vx *= FLEE_MULTIPLIER;
            t.vy *= FLEE_MULTIPLIER;
        }
    }

    /// Pick the nearest plant within reach.
    pub fn pick(&mut self) {
        if self.mode == GatherMode::Poach || self.over() {
            return;
        }
        let me = (self.x, self.y);
        let nearest = self
            .things
            .iter()
            .enumerate()
            .map(|(i, t)| (distance(me, (t.x, t.y)), i))
            .filter(|(d, _)| *d <= PICK_REACH)
            .min_by(|a, b| a.0.total_cmp(&b.0));
        let Some((_, i)) = nearest else {
            return;
        };
        // The first go at a pile only digs it open; taking is a choice.
        if self.things[i].hidden {
            self.dig(i);
            return;
        }
        let index = self.things.remove(i).kind;
        self.picked = Some((index, PHOTO_SECONDS));
        self.announce(index, true);
        let kind = &self.kinds[index];
        if kind.poison {
            self.poison += 1;
            self.message = format!("{}! That one is poisonous.", kind.name);
        } else if kind.food == 0 {
            self.message = format!("{}. {}", kind.name, kind.fact);
        } else {
            self.message = format!("+{} {}. {}", kind.food, kind.name, kind.fact);
            self.food = (self.food + kind.food).min(GATHER_CARRY);
        }
        self.spawn_plant();
    }

    /// Open the pile at `i` and see what it was hiding.
    fn dig(&mut self, i: usize) {
        let Some(thing) = self.things.get_mut(i) else {
            return;
        };
        thing.hidden = false;
        let index = thing.kind;
        let kind = &self.kinds[index];
        let verdict = if kind.poison {
            thing.fade = SHOW_BAD_SECONDS;
            "Leave it!"
        } else if kind.food == 0 {
            thing.fade = SHOW_BAD_SECONDS;
            "Not worth it."
        } else {
            "Take it?"
        };
        self.picked = Some((index, PHOTO_SECONDS));
        self.message = format!("You dig up: {}. {verdict}", kind.name);
        self.announce(index, false);
    }

    /// Shout about a dumpster find. Plants are quieter: their photo shows.
    fn announce(&mut self, kind: usize, taken: bool) {
        if self.mode == GatherMode::Scavenge {
            self.flash = Some(Flash {
                kind,
                taken,
                left: FLASH_SECONDS,
            });
        }
    }

    /// Rubbish left lying slides back under, and a fresh pile turns up.
    fn sink(&mut self, dt: f64) {
        let mut gone = 0;
        self.things.retain_mut(|t| {
            if t.fade <= 0.0 {
                return true;
            }
            t.fade -= dt;
            let stays = t.fade > 0.0;
            gone += usize::from(!stays);
            stays
        });
        for _ in 0..gone {
            self.spawn_plant();
        }
    }

    /// Advance by `dt` seconds with the steering direction (`dx`, `dy`).
    pub fn update(&mut self, dt: f64, dx: f64, dy: f64) {
        if self.over() {
            self.moving = false;
            return;
        }
        let dt = dt.clamp(0.0, 0.1);
        self.time_left -= dt;
        self.cooldown = (self.cooldown - dt).max(0.0);
        self.picked = self
            .picked
            .map(|(k, left)| (k, left - dt))
            .filter(|(_, left)| *left > 0.0);
        self.flash = self
            .flash
            .map(|f| Flash {
                left: f.left - dt,
                ..f
            })
            .filter(|f| f.left > 0.0);
        self.walk(dt, dx, dy);
        if self.mode == GatherMode::Scavenge {
            self.sink(dt);
        }
        if self.mode == GatherMode::Poach {
            self.move_shots(dt);
            let (high, low) = (self.top + MARGIN, self.height - FIELD_FLOOR);
            for t in &mut self.things {
                t.x += t.vx * dt;
                t.y = (t.y + t.vy * dt).clamp(high, low);
            }
            let right = self.width + GONE;
            self.things.retain(|t| t.x > -GONE && t.x < right);
            self.spawn_in -= dt;
            if self.things.len() < HERD_SIZE && self.spawn_in <= 0.0 {
                self.spawn_animal(false);
                self.spawn_in = SPAWN_SECONDS;
            }
        }
    }

    fn walk(&mut self, dt: f64, mut dx: f64, mut dy: f64) {
        let mut step = WALK_SPEED * dt;
        let mut arriving = false;
        if dx != 0.0 || dy != 0.0 {
            self.goal = None;
        } else if let Some((gx, gy)) = self.goal {
            let d = distance((gx, gy), (self.x, self.y));
            if d <= step {
                step = d;
                arriving = true;
            }
            (dx, dy) = (gx - self.x, gy - self.y);
        }
        let len = dx.hypot(dy);
        self.moving = len > 1e-6;
        if self.moving {
            self.face = (dx / len, dy / len);
            (self.x, self.y) =
                self.walkable(self.x + self.face.0 * step, self.y + self.face.1 * step);
        }
        if arriving {
            self.goal = None;
            self.pick();
        }
    }

    fn move_shots(&mut self, dt: f64) {
        let (width, height) = (self.width, self.height);
        // Small steps, so a fast shot cannot skip over a rabbit.
        let steps = (BULLET_SPEED * dt / 4.0).ceil().max(1.0) as u32;
        let slice = dt / f64::from(steps);
        for _ in 0..steps {
            let mut i = 0;
            while i < self.shots.len() {
                let shot = &mut self.shots[i];
                shot.x += shot.vx * slice;
                shot.y += shot.vy * slice;
                let at = (shot.x, shot.y);
                let hit = self
                    .things
                    .iter()
                    .position(|t| distance(at, (t.x, t.y)) <= HIT_RADIUS * self.kinds[t.kind].size);
                if let Some(h) = hit {
                    let kind = &self.kinds[self.things.remove(h).kind];
                    self.meat_shot += kind.pounds;
                    let full = if self.meat_shot > MEAT_CARRY_LB {
                        format!(" You can only carry {MEAT_CARRY_LB} lb.")
                    } else {
                        String::new()
                    };
                    self.message = format!(
                        "{}: {} lb of meat.{full} {}",
                        kind.name, kind.pounds, kind.fact
                    );
                    self.food = self.carried() / POUNDS_PER_SERVING;
                    self.shots.remove(i);
                    continue;
                }
                let off = at.0 < 0.0 || at.0 > width || at.1 < 0.0 || at.1 > height;
                if off {
                    self.shots.remove(i);
                } else {
                    i += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::load_gather;

    /// Servings a named plant gives, as the data has it.
    fn yields(name: &str) -> u32 {
        let found = load_gather().into_iter().find(|k| k.name == name);
        found.expect("a known plant").food
    }

    fn session(mode: GatherMode, ammo: u32) -> Session {
        Session::new(mode, LANDSCAPE, &load_gather(), ammo, Rng::for_day(1, 1))
    }

    /// Replace the field with a single thing of the named kind.
    fn only(s: &mut Session, name: &str, x: f64, y: f64) {
        let kind = s.kinds.iter().position(|k| k.name == name).unwrap();
        s.things = vec![Thing {
            kind,
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            fleeing: true,
            hidden: false,
            fade: 0.0,
        }];
        s.spawn_in = 1e9;
    }

    #[test]
    fn a_good_shot_fills_the_bag_and_costs_a_round() {
        let mut s = session(GatherMode::Poach, 3);
        let y = s.y;
        only(&mut s, "Elk", 250.0, y);
        s.fire(Some((250.0, s.y)));
        for _ in 0..20 {
            s.update(0.05, 0.0, 0.0);
        }
        // All 220 lb were shot, but only 200 come home.
        assert_eq!((s.ammo, s.meat_shot, s.carried()), (2, 220, 200));
        assert_eq!(s.food, MEAT_CARRY_LB / POUNDS_PER_SERVING);
        assert!(!s.over(), "a full pack doesn't end the hunt");
        assert!(s.things.is_empty());
    }

    #[test]
    fn running_out_of_ammo_or_time_ends_the_round() {
        let mut s = session(GatherMode::Poach, 1);
        s.things.clear();
        s.spawn_in = 1e9;
        s.fire(None);
        assert!(!s.over(), "the last shot is still flying");
        for _ in 0..40 {
            s.update(0.05, 0.0, 0.0);
        }
        assert!(s.over());

        let mut s = session(GatherMode::Forage, 0);
        for _ in 0..1000 {
            s.update(0.1, 0.0, 0.0);
        }
        assert!(s.over());
    }

    #[test]
    fn tapping_a_plant_walks_there_and_picks_it() {
        let mut s = session(GatherMode::Forage, 0);
        only(&mut s, "Serviceberry", 60.0, 60.0);
        s.go_to(60.0, 60.0);
        for _ in 0..60 {
            s.update(0.05, 0.0, 0.0);
        }
        assert_eq!((s.food, s.poison), (yields("Serviceberry"), 0));
    }

    #[test]
    fn death_camas_is_no_meal() {
        let mut s = session(GatherMode::Forage, 0);
        let (x, y) = (s.x + 5.0, s.y);
        only(&mut s, "Death camas", x, y);
        s.pick();
        assert_eq!((s.food, s.poison), (0, 1));
    }

    #[test]
    fn a_tall_field_keeps_everything_inside() {
        let mut s = Session::new(
            GatherMode::Forage,
            PORTRAIT,
            &load_gather(),
            0,
            Rng::for_day(2, 1),
        );
        assert!(s.things.iter().all(|t| t.x < s.width && t.y < s.height));
        for _ in 0..100 {
            s.update(0.1, 1.0, 1.0);
        }
        assert!(s.x < s.width && s.y < s.height);
    }

    #[test]
    fn nothing_walks_in_the_sky() {
        let field = crate::core::data::load_fields().remove(0);
        for mode in [GatherMode::Poach, GatherMode::Forage] {
            let mut s = session(mode, 5);
            s.set_backdrop(field.clone());
            for _ in 0..200 {
                s.update(0.1, 0.0, -1.0);
                assert!(s.y >= s.top);
                assert!(s.things.iter().all(|t| t.y >= s.top));
            }
        }
    }

    #[test]
    fn tapping_just_past_the_edge_still_picks_the_plant() {
        let mut s = session(GatherMode::Forage, 0);
        s.set_backdrop(crate::core::data::load_fields().remove(0));
        let (x, y) = (60.0, s.top + MARGIN);
        only(&mut s, "Serviceberry", x, y);
        s.go_to(x, y - 12.0); // the upper half of the picture
        for _ in 0..80 {
            s.update(0.05, 0.0, 0.0);
        }
        assert_eq!(s.food, yields("Serviceberry"));
    }

    #[test]
    fn tapping_the_top_of_a_tall_plant_picks_it() {
        let mut s = session(GatherMode::Forage, 0);
        only(&mut s, "Cattail", 80.0, 90.0);
        s.go_to(84.0, 70.0); // the head, well above the plant's centre
        for _ in 0..80 {
            s.update(0.05, 0.0, 0.0);
        }
        assert_eq!(s.food, yields("Cattail"));
    }

    #[test]
    fn every_backdrop_still_grows_a_full_enough_patch() {
        for field in crate::core::data::load_fields() {
            for shape in [LANDSCAPE, PORTRAIT] {
                for seed in 0..20 {
                    let rng = Rng::for_day(seed, 1);
                    let mut s = Session::new(GatherMode::Forage, shape, &load_gather(), 0, rng);
                    s.set_backdrop(field.clone());
                    assert!(s.things.len() >= 8, "{}: {}", field.id, s.things.len());
                    assert!(s.things.iter().all(|t| {
                        t.y <= s.height - FIELD_FLOOR
                            && (PLANT_EDGE..=s.width - PLANT_EDGE).contains(&t.x)
                    }));
                }
            }
        }
    }

    #[test]
    fn each_mode_only_offers_its_own_things() {
        let hunt = session(GatherMode::Poach, 1);
        assert!(hunt.kinds.iter().all(|k| k.mode == GatherMode::Poach));
        assert!(!hunt.things.is_empty());
        let walk = session(GatherMode::Forage, 0);
        assert!(walk.things.len() > 6);
    }

    #[test]
    fn the_dumpster_holds_enough_piles_on_any_screen() {
        let fields = crate::core::data::load_fields();
        for field in fields.into_iter().filter(|f| f.scavenge) {
            for shape in [LANDSCAPE, PORTRAIT] {
                for seed in 0..20 {
                    let rng = Rng::for_day(seed, 1);
                    let mut s = Session::new(GatherMode::Scavenge, shape, &load_gather(), 0, rng);
                    s.set_backdrop(field.clone());
                    assert!(s.things.len() >= 8, "{}: {}", field.id, s.things.len());
                    assert!(s.things.iter().all(|t| t.hidden && t.y >= s.top));
                }
            }
        }
    }

    #[test]
    fn a_pile_must_be_dug_open_before_its_find_can_be_taken() {
        let mut s = session(GatherMode::Scavenge, 0);
        assert!(s.things.iter().all(|t| t.hidden));
        let (x, y) = (s.x + 4.0, s.y);
        only(&mut s, "Dented cans", x, y);
        s.things[0].hidden = true;
        s.pick();
        assert_eq!(s.food, 0, "the first go only digs");
        assert!(!s.things[0].hidden);
        assert!(s.message.contains("Dented cans"));
        s.pick();
        assert_eq!(s.food, yields("Dented cans"));
        assert!(s.things.iter().any(|t| t.hidden), "another pile turns up");
    }

    #[test]
    fn rubbish_can_be_left_to_sink_and_hurts_only_if_taken() {
        for (name, poison) in [("Mystery burrito", 1), ("Old sock", 0)] {
            let mut s = session(GatherMode::Scavenge, 0);
            let (x, y) = (s.x + 4.0, s.y);
            only(&mut s, name, x, y);
            s.things[0].hidden = true;
            s.pick();
            let mut taken = s.clone();
            taken.pick();
            assert_eq!((taken.food, taken.poison), (0, poison), "{name}");
            for _ in 0..40 {
                s.update(0.1, 0.0, 0.0);
            }
            assert_eq!(s.poison, 0, "left alone");
            let left = s
                .things
                .iter()
                .any(|t| s.kinds[t.kind].name == name && !t.hidden);
            assert!(!left, "{name} sank away");
        }
    }

    #[test]
    fn dumpster_finds_are_announced_and_the_card_clears_itself() {
        let mut s = session(GatherMode::Scavenge, 0);
        let (x, y) = (s.x + 4.0, s.y);
        only(&mut s, "Day-old bagels", x, y);
        s.things[0].hidden = true;
        assert!(s.flash.is_none());
        s.pick();
        assert!(s.flash.is_some_and(|f| !f.taken), "uncovered");
        s.pick();
        let flash = s.flash.expect("taken");
        assert!(flash.taken);
        assert_eq!(s.kinds[flash.kind].name, "Day-old bagels");
        for _ in 0..40 {
            s.update(0.1, 0.0, 0.0);
        }
        assert!(s.flash.is_none());
        // Picking a plant puts up no card.
        let mut walk = session(GatherMode::Forage, 0);
        let (x, y) = (walk.x + 4.0, walk.y);
        only(&mut walk, "Serviceberry", x, y);
        walk.pick();
        assert!(walk.flash.is_none());
    }
}

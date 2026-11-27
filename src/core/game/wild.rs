//! Poaching and foraging: starting a round and banking its results.

use super::*;

impl Game {
    /// How the party gets free food today: a forager takes to the dumpsters
    /// when the season grows nothing.
    pub fn gather_mode(&self) -> Option<GatherMode> {
        let grows = self.season(self.state.day).is_none_or(|s| s.forage);
        self.job().map(|j| match j.gather {
            GatherMode::Forage if !grows => GatherMode::Scavenge,
            other => other,
        })
    }

    /// Why there's no gathering where the party is, if there isn't.
    pub fn gather_blocked(&self) -> Option<&'static str> {
        if self.gather_mode() != Some(GatherMode::Scavenge) {
            return None;
        }
        let store = self.state.en_route.is_none()
            && self
                .node()
                .is_some_and(|n| n.services.food || n.services.fuel);
        (!store).then_some("Nothing grows in winter. Dive at a town with a store or gas station.")
    }

    /// The country the party is in: the road being driven, or the first road
    /// out of the current stop.
    pub fn local_terrain(&self) -> Terrain {
        // Out on a road, the stretch you are on; at a stop, where the first
        // road out begins.
        let found = match &self.state.en_route {
            Some(e) => {
                let done = 1.0 - e.remaining_miles / e.total_miles.max(1.0);
                self.content.map.road(e.road).map(|r| r.terrain_at(done))
            }
            None => {
                let out = self.content.map.outgoing(&self.state.current_node);
                out.first().map(|r| r.terrain_at(0.0))
            }
        };
        found.unwrap_or(Terrain::SagebrushBasin)
    }

    /// A photo of country like this; it changes from day to day.
    pub(super) fn backdrop(&self) -> Option<&FieldDef> {
        let terrain = self.local_terrain();
        let suited: Vec<&FieldDef> = self
            .content
            .fields
            .iter()
            .filter(|f| match self.gather_mode() {
                Some(GatherMode::Scavenge) => f.scavenge,
                _ => f.terrains.contains(&terrain),
            })
            .collect();
        suited
            .get(self.state.day as usize % suited.len().max(1))
            .copied()
    }

    /// Head out to poach or forage. Nothing changes until the round is
    /// handed back to [`Game::finish_gather`].
    pub fn start_gather(&self, field: (f64, f64)) -> Result<Session, String> {
        if !self.stopped_or_driving() {
            return Err("you can't do that right now".into());
        }
        if let Some(why) = self.gather_blocked() {
            return Err(why.into());
        }
        let mode = self.gather_mode().ok_or("no such profession")?;
        let ammo = self.ammo();
        if mode == GatherMode::Poach && ammo == 0 {
            return Err("You're out of ammo. Buy a box at a pump or outfitter.".into());
        }
        let rng = Rng::for_day(self.state.seed ^ 0x6A7E, self.state.day);
        let mut round = Session::new(mode, field, &self.content.gatherables, ammo, rng);
        round.day = self.state.day;
        if let Some(backdrop) = self.backdrop() {
            round.set_backdrop(backdrop.clone());
        }
        Ok(round)
    }

    /// Bank the round: ammo spent, food brought home, any trouble, one day.
    pub fn finish_gather(&mut self, round: &Session) -> Result<(), String> {
        if !self.stopped_or_driving() {
            return Err("you can't do that right now".into());
        }
        // A leftover round must not be banked twice, or on a later day.
        if round.day != self.state.day {
            return Err("that outing is over".into());
        }
        self.state.log.clear();
        let mut rng = Rng::for_day(self.state.seed ^ 0x3A9D, self.state.day);
        let mut food = round.food;
        if round.mode == GatherMode::Poach {
            self.set_ammo(self.ammo().saturating_sub(round.fired));
            // Towns and parks are watched; the open road less so.
            let protected =
                self.state.en_route.is_none() && self.node().is_some_and(|n| !n.hunting_allowed);
            let risk = if protected {
                WARDEN_PROTECTED_CHANCE
            } else {
                WARDEN_CHANCE
            };
            if round.fired > 0 && rng.chance(risk) {
                self.warden(&mut rng, true);
                food = 0;
            }
        } else if round.mode == GatherMode::Forage {
            food = (f64::from(food) * self.perk().forage_yield).round() as u32;
        } else if round.mode == GatherMode::Scavenge && rng.chance(MANAGER_CHANCE) {
            let scale = self.mood_scale(-MANAGER_MOOD_LOSS);
            for m in self.state.party.members.iter_mut().filter(|m| m.in_trip()) {
                m.apply_mood(-MANAGER_MOOD_LOSS * scale);
            }
            self.state
                .log
                .push("The night manager chases you off with a mop. You drop everything.".into());
            food = 0;
        }
        self.state.resources.food += food;
        self.state.resources.wild += food;
        self.state.log.push(match round.mode {
            GatherMode::Poach if food > 0 => format!(
                "Shot {} lb of meat and carried back {} lb: {food} servings.",
                round.meat_shot,
                round.carried()
            ),
            _ => format!("Brought back {food} servings of food."),
        });
        if round.poison > 0 {
            let alive: Vec<usize> = (0..self.state.party.members.len())
                .filter(|&i| self.state.party.members[i].in_trip())
                .collect();
            if let Some(i) = rng.choose(&alive) {
                let loss = POISON_HEALTH_LOSS * f64::from(round.poison);
                let bad = round.kinds.iter().find(|k| k.poison);
                let bad = bad.map_or("wrong thing".into(), |k| k.name.to_lowercase());
                let m = &mut self.state.party.members[i];
                let name = m.name.clone();
                let gone = m.apply_health(-loss);
                self.state
                    .log
                    .push(format!("{name} ate the {bad}: -{loss:.0} health."));
                if gone {
                    self.report_departures(vec![name]);
                }
            }
        }
        // Picking in a town or a park draws a ranger now and then.
        let watched =
            self.state.en_route.is_none() && self.node().is_some_and(|n| !n.hunting_allowed);
        if round.mode == GatherMode::Forage && watched && rng.chance(RANGER_CHANCE) {
            let paid = RANGER_FINE.min(self.state.resources.money);
            self.state.resources.lose_money(paid);
            self.state.log.push(format!(
                "A ranger explains that picking here is not allowed, at length, and fines you ${paid:.0}."
            ));
        }
        self.pass_day();
        if !self.check_party() {
            self.check_deadline();
        }
        Ok(())
    }

    /// A game warden catches up. What can't be paid is served in days. With
    /// `afield`, they also take the ammunition and the time, not just cash.
    pub(super) fn warden(&mut self, rng: &mut Rng, afield: bool) {
        let total: u32 = self.content.wardens.iter().map(|w| w.weight).sum();
        let mut roll = (rng.unit() * f64::from(total)) as u32;
        let found = self.content.wardens.iter().find(|w| {
            if roll < w.weight {
                true
            } else {
                roll -= w.weight;
                false
            }
        });
        let Some(w) = found.cloned() else {
            return;
        };
        let paid = w.fine.min(self.state.resources.money);
        self.state.resources.lose_money(paid);
        self.state
            .log
            .push(format!("{} Fine: ${:.0}.", w.text, w.fine));
        let mut days = if afield { w.days } else { 0 };
        if paid + 0.5 < w.fine {
            self.state
                .log
                .push("You can't cover it, so you work off the rest in jail.".into());
            days += 1;
        }
        if afield && w.ammo {
            self.set_ammo(0);
        }
        for _ in 0..days {
            self.pass_day();
        }
    }
}

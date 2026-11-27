//! Things that happen on the road, including hitchhikers.

use super::*;

impl Game {
    /// Answer the person by the road.
    pub fn pick_up(&mut self, give_ride: bool) -> Result<(), String> {
        self.expect_screen(Screen::Hitchhiker)?;
        self.state.log.clear();
        if !give_ride {
            self.state
                .log
                .push("You wave and keep driving. They wave back, mostly.".into());
            self.state.screen = self.state.next_screen;
            return Ok(());
        }
        let mut rng = Rng::for_day(self.state.seed ^ 0x40B0, self.state.day);
        let pool: Vec<&EventDef> = self.content.hitchhikers.iter().collect();
        let outcome = weighted(&pool, &mut rng).cloned().ok_or("nobody there")?;
        let report = self.apply_event(&outcome, &mut rng);
        if self.state.next_screen == Screen::Travel {
            // A shortcut can finish the leg.
            if let Some(e) = self.state.en_route.clone() {
                if e.remaining_miles <= 1e-6 {
                    self.state.screen = Screen::Travel;
                    self.arrive(&e.destination);
                    self.state.next_screen = self.state.screen;
                }
            }
        }
        self.state.event = Some(report);
        self.state.screen = Screen::Event;
        if self.check_party() || self.state.next_screen == Screen::Win {
            self.state.screen = self.state.next_screen;
            if self.state.party.all_left() {
                self.state.screen = Screen::GameOver;
            }
        }
        Ok(())
    }

    /// Buy a truck, then head into the starting store.
    pub fn buy_truck(&mut self, index: usize) -> Result<(), String> {
        self.expect_screen(Screen::Outfit)?;
        let def = self
            .content
            .vehicles
            .get(index)
            .filter(|v| v.for_sale)
            .ok_or("no such truck")?
            .clone();
        if !self.state.resources.pay(def.price) {
            return Err(format!("You can't afford the {}.", def.name));
        }
        self.state.truck = Truck::from_def(&def);
        self.state.screen = Screen::Store;
        self.state.next_screen = Screen::RoadChoice;
        Ok(())
    }

    pub(super) fn roll_event(
        &mut self,
        table: &str,
        risk: u8,
        weather: f64,
    ) -> Option<EventReport> {
        let mut rng = Rng::for_day(self.state.seed, self.state.day);
        if !rng.chance(EVENT_BASE_CHANCE + f64::from(risk) * EVENT_RISK_STEP + weather) {
            return None;
        }
        let ride = self.ride();
        let pool: Vec<&EventDef> = ["any", table]
            .iter()
            .filter_map(|t| self.content.events.events(t))
            .flatten()
            .filter(|e| e.ride.is_none_or(|r| r == ride))
            .collect();
        let event = weighted(&pool, &mut rng)?;
        let event = event.clone();
        Some(self.apply_event(&event, &mut rng))
    }

    pub(super) fn apply_event(&mut self, event: &EventDef, rng: &mut Rng) -> EventReport {
        let alive: Vec<usize> = (0..self.state.party.members.len())
            .filter(|&i| self.state.party.members[i].in_trip())
            .collect();
        let target = rng.choose(&alive);
        let name = target
            .map(|i| self.state.party.members[i].name.clone())
            .unwrap_or_else(|| "Someone".into());

        let saved = event.needs.as_deref().is_some_and(|id| {
            let item = self.content.items.get(id);
            let free = self.perk().free_repairs && item.is_some_and(|d| d.repair);
            let reusable = item.is_some_and(|d| d.reusable);
            free || if reusable {
                self.state.resources.count(id) > 0
            } else {
                self.state.resources.use_item(id)
            }
        });
        let (text, effect): (&str, Effect) = match (&event.saved_text, saved) {
            (Some(t), true) => (t, event.saved_effect),
            _ => (&event.text, event.effect),
        };

        let mut out = Vec::new();
        let r = &mut self.state.resources;
        if effect.money < 0.0 {
            let paid = (-effect.money).min(r.money);
            r.lose_money(paid);
            out.push(format!("-${paid:.0}"));
        } else if effect.money > 0.0 {
            r.money += effect.money;
            out.push(format!("+${:.0}", effect.money));
        }
        if effect.food != 0 {
            let before = r.food;
            r.food = r.food.saturating_add_signed(effect.food);
            r.wild = r.wild.min(r.food);
            out.push(format!("{:+} food", i64::from(r.food) - i64::from(before)));
        }
        if effect.fuel < 0.0 {
            let used = self.state.truck.drain(-effect.fuel);
            out.push(format!("-{used:.1} gal"));
        } else if effect.fuel > 0.0 {
            let added = self.state.truck.add_fuel(effect.fuel);
            out.push(format!("+{added:.1} gal"));
        }
        let scale = self.mood_scale(effect.mood);
        let mut gone = Vec::new();
        for (i, m) in self.state.party.members.iter_mut().enumerate() {
            if !m.in_trip() {
                continue;
            }
            m.apply_mood(effect.mood * scale);
            let hurt = effect.health != 0.0 && (!event.one || Some(i) == target);
            if hurt && m.apply_health(effect.health) {
                gone.push(m.name.clone());
            }
        }
        if effect.health != 0.0 {
            let who = if event.one { name.as_str() } else { "Everyone" };
            out.push(format!("{who}: {:+.0} health", effect.health));
        }
        if effect.mood != 0.0 {
            out.push(if effect.mood > 0.0 {
                "Spirits rise".into()
            } else {
                "Spirits sink".into()
            });
        }
        if effect.miles != 0.0 {
            if let Some(e) = self.state.en_route.as_mut() {
                let before = e.remaining_miles;
                e.remaining_miles = (before + effect.miles).max(0.0);
                let changed = e.remaining_miles - before;
                if changed > 0.0 {
                    // The leg gets longer, so miles already covered stay put.
                    // Driving the extra miles is what burns the gas.
                    e.total_miles += changed;
                    out.push(format!("{changed:.0} extra miles"));
                } else if changed < 0.0 {
                    out.push(format!("{:.0} miles saved", -changed));
                }
            }
        }
        if effect.days > 0 {
            let s = if effect.days == 1 { "" } else { "s" };
            out.push(format!("Lost {} day{s}", effect.days));
            for _ in 0..effect.days {
                self.pass_day();
            }
        }
        self.report_departures(gone);
        EventReport {
            name: event.name.clone(),
            text: text.replace("{name}", &name),
            consequences: out,
        }
    }
}

//! Driving: setting off, the drive day, running dry and arriving.

use super::*;
use crate::core::defs::Ride;

impl Game {
    /// Leave `current_node` on a road.
    pub fn set_off(&mut self, road_index: usize) -> Result<(), String> {
        if !self.stopped() {
            return Err("you can't set off right now".into());
        }
        let road = self
            .content
            .map
            .road(road_index)
            .ok_or_else(|| format!("no such road #{road_index}"))?;
        if road.from != self.state.current_node || road.closed {
            return Err(format!("{} is not open from here", road.highway));
        }
        self.state.en_route = Some(EnRoute {
            road: road_index,
            from: road.from.clone(),
            destination: road.to.clone(),
            remaining_miles: road.distance_miles,
            total_miles: road.distance_miles,
        });
        self.state.screen = Screen::Travel;
        self.state.log.clear();
        Ok(())
    }

    /// Still at the start of a road, so a wrong choice can be undone.
    pub fn can_turn_back(&self) -> bool {
        self.state.screen == Screen::Travel
            && self
                .state
                .en_route
                .as_ref()
                .is_some_and(|e| e.remaining_miles >= e.total_miles)
    }

    /// Pick a different road before any miles are driven. Costs nothing.
    pub fn turn_back(&mut self) -> Result<(), String> {
        if !self.can_turn_back() {
            return Err("you're already on your way".into());
        }
        self.state.en_route = None;
        self.state.screen = Screen::RoadChoice;
        self.state.log.clear();
        Ok(())
    }

    /// Why driving shouldn't carry on into another day without the player
    /// saying so.
    pub fn reason_to_stop(&self) -> Option<String> {
        let e = self.state.en_route.as_ref()?;
        let ride = self.ride_def();
        let road = self.content.map.road(e.road)?;
        let pace = PACES.get(self.state.pace_index)?;
        let mut reach = miles_per_day(pace.hours, road.terrain);
        if let Some(most) = self.vehicle().and_then(|v| v.day_miles) {
            reach = reach.min(most * pace.hours / PACES[1].hours);
        }
        if self.range_miles() + 1e-6 < reach.min(e.remaining_miles) {
            return Some(format!("There isn't the {} for another day.", ride.fuel));
        }
        if self.state.resources.food < self.daily_food() {
            return Some("There isn't food for another day.".into());
        }
        let poorly = self.state.party.in_trip().into_iter();
        let poorly = poorly
            .filter(|m| m.health < PULL_OVER_HEALTH)
            .min_by(|a, b| a.health.total_cmp(&b.health));
        poorly.map(|m| format!("{} is in a bad way and needs rest.", m.name))
    }

    /// One day goes by: the calendar advances and everyone eats.
    pub(super) fn pass_day(&mut self) {
        self.state.day += 1;
        let meal = MEALS[self.state.meal_index.min(MEALS.len() - 1)];
        let wanted = self.daily_food();
        let eaten = wanted.min(self.state.resources.food);
        self.state.resources.food -= eaten;
        // Shop food goes first; the wild stuff is whatever is left of it.
        self.state.resources.wild = self.state.resources.wild.min(self.state.resources.food);
        if eaten < wanted {
            self.state
                .log
                .push("Not enough food. Stomachs growl.".into());
        }
        let mood = meal.mood_delta * self.mood_scale(meal.mood_delta);
        let mut gone = Vec::new();
        for m in self.state.party.members.iter_mut().filter(|m| m.in_trip()) {
            m.apply_mood(mood);
            if eaten < wanted {
                m.apply_fatigue(STARVE_FATIGUE_GAIN);
                if m.apply_health(-STARVE_HEALTH_LOSS) {
                    gone.push(m.name.clone());
                }
            }
        }
        self.report_departures(gone);
    }

    pub(super) fn mood_scale(&self, delta: f64) -> f64 {
        if delta < 0.0 {
            self.perk().mood_loss
        } else {
            1.0
        }
    }

    pub(super) fn report_departures(&mut self, names: Vec<String>) {
        for name in names {
            self.state.log.push(format!(
                "{name} is taken to the hospital and leaves the trip."
            ));
        }
    }

    /// Drive one day — the core loop step.
    pub fn drive_day(&mut self) -> Result<(), String> {
        self.expect_screen(Screen::Travel)?;
        let en = self.state.en_route.clone().ok_or("not en route")?;
        let road = self
            .content
            .map
            .road(en.road)
            .cloned()
            .ok_or("en-route road is missing from the map")?;
        let pace = *PACES.get(self.state.pace_index).ok_or("bad pace")?;
        self.state.log.clear();
        self.record();
        let started = self.state.day;

        // --- miles & fuel -----------------------------------------------------
        let sky = self.weather_on(self.state.day).cloned();
        let mut speed = sky.as_ref().map_or(1.0, |w| w.speed);
        if road.terrain == Terrain::MountainPass {
            speed *= sky.as_ref().map_or(1.0, |w| w.pass_speed);
        }
        let mut reach = miles_per_day(pace.hours, road.terrain);
        if let Some(most) = self.vehicle().and_then(|v| v.day_miles) {
            reach = reach.min(most * pace.hours / PACES[1].hours);
        }
        let planned = (reach * speed).min(en.remaining_miles);
        while self.state.truck.gallons_for(planned) > self.state.truck.fuel_gallons {
            let Some(gallons) = self.use_fuel_can() else {
                break;
            };
            self.state.truck.add_fuel(gallons);
            let line = self.ride_def().refill.clone();
            self.state.log.push(line);
        }
        let range = self.state.truck.current_range_miles();
        let ran_dry = range + 1e-6 < planned;
        let miles = planned.min(range);
        self.state.truck.drain(self.state.truck.gallons_for(miles));
        let remaining = (en.remaining_miles - miles).max(0.0);
        if let Some(e) = self.state.en_route.as_mut() {
            e.remaining_miles = remaining;
        }
        let plural = if miles.round() == 1.0 { "" } else { "s" };
        let past = self.ride_def().past.clone();
        self.state.log.push(format!(
            "{past} {miles:.0} mile{plural} on {}.",
            road.highway
        ));
        self.log_miles(started, miles);

        if let Some(w) = &sky {
            let line = format!("Weather: {}. {}", w.name, self.weather_text(w));
            self.state.log.push(line);
        }

        // --- the day itself ----------------------------------------------------
        self.pass_day();
        // Resting pays the same for everyone; only tiring is eased.
        let tire = if pace.fatigue_delta > 0.0 {
            self.perk().fatigue
        } else {
            1.0
        };
        // Scenery nudges mood on pretty stretches.
        let mood = pace.mood_delta + (f64::from(road.scenery) - SCENERY_NEUTRAL) * SCENERY_MOOD;
        let weather_mood = sky.as_ref().map_or(0.0, |w| w.mood);
        let weather_health = sky.as_ref().map_or(0.0, |w| w.health);
        let mood = mood + weather_mood;
        let scale = self.mood_scale(mood);
        let mut gone = Vec::new();
        for m in self.state.party.members.iter_mut().filter(|m| m.in_trip()) {
            m.apply_fatigue(pace.fatigue_delta * tire);
            m.apply_mood(mood * scale);
            if weather_health < 0.0 && m.apply_health(weather_health) {
                gone.push(m.name.clone());
                continue;
            }
            if m.fatigue > EXHAUSTED_FATIGUE {
                if m.apply_health(-EXHAUSTED_HEALTH_LOSS) {
                    gone.push(m.name.clone());
                }
            } else if m.fatigue < WELL_RESTED_FATIGUE {
                m.apply_health(REST_HEALTH_GAIN);
                if m.status == MemberStatus::Injured {
                    m.status = MemberStatus::Healthy;
                }
            }
        }
        self.report_departures(gone);

        // --- random event -------------------------------------------------------
        // Stored at once, so an event that ends the trip is still shown.
        let weather_risk = sky.as_ref().map_or(0.0, |w| w.risk);
        self.state.event = self.roll_event(&road.events_table, road.seasonal_risk, weather_risk);
        // A wrong turn leaves more road to go.
        let remaining = self
            .state
            .en_route
            .as_ref()
            .map_or(0.0, |e| e.remaining_miles);

        // --- outcome ---------------------------------------------------------------
        if self.check_party() {
            return Ok(());
        }
        if ran_dry && remaining > 1e-6 && !self.rescue(remaining, &en.destination) {
            return Ok(());
        }
        if remaining <= 1e-6 && !self.overdue() {
            self.arrive(&en.destination);
        }
        self.check_deadline();
        if self.state.event.is_some() && self.stopped_or_driving() {
            self.state.next_screen = self.state.screen;
            self.state.screen = Screen::Event;
        } else if self.stopped_or_driving() {
            let mut rng = Rng::for_day(self.state.seed ^ 0x7A5E, self.state.day);
            // Nobody thumbs a lift from a horse.
            if self.ride() == Ride::Motor && rng.chance(HITCHHIKER_CHANCE) {
                self.state.next_screen = self.state.screen;
                self.state.screen = Screen::Hitchhiker;
            }
        }
        Ok(())
    }

    /// Out of fuel: pay to be rescued, or the trip is over.
    pub(super) fn rescue(&mut self, remaining: f64, destination: &str) -> bool {
        let ride = self.ride_def().clone();
        if self.state.resources.pay(ride.rescue_cost) {
            self.state.truck.add_fuel(ride.rescue_amount);
            self.state.log.push(
                ride.rescue
                    .replace("{cost}", &format!("{:.0}", ride.rescue_cost))
                    .replace("{amount}", &format!("{:.0}", ride.rescue_amount)),
            );
            self.pass_day();
            return !self.check_party();
        }
        let place = self
            .content
            .map
            .node(destination)
            .map_or(destination, |n| n.name.as_str());
        self.state.ending_reason = ride
            .stranded
            .replace("{miles}", &format!("{remaining:.0}"))
            .replace("{place}", place);
        self.finish(Screen::GameOver);
        false
    }

    /// End the run if nobody is left. Returns true if it ended.
    pub(super) fn check_party(&mut self) -> bool {
        if !self.state.party.all_left() {
            return false;
        }
        self.state.ending_reason = "The whole party left before the job was done.".into();
        self.finish(Screen::GameOver);
        true
    }

    /// Arrived at `node`: record the visit, then Win / Town screen.
    pub(super) fn arrive(&mut self, node: &str) {
        if !self.state.visited.iter().any(|v| v == node) {
            self.state.visited.push(node.to_string());
        }
        self.state.current_node = node.to_string();
        self.state.en_route = None;
        self.state.screen = Screen::Town;
        let Some(step) = self.current_step() else {
            return;
        };
        if !step.at.iter().any(|a| a == node) {
            return;
        }
        // "Rusty ..." reads as "your rusty ..."; proper nouns keep their case.
        let mut letters = self.state.truck.name.chars();
        let vehicle: String = letters
            .next()
            .map(|c| c.to_ascii_lowercase())
            .into_iter()
            .chain(letters)
            .collect();
        let done = step.done.replace("{vehicle}", &vehicle);
        let met = format!("[X] GOAL MET: {}", step.task);
        self.state.goal_step += 1;
        if self.current_step().is_some() {
            self.state.log.push(met);
            self.state.log.push(done);
        } else {
            self.state.ending_reason = done;
            self.finish(Screen::Win);
        }
    }

    /// Dismiss the arrival or event screen.
    pub fn continue_journey(&mut self) {
        match self.state.screen {
            Screen::Town => self.state.screen = Screen::RoadChoice,
            Screen::Event => {
                self.state.screen = self.state.next_screen;
                self.state.event = None;
            }
            _ => {}
        }
    }

    pub(super) fn finish(&mut self, screen: Screen) {
        let members = self.state.party.in_trip();
        let survivors = members.len();
        let avg_health = members.iter().map(|m| m.health).sum::<f64>() / survivors.max(1) as f64;
        let money = self.state.resources.money;
        let places = self.state.visited.len();
        let multiplier = self
            .content
            .professions
            .get(self.state.profession)
            .map_or(1, |p| p.multiplier);
        let total = if screen == Screen::Win {
            let base = survivors as f64 * SCORE_PER_SURVIVOR
                + avg_health * SCORE_PER_AVG_HEALTH
                + money * SCORE_PER_DOLLAR
                + places as f64 * SCORE_PER_PLACE
                + f64::from(self.state.day - 1) * SCORE_PER_DAY
                + self.collection().iter().map(|(_, d)| d.score).sum::<u64>() as f64;
            base.max(0.0) as u64 * u64::from(multiplier)
        } else {
            0
        };
        self.state.score = Score {
            party_survivors: survivors,
            days_traveled: self.state.day - 1,
            money_left: money,
            places_visited: places,
            total,
        };
        self.state.screen = screen;
    }
}

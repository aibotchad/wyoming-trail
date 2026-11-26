//! Core gameplay rules: setup, road choice, the drive-day loop, arrival,
//! win / lose. All functions are pure over `Game` (no I/O, no DOM) so they
//! are unit-testable on the host.

use crate::core::config::{
    DAYS_PER_MONTH, EXHAUSTED_FATIGUE, EXHAUSTED_HEALTH_LOSS, FUEL_PRICE_PER_GAL, MEALS,
    MEALS_PER_DAY, PACES, REST_HEALTH_GAIN, STARTING_FOOD, STARTING_MONEY, STARVE_FATIGUE_GAIN,
    STARVE_HEALTH_LOSS, WELL_RESTED_FATIGUE,
};
use crate::core::map_graph::{miles_per_day, MapGraph};
use crate::core::party::MemberStatus;
use crate::core::resources::Resources;
use crate::core::state::{EnRoute, GameState, Screen, DEFAULT_PARTY_NAMES};
use crate::core::truck::Truck;

/// Owns the map (reference data) plus the mutable run state.
#[derive(Debug, Clone)]
pub struct Game {
    pub map: MapGraph,
    pub state: GameState,
}

/// Report for one drive-day, surfaced to the UI for logging.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DayReport {
    pub miles_driven: f64,
    pub remaining_miles: f64,
    pub fuel_used_gallons: f64,
    pub meals_eaten: u32,
    /// True when the truck ran out of fuel mid-leg (trip-ending for M1).
    pub stranded: bool,
    /// True when the leg finished on this drive.
    pub arrived: bool,
}

/// Fuel price, re-exported so UIs reference the single source of truth.
#[allow(dead_code)] // first consumer: M3 (gas stations).
pub const FUEL_PRICE: f64 = FUEL_PRICE_PER_GAL;

impl Game {
    /// A fresh run at the Title/Outfit screen. `seed` makes the whole run
    /// reproducible.
    pub fn new_run(seed: u64) -> Self {
        let map = MapGraph::load();
        // Outfitting defaults: standard pickup, starting money & food.
        // The Outfit screen can change the truck before set_off.
        let start = map.start.clone();
        let state = GameState {
            seed,
            screen: Screen::Outfit,
            party: crate::core::party::Party::new(&DEFAULT_PARTY_NAMES),
            truck: Truck::from_def(crate::core::config::TRUCKS[1]),
            resources: Resources {
                money: STARTING_MONEY,
                food: STARTING_FOOD,
                ..Default::default()
            },
            current_node: start.clone(),
            en_route: None,
            day: 1,
            month: 1,
            pace_index: 1, // Standard
            meal_index: 0, // Cheap
            visited: vec![start],
            ending_reason: String::new(),
            score: Default::default(),
        };
        Game { map, state }
    }

    /// Choose which truck to travel in (Outfit screen).
    pub fn choose_truck(&mut self, index: usize) -> bool {
        match crate::core::config::TRUCKS.get(index) {
            Some(def) => {
                self.state.truck = Truck::from_def(*def);
                true
            }
            None => false,
        }
    }

    /// Begin (or continue) the trip by leaving `current_node` on a road.
    pub fn set_off(&mut self, road_index: usize) -> Result<(), String> {
        let road = self
            .map
            .road(road_index)
            .ok_or_else(|| format!("no such road #{road_index}"))?;
        if road.from != self.state.current_node {
            return Err(format!(
                "road {} does not leave the current node ({})",
                road.highway, self.state.current_node
            ));
        }
        self.state.en_route = Some(EnRoute {
            road: road_index,
            from: road.from.clone(),
            destination: road.to.clone(),
            remaining_miles: road.distance_miles,
            total_miles: road.distance_miles,
        });
        self.state.screen = Screen::Travel;
        Ok(())
    }

    /// Drive one day — the core loop step.
    pub fn drive_day(&mut self) -> Result<DayReport, String> {
        let en = self
            .state
            .en_route
            .clone()
            .ok_or("not en route — nothing to drive")?;
        let road = self
            .map
            .road(en.road)
            .expect("en_route road index is set by set_off on the same map");
        if road.from != en.from || road.to != en.destination {
            return Err("en-route data no longer matches the map".into());
        }

        let pace = PACES
            .get(self.state.pace_index)
            .expect("pace_index is set by the menu (0..2)");
        let meal = MEALS
            .get(self.state.meal_index)
            .expect("meal_index is set by the menu (0..2)");
        let destination = en.destination.clone();

        // --- miles & fuel ---------------------------------------------------
        let capacity = miles_per_day(pace.hours, road.terrain);
        let planned = capacity.min(en.remaining_miles);
        let gallons_wanted = self.state.truck.gallons_for(planned);
        let fuel_on_hand = self.state.truck.fuel_gallons;

        let (miles_driven, stranded) = if gallons_wanted <= fuel_on_hand {
            (planned, false)
        } else {
            // Fuel cannot cover the planned day-drive: go as far as it lasts.
            let miles = fuel_on_hand * self.state.truck.mpg;
            (miles, en.remaining_miles - miles > 1e-6)
        };
        let fuel_used = self.state.truck.gallons_for(miles_driven).min(fuel_on_hand);
        self.state.truck.drain(fuel_used);

        // --- food & meals (money) -------------------------------------------
        let alive = self.state.party.in_trip_count();
        let meals_needed = alive as u32 * MEALS_PER_DAY;
        let had_food = self.state.resources.food >= meals_needed;
        if had_food {
            self.state.resources.take_food(meals_needed);
        }
        let meal_cost = meal.cost_per_person * alive as f64;
        let _affordable = self.state.resources.pay(meal_cost); // unaffordable → skipped

        // --- party deltas -----------------------------------------------------
        for m in self.state.party.members.iter_mut() {
            if !m.in_trip() {
                continue;
            }
            m.apply_fatigue(pace.fatigue_delta);
            m.apply_mood(pace.mood_delta + meal.mood_delta);
            // Scenery nudges mood on scenic stretches (placeholder tuning).
            m.apply_mood((road.scenery as f64 - 2.5) * 0.5);
            if !had_food {
                let left = m.apply_health(-STARVE_HEALTH_LOSS);
                m.apply_fatigue(STARVE_FATIGUE_GAIN);
                if left {
                    self.state.ending_reason =
                        format!("{} ran out of food and had to leave the trip.", m.name);
                }
            }
            if m.fatigue > EXHAUSTED_FATIGUE {
                m.apply_health(-EXHAUSTED_HEALTH_LOSS);
            } else if m.fatigue < WELL_RESTED_FATIGUE {
                m.apply_health(REST_HEALTH_GAIN);
                if m.status == MemberStatus::Injured && m.health > 0.0 {
                    m.status = MemberStatus::Healthy;
                }
            }
        }

        // --- advance the road -------------------------------------------------
        if let Some(e) = self.state.en_route.as_mut() {
            e.remaining_miles = (e.remaining_miles - miles_driven).max(0.0);
        }

        self.advance_day();

        // --- outcome ----------------------------------------------------------
        let mut arrived = false;
        if self.state.party.all_left() {
            if self.state.ending_reason.is_empty() {
                self.state.ending_reason =
                    "The whole party left the trip before Yellowstone.".to_string();
            }
            self.finish(Screen::GameOver);
        } else if stranded {
            let remaining = self
                .state
                .en_route
                .as_ref()
                .map(|e| e.remaining_miles)
                .unwrap_or(0.0);
            self.state.ending_reason = format!(
                "The {} ran out of fuel, {:.0} miles short of {}.",
                self.state.truck.name, remaining, destination
            );
            self.finish(Screen::GameOver);
        } else if let Some(e) = self.state.en_route.clone() {
            if e.remaining_miles <= 1e-6 {
                arrived = true;
                self.arrive(&e.destination);
            }
        }

        Ok(DayReport {
            miles_driven,
            remaining_miles: self
                .state
                .en_route
                .as_ref()
                .map(|e| e.remaining_miles)
                .unwrap_or(0.0),
            fuel_used_gallons: fuel_used,
            meals_eaten: meals_needed,
            stranded,
            arrived,
        })
    }

    /// One day passes (simplified 30-day-month / 12-month calendar).
    pub fn advance_day(&mut self) {
        self.state.day += 1;
        if self.state.day > DAYS_PER_MONTH {
            self.state.day = 1;
            self.state.month += 1;
            if self.state.month > 12 {
                self.state.month = 1;
            }
        }
    }

    /// Arrived at `node`: record the visit, then Win / Town screen.
    fn arrive(&mut self, node: &str) {
        if !self.map.node(node).is_some() {
            // Should not happen with validated data; fail loudly in tests and
            // keep the trip going (rather than panic) if data is somehow bad.
            eprintln!("wyoming-trail: arrived at unknown node '{node}'");
        }
        if !self.state.visited.iter().any(|v| v == node) {
            self.state.visited.push(node.to_string());
        }
        self.state.current_node = node.to_string();
        self.state.en_route = None;

        if self.map.is_goal(node) {
            let month_days = (self.state.month - 1) * DAYS_PER_MONTH + self.state.day;
            self.state.score = crate::core::state::Score {
                party_survivors: self.state.party.in_trip_count(),
                days_traveled: month_days,
                money_left: self.state.resources.money,
                landmarks_visited: self.state.visited.len(),
                npcs_helped: 0,
            };
            self.state.ending_reason = format!(
                "The {} rolled into Yellowstone National Park. Yellowstone reached!",
                self.state.truck.name
            );
            self.finish(Screen::Win);
        } else {
            self.state.screen = Screen::Town;
        }
    }

    /// Leave a town/junction: back to choosing the next road.
    pub fn continue_journey(&mut self) {
        if self.state.screen == Screen::Town {
            self.state.screen = Screen::RoadChoice;
        }
    }

    /// Start a brand-new run (Play Again) — fresh seed for a fresh stream.
    #[allow(dead_code)] // first consumer: M2 (title screen Play Again).
    pub fn restart(seed: u64) -> Self {
        Game::new_run(seed)
    }

    fn finish(&mut self, screen: Screen) {
        self.state.screen = screen;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> Game {
        Game::new_run(42)
    }

    #[test]
    fn outfitting_has_three_trucks() {
        let mut g = game();
        for i in 0..3 {
            assert!(g.choose_truck(i));
        }
        assert!(!g.choose_truck(99));
        assert_eq!(g.state.truck.name, "Diesel workhorse");
    }

    #[test]
    fn start_node_offers_two_routes() {
        let g = game();
        let start = &g.state.current_node;
        let roads = g.map.outgoing(start);
        assert!(roads.len() >= 2, "Cheyenne must offer at least two routes");
        let dests: std::collections::HashSet<&str> = roads.iter().map(|r| r.to.as_str()).collect();
        assert!(dests.len() >= 2, "routes must head in different directions");
        for d in &dests {
            assert!(
                g.map.reachable(d, "yellowstone_south") || g.map.reachable(d, "yellowstone_east"),
                "route via {d} must be able to reach Yellowstone"
            );
        }
    }

    #[test]
    fn set_off_requires_outgoing_road() {
        let mut g = game();
        let idx = g
            .map
            .roads
            .iter()
            .position(|r| r.from == g.map.start)
            .expect("start has outgoing roads");
        assert!(g.set_off(idx).is_ok());
        assert_eq!(g.state.screen, Screen::Travel);
        assert!(g.state.en_route.is_some());
    }

    #[test]
    fn drive_day_consumes_fuel_food_and_advances_day() {
        let mut g = game();
        let idx = g
            .map
            .roads
            .iter()
            .position(|r| r.from == g.map.start)
            .unwrap();
        g.set_off(idx).unwrap();
        let fuel_before = g.state.truck.fuel_gallons;
        let food_before = g.state.resources.food;
        let day_before = g.state.day;

        let report = g.drive_day().unwrap();
        assert!(report.miles_driven > 0.0);
        assert!(g.state.truck.fuel_gallons < fuel_before);
        assert!(g.state.resources.food < food_before);
        assert_eq!(report.meals_eaten, 4, "4 members × 1 meal/day");
        assert_eq!(g.state.day, day_before + 1);
    }

    #[test]
    fn driving_out_and_back_wins_the_game() {
        let mut g = game();
        // Drive day after day across the map until we win. Safety: 500 days is
        // far more than needed for the test map.
        let mut days = 0;
        loop {
            assert!(days < 500, "safety: too many days");
            days += 1;

            match g.state.screen {
                Screen::Outfit => {
                    let idx = g
                        .map
                        .roads
                        .iter()
                        .position(|r| r.from == g.map.start)
                        .unwrap();
                    g.set_off(idx).unwrap();
                }
                Screen::Travel => {
                    let report = g.drive_day().unwrap();
                    if report.stranded {
                        panic!("got stranded: {}", g.state.ending_reason);
                    }
                }
                Screen::Town | Screen::RoadChoice => {
                    let idx = g
                        .map
                        .roads
                        .iter()
                        .position(|r| r.from == g.state.current_node && !r.closed)
                        .unwrap_or_else(|| panic!("no road out of {}", g.state.current_node));
                    g.set_off(idx).unwrap();
                }
                Screen::Win => break,
                other => panic!("unexpected screen {other:?}"),
            }
        }
        assert_eq!(g.state.screen, Screen::Win);
        assert_eq!(g.state.score.party_survivors, 4);
        assert!(!g.state.visited.is_empty());
        assert!(
            g.state.visited.iter().any(|v| g.map.is_goal(v)),
            "the goal node must have been visited"
        );
    }

    #[test]
    fn running_out_of_fuel_ends_the_trip() {
        let mut g = game();
        let idx = g
            .map
            .roads
            .iter()
            .position(|r| r.from == g.map.start)
            .unwrap();
        g.set_off(idx).unwrap();
        // Drained the tank: next day of driving must strand the party.
        g.state.truck.fuel_gallons = 1.0; // barely any fuel left
        let report = g.drive_day().unwrap();
        assert!(report.stranded || report.arrived);
        if report.stranded {
            assert_eq!(g.state.screen, Screen::GameOver);
            assert!(g.state.truck.fuel_gallons == 0.0);
        }
    }
}

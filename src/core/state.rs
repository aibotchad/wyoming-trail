//! Serializable game state: the screen the player is on and everything the
//! rules need to keep going (party, truck, resources, calendar, route).

use serde::{Deserialize, Serialize};

use crate::core::config::DEFAULT_MONTH;

use crate::core::defs::VehicleDef;
use crate::core::party::Party;
use crate::core::resources::Resources;
use crate::core::truck::Truck;

/// Which screen the web layer should draw next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Screen {
    /// Pick a profession (sets starting money and a perk).
    Profession,
    /// Name the four travellers.
    Party,
    /// Pick the month to set out.
    Season,
    /// Buy a truck.
    Outfit,
    /// Buy supplies.
    Store,
    /// Sell wild food and pawn gear.
    Trade,
    /// Look for odd jobs.
    Work,
    /// Driving along a chosen road.
    Travel,
    /// At a stop: pick the next road.
    RoadChoice,
    /// Just arrived somewhere.
    Town,
    /// A random event happened on the road.
    Event,
    /// Someone by the road wants a ride: yes or no.
    Hitchhiker,
    GameOver,
    Win,
}

impl Screen {
    /// The questions asked before the trip starts.
    pub fn is_setup(self) -> bool {
        matches!(
            self,
            Screen::Profession | Screen::Party | Screen::Season | Screen::Outfit
        )
    }

    /// The run has ended, one way or the other.
    pub fn is_over(self) -> bool {
        matches!(self, Screen::Win | Screen::GameOver)
    }
}

/// A leg of the current route, while `screen` is Travel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnRoute {
    /// Index into `MapGraph::roads`.
    pub road: usize,
    pub from: String,
    /// The node we are driving to.
    pub destination: String,
    /// Miles of this road still to drive, >= 0.
    pub remaining_miles: f64,
    /// Total mileage of this leg (for percentages).
    pub total_miles: f64,
}

/// A random event that has just been resolved, for display.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventReport {
    pub name: String,
    pub text: String,
    /// Human-readable consequences ("-$80", "lost 1 day").
    pub consequences: Vec<String>,
}

/// Ending stats for the Win / GameOver screens.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Score {
    pub party_survivors: usize,
    pub days_traveled: u32,
    pub money_left: f64,
    pub places_visited: usize,
    pub total: u64,
}

/// One day's line in the trip's record.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DaySample {
    pub day: u32,
    /// Miles driven that day.
    pub miles: f64,
    /// Cash and food as the day was last seen.
    pub money: f64,
    pub food: u32,
}

/// The whole serializable game. Anything in here is saved to localStorage;
/// derived or browser-only state does not live here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    /// Seed for the reproducible run; also the source of day-derivation.
    pub seed: u64,
    pub screen: Screen,
    pub party: Party,
    pub truck: Truck,
    pub resources: Resources,
    /// Index into the professions table.
    pub profession: usize,
    /// The profession's name, which finds it again if the table is reordered.
    #[serde(default)]
    pub job: String,
    /// The player character is a woman (picks the sprite and default name).
    #[serde(default)]
    pub female: bool,
    /// Mission steps completed so far.
    #[serde(default)]
    pub goal_step: usize,

    /// Node the party last arrived at.
    pub current_node: String,
    /// Set while driving.
    pub en_route: Option<EnRoute>,

    /// Month the trip set out, 1..=12.
    #[serde(default = "default_month")]
    pub start_month: u32,
    /// Day of the trip, starting at 1.
    pub day: u32,
    /// Index into `config::PACES`.
    pub pace_index: usize,
    /// Index into `config::MEALS`.
    pub meal_index: usize,

    /// Nodes visited (for score), in visit order.
    pub visited: Vec<String>,

    /// Odd jobs already done, as `gig@stop`: you can't go back.
    #[serde(default)]
    pub worked: Vec<String>,
    /// How each day went, for the graphs.
    #[serde(default)]
    pub history: Vec<DaySample>,

    /// What happened during the last action.
    pub log: Vec<String>,
    /// The event shown while `screen` is Event.
    pub event: Option<EventReport>,
    /// Where to go after the Event or Store screen.
    pub next_screen: Screen,

    /// Short human-readable reason text for Win / GameOver.
    pub ending_reason: String,
    pub score: Score,
}

impl GameState {
    /// The start of a run: nothing chosen yet, everyone at `start`.
    /// `vehicle` stands in until one is bought.
    pub fn new(seed: u64, start: &str, names: &[String], vehicle: &VehicleDef) -> Self {
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        GameState {
            seed,
            screen: Screen::Profession,
            party: Party::new(&names),
            truck: Truck::from_def(vehicle),
            resources: Resources::default(),
            profession: 0,
            job: String::new(),
            female: false,
            current_node: start.to_string(),
            en_route: None,
            start_month: DEFAULT_MONTH,
            day: 1,
            goal_step: 0,
            history: Vec::new(),
            pace_index: 1,
            meal_index: 1,
            visited: vec![start.to_string()],
            worked: Vec::new(),
            log: Vec::new(),
            event: None,
            next_screen: Screen::RoadChoice,
            ending_reason: String::new(),
            score: Score::default(),
        }
    }
}

fn default_month() -> u32 {
    crate::core::config::DEFAULT_MONTH
}

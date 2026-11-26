//! Serializable game state: the screen the player is on and everything the
//! rules need to keep going (party, truck, resources, calendar, route).

use serde::{Deserialize, Serialize};

use crate::core::party::{Party, PartyMember};
use crate::core::resources::Resources;
use crate::core::truck::Truck;

/// Which screen the web layer should draw next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Screen {
    Title,
    /// Outfitting in Cheyenne: pick truck (party created with 4 members).
    Outfit,
    /// Driving along a chosen road.
    Travel,
    /// At a node: pick the next road.
    RoadChoice,
    /// Arrived somewhere: services + history text.
    Town,
    /// A random event resolved. (M2)
    Event,
    /// An NPC interaction. (M2)
    Encounter,
    /// A road is closed for the day — reroute. (M3)
    RoadClosure,
    /// A hunt. (M3)
    Hunt,
    GameOver,
    Win,
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

/// Ending stats for the Win / GameOver screens.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Score {
    pub party_survivors: usize,
    pub days_traveled: u32,
    pub money_left: f64,
    pub landmarks_visited: usize,
    pub npcs_helped: usize,
}

/// The whole serializable game. Anything in here may be saved/restored
/// (localStorage in M5); derived or browser-only state does not live here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    /// Seed for the reproducible run; also the source of day-derivation.
    pub seed: u64,
    pub screen: Screen,
    pub party: Party,
    pub truck: Truck,
    pub resources: Resources,

    /// Node the party last arrived at.
    pub current_node: String,
    /// Set while `screen` is Travel.
    pub en_route: Option<EnRoute>,

    /// Day of the (simplified) month, 1..=30.
    pub day: u32,
    /// 1..=12.
    pub month: u32,
    /// Index into `config::PACES`.
    pub pace_index: usize,
    /// Index into `config::MEALS`.
    pub meal_index: usize,

    /// Nodes visited (for score + landmark counter), in visit order.
    pub visited: Vec<String>,

    /// Short human-readable reason text for Win / GameOver.
    pub ending_reason: String,
    pub score: Score,
}

impl GameState {
    /// The node the party is heading toward, if on the road.
    #[allow(dead_code)] // first consumer: M2 (HUD).
    pub fn destination(&self) -> Option<&str> {
        self.en_route.as_ref().map(|e| e.destination.as_str())
    }
}

/// Default party member names (4 members = driver + 3).
pub const DEFAULT_PARTY_NAMES: [&str; 4] = ["Driver", "Sam", "Alex", "Joy"];

/// Build one party member at full strength.
#[allow(dead_code)] // first consumer: M2 (party health reset).
pub fn new_member(name: &str) -> PartyMember {
    PartyMember {
        name: name.to_string(),
        health: 100.0,
        fatigue: 0.0,
        mood: 70.0,
        status: crate::core::party::MemberStatus::Healthy,
    }
}

//! Core gameplay rules. `Game` is shared read-only [`Content`] plus the run's
//! [`GameState`]; each module here is a set of rules that move that state on.
//! No I/O and no browser code, so everything is unit-testable on the host.

#[cfg(test)]
mod balance;
mod events;
mod history;
mod mission;
mod money;
mod road;
mod save;
mod setup;
mod shop;
#[cfg(test)]
mod stress;
mod supplies;
#[cfg(test)]
mod tests;
mod wild;

pub use mission::GoalState;

use std::rc::Rc;

use crate::core::config::*;
use crate::core::content::Content;
use crate::core::defs::{Effect, EventDef, ItemDef, ItemKind};
use crate::core::defs::{
    FieldDef, GatherMode, GigDef, GoalStep, NamePool, Perk, ProfessionDef, WeatherDef,
};
use crate::core::gather::Session;
use crate::core::map_graph::{miles_per_day, Node, Terrain};
use crate::core::party::MemberStatus;
use crate::core::rng::Rng;
use crate::core::state::{EnRoute, EventReport, GameState, Score, Screen};
use crate::core::truck::Truck;

/// Something that costs money, for confirming before it is bought.
#[derive(Debug, Clone, PartialEq)]
pub enum Purchase {
    /// Index into the vehicles table.
    Truck(usize),
    /// Store item id.
    Item(String),
    /// As much of a full tank as money allows.
    Fuel,
    Motel,
    /// Free nights in camp: no money, but days.
    Camp(u32),
    /// Sell this many servings of wild food.
    SellFood(u32),
    /// Pawn one purchase's worth of an item.
    SellItem(String),
    /// Take an odd job.
    Work(String),
}

/// What a purchase is and what it will cost.
#[derive(Debug, Clone, PartialEq)]
pub struct Quote {
    pub what: String,
    /// Cash out of pocket; negative when you are the one being paid.
    pub cost: f64,
    /// Days it takes.
    pub days: u32,
}

/// Four starting names, alternating men and women, the player's sex first.
fn party_names(names: &NamePool, seed: u64, female: bool) -> Vec<String> {
    // A fixed offset keeps this stream apart from the day-by-day ones.
    let mut rng = Rng::for_day(seed ^ 0x4E41, 0);
    let mut pools = [names.men.clone(), names.women.clone()];
    (0..4)
        .filter_map(|i| {
            let pool = &mut pools[(i + usize::from(female)) % 2];
            if pool.is_empty() {
                return None;
            }
            let at = (rng.unit() * pool.len() as f64) as usize;
            Some(pool.remove(at.min(pool.len() - 1)))
        })
        .collect()
}

/// Choose an entry in proportion to its weight.
fn weighted<'a>(pool: &[&'a EventDef], rng: &mut Rng) -> Option<&'a EventDef> {
    let total: u32 = pool.iter().map(|e| e.weight).sum();
    let mut pick = (rng.unit() * f64::from(total)) as u32;
    pool.iter().copied().find(|e| {
        if pick < e.weight {
            true
        } else {
            pick -= e.weight;
            false
        }
    })
}

/// Shared reference data plus the one thing that changes: the run's state.
/// The methods here are the rules that move the state along.
#[derive(Debug, Clone)]
pub struct Game {
    pub content: Rc<Content>,
    pub state: GameState,
}

impl Game {
    /// A fresh run that loads its own content, for tests.
    #[cfg(test)]
    pub fn new_run(seed: u64) -> Self {
        Self::new(Rc::new(Content::load()), seed)
    }

    /// A fresh run on content already loaded. `seed` makes the whole run
    /// reproducible.
    pub fn new(content: Rc<Content>, seed: u64) -> Self {
        let state = GameState::new(
            seed,
            &content.map.start,
            &party_names(&content.names.wyoming, seed, false),
            &content.vehicles[0],
        );
        Game { content, state }
    }

    /// Resume a saved run, or `None` if it no longer fits the bundled data or
    /// holds figures no game could reach.
    pub fn from_state(content: Rc<Content>, mut state: GameState) -> Option<Self> {
        let map = &content.map;
        if let Some(e) = state.en_route.as_mut() {
            // Road numbers shift when the map gains a road; the ends don't.
            let same = |i: usize| {
                map.road(i)
                    .is_some_and(|r| r.from == e.from && r.to == e.destination)
            };
            if !same(e.road) {
                e.road = (0..map.roads.len()).find(|&i| same(i))?;
            }
        }
        // So do job numbers when a job is added or dropped. A job that is
        // gone takes its saved runs with it.
        if state.screen != Screen::Profession {
            let jobs = &content.professions;
            state.profession = jobs.iter().position(|p| p.name == state.job)?;
        }
        save::is_sound(&content, &state).then_some(Game { content, state })
    }

    pub fn node(&self) -> Option<&Node> {
        self.content.map.node(&self.state.current_node)
    }

    pub(super) fn expect_screen(&self, screen: Screen) -> Result<(), String> {
        if self.state.screen == screen {
            Ok(())
        } else {
            Err("you can't do that right now".into())
        }
    }

    /// True while stopped at a node with the trip under way.
    pub(super) fn stopped(&self) -> bool {
        matches!(self.state.screen, Screen::Town | Screen::RoadChoice)
    }

    /// True while the trip is still going.
    pub(super) fn stopped_or_driving(&self) -> bool {
        self.stopped() || self.state.screen == Screen::Travel
    }
}

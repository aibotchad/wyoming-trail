//! The Wyoming road network: nodes, one-way-safe roads, node services.
//! Data comes from `assets/data/map.json`; the graph is pre-validated at load.

use serde::{Deserialize, Serialize};

use crate::core::config::BASE_MPH;

/// What a node is on the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    Start,
    Junction,
    Town,
    Landmark,
    ParkEntrance,
}

impl std::fmt::Display for NodeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            NodeKind::Start => "trailhead",
            NodeKind::Junction => "junction",
            NodeKind::Town => "town",
            NodeKind::Landmark => "landmark",
            NodeKind::ParkEntrance => "park entrance",
        };
        f.write_str(label)
    }
}

/// Services available at a node when stopped.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Services {
    pub fuel: bool,
    pub food: bool,
    pub motel: bool,
    pub mechanic: bool,
    pub outfitter: bool,
}

/// A place on the map (town, landmark, junction, park entrance).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub name: String,
    pub kind: NodeKind,
    pub services: Services,
    /// Short history / flavor text shown on the Town screen.
    pub history: String,
    pub hunting_allowed: bool,
}

/// Surface the road runs over; sets pace feel, speed factor and scenery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Terrain {
    HighPlains,
    SagebrushBasin,
    RiverValley,
    Badlands,
    MountainPass,
    GeyserBasin,
}

impl Terrain {
    /// Speed multiplier for this surface (see config::BASE_MPH).
    pub fn mph_factor(self) -> f64 {
        match self {
            Terrain::HighPlains | Terrain::SagebrushBasin => 1.0,
            Terrain::RiverValley => 0.95,
            Terrain::Badlands => 0.9,
            Terrain::GeyserBasin => 0.9,
            Terrain::MountainPass => 0.75,
        }
    }

    /// Short display name for menus.
    pub fn label(self) -> &'static str {
        match self {
            Terrain::HighPlains => "high plains",
            Terrain::SagebrushBasin => "sagebrush basin",
            Terrain::RiverValley => "river valley",
            Terrain::Badlands => "badlands",
            Terrain::MountainPass => "mountain pass",
            Terrain::GeyserBasin => "geyser basin",
        }
    }
}

/// A drivable road between two nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Road {
    pub from: String,
    pub to: String,
    pub highway: String,
    pub distance_miles: f64,
    pub terrain: Terrain,
    /// Long stretch without fuel / shops while driving it.
    pub services_gap: bool,
    /// 0-5: wind-blown / seasonal weather risk while driving.
    pub seasonal_risk: u8,
    /// 0-5: how scenic it is (affects mood).
    pub scenery: u8,
    /// Which random event table (by name, in events.json) applies on it.
    pub events_table: String,
    /// Which NPC table (by name, in npcs.json) may appear on it.
    pub npc_table: String,
    /// Road closed (seasonal) — hidden from choices until reopened (M3).
    pub closed: bool,
}

/// The full graph. Indices (`nodes`, `roads`) are stable for the life of the
/// game; lookups are O(1) via `node_index`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapGraph {
    pub nodes: Vec<Node>,
    pub roads: Vec<Road>,
    pub node_index: std::collections::HashMap<String, usize>,
    /// Node id the run begins at.
    pub start: String,
    /// Node ids that count as "reached Yellowstone" for the Win screen.
    pub goals: Vec<String>,
}

impl MapGraph {
    /// Load + validate the bundled map. All data is compile-time (include_str!),
    /// so failure here is a build/test error, not a gameplay state.
    pub fn load() -> Self {
        let raw: MapGraph = serde_json::from_str(crate::core::data::MAP_JSON)
            .expect("map.json: must be valid JSON at build time");
        raw.validate();
        raw
    }

    /// Fail (test-visible panic with a message) if the graph is inconsistent.
    fn validate(&self) {
        if self.start.is_empty() || self.goals.is_empty() {
            panic!("map.json: start and at least one goal are required");
        }
        self.node(&self.start)
            .expect("map.json: start node must exist");

        let mut seen = std::collections::HashSet::new();
        for (i, n) in self.nodes.iter().enumerate() {
            if !seen.insert(n.id.clone()) {
                panic!("map.json: duplicate node id {}", n.id);
            }
            // Keep index in sync so O(1) lookups can never lie.
            let existing = *self.node_index.get(&n.id).unwrap_or(&usize::MAX);
            assert_eq!(existing, i, "map.json: node_index out of sync for {}", n.id);
        }
        for r in &self.roads {
            self.node(&r.from)
                .expect("map.json: road `from` node missing");
            self.node(&r.to).expect("map.json: road `to` node missing");
            assert!(
                r.distance_miles > 0.0,
                "map.json: road {}→{} has non-positive distance",
                r.from,
                r.to
            );
        }
    }

    /// O(1) node lookup. Returns `Option` so callers can handle unknowns.
    pub fn node(&self, id: &str) -> Option<&Node> {
        self.node_index.get(id).and_then(|&i| self.nodes.get(i))
    }

    #[allow(dead_code)] // first consumer: M2+ (road state changes).
    pub fn node_mut(&mut self, id: &str) -> Option<&mut Node> {
        self.node_index.get(id).and_then(|&i| self.nodes.get_mut(i))
    }

    /// Open (not closed) roads leaving `node`, in map order — menu order.
    #[allow(dead_code)] // first consumer: M2 (road choice menus).
    pub fn outgoing(&self, node: &str) -> Vec<&Road> {
        self.roads
            .iter()
            .filter(|r| r.from == node && !r.closed)
            .collect()
    }

    /// Roads entering a node (M3 shop/encounter logic).
    #[allow(dead_code)] // first consumer: M3 (encounter logic).
    pub fn incoming(&self, node: &str) -> Vec<&Road> {
        self.roads.iter().filter(|r| r.to == node).collect()
    }

    pub fn road(&self, idx: usize) -> Option<&Road> {
        self.roads.get(idx)
    }

    pub fn is_goal(&self, node: &str) -> bool {
        self.goals.iter().any(|g| g == node)
    }

    /// Breadth-first search: is `goal` reachable from `start` over open roads?
    /// Used by tests and for the "can still reach Yellowstone" sanity check.
    #[allow(dead_code)] // exercised by tests; first gameplay consumer: M4 (closed roads).
    pub fn reachable(&self, start: &str, goal: &str) -> bool {
        if start == goal {
            return true;
        }
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(start.to_string());
        visited.insert(start.to_string());
        while let Some(cur) = queue.pop_front() {
            for r in self.outgoing(&cur) {
                if visited.insert(r.to.clone()) {
                    if r.to == goal {
                        return true;
                    }
                    queue.push_back(r.to.clone());
                }
            }
        }
        false
    }
}

/// Miles driven per day at `hours` pace on `terrain`.
pub fn miles_per_day(hours: f64, terrain: Terrain) -> f64 {
    hours * BASE_MPH * terrain.mph_factor()
}

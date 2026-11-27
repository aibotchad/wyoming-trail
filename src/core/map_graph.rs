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

/// One kind of service, as named in item and job data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Service {
    Fuel,
    Food,
    Motel,
    Mechanic,
    Outfitter,
}

impl Services {
    pub fn has(&self, service: Service) -> bool {
        match service {
            Service::Fuel => self.fuel,
            Service::Food => self.food,
            Service::Motel => self.motel,
            Service::Mechanic => self.mechanic,
            Service::Outfitter => self.outfitter,
        }
    }
}

/// A place on the map (town, landmark, junction, park entrance).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub name: String,
    /// Shorter name for the map, where the full one is too long.
    #[serde(default)]
    pub label: Option<String>,
    pub kind: NodeKind,
    pub services: Services,
    pub lat: f64,
    pub lon: f64,
    /// Price per gallon where `services.fuel` is set (a game value).
    pub fuel_price: f64,
    /// True history / geography, sourced from Wikipedia.
    pub fact: String,
    /// Invented joke about the stop. Never presented as fact.
    pub legend: String,
    pub hunting_allowed: bool,
}

impl Node {
    pub fn short_name(&self) -> &str {
        self.label.as_deref().unwrap_or(&self.name)
    }
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
    #[serde(default)]
    pub closed: bool,
    /// The country passed through, in order: each stretch's terrain and how
    /// far along the road (0..1) it reaches. `terrain` is what sets the pace.
    #[serde(default)]
    pub country: Vec<(Terrain, f64)>,
}

impl Road {
    /// What the land looks like `done` (0..1) of the way along.
    pub fn terrain_at(&self, done: f64) -> Terrain {
        let here = self.country.iter().find(|(_, to)| done < *to);
        here.or(self.country.last())
            .map_or(self.terrain, |&(t, _)| t)
    }

    /// The stretch coming up within `reach` of `done`, if the land is about to
    /// change, with how far into the change we are (0..1).
    pub fn changing_to(&self, done: f64, reach: f64) -> Option<(Terrain, f64)> {
        let next = self
            .country
            .windows(2)
            .find(|w| done < w[0].1 && done >= w[0].1 - reach)?;
        Some((next[1].0, 1.0 - (next[0].1 - done) / reach))
    }

    /// The same road driven the other way.
    fn reversed(&self) -> Road {
        let mut country = Vec::new();
        for (i, &(terrain, _)) in self.country.iter().enumerate().rev() {
            // A stretch now ends where, going out, it began.
            let began = if i == 0 { 0.0 } else { self.country[i - 1].1 };
            country.push((terrain, 1.0 - began));
        }
        Road {
            from: self.to.clone(),
            to: self.from.clone(),
            country,
            ..self.clone()
        }
    }
}

/// Geographic box covered by the bundled map image.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MapBounds {
    pub west: f64,
    pub east: f64,
    pub south: f64,
    pub north: f64,
}

impl MapBounds {
    /// Web Mercator position of a coordinate as fractions (0..1) of the map
    /// image, measured from its top-left corner.
    pub fn project(&self, lat: f64, lon: f64) -> (f64, f64) {
        let merc = |lat: f64| lat.to_radians().tan().asinh();
        let x = (lon - self.west) / (self.east - self.west);
        let y = (merc(self.north) - merc(lat)) / (merc(self.north) - merc(self.south));
        (x, y)
    }

    /// Image width divided by height.
    pub fn aspect(&self) -> f64 {
        let merc = |lat: f64| lat.to_radians().tan().asinh();
        (self.east - self.west).to_radians() / (merc(self.north) - merc(self.south))
    }
}

/// The full graph. Indices (`nodes`, `roads`) are stable for the life of the
/// game; lookups are O(1) via `node_index`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapGraph {
    pub nodes: Vec<Node>,
    /// Authored once per road; `load` adds the reverse direction.
    pub roads: Vec<Road>,
    #[serde(skip)]
    pub node_index: std::collections::HashMap<String, usize>,
    pub bounds: MapBounds,
    /// Node id the run begins at.
    pub start: String,
}

impl MapGraph {
    /// Load + validate the bundled map. All data is compile-time (include_str!),
    /// so failure here is a build/test error, not a gameplay state.
    pub fn load() -> Self {
        let mut map: MapGraph = serde_json::from_str(crate::core::data::MAP_JSON)
            .expect("map.json: must be valid JSON at build time");
        map.node_index = map
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id.clone(), i))
            .collect();
        let reverse: Vec<Road> = map.roads.iter().map(Road::reversed).collect();
        map.roads.extend(reverse);
        map.validate();
        map
    }

    /// Fail (test-visible panic with a message) if the graph is inconsistent.
    fn validate(&self) {
        self.node(&self.start)
            .expect("map.json: start node must exist");

        let mut seen = std::collections::HashSet::new();
        for n in &self.nodes {
            if !seen.insert(n.id.clone()) {
                panic!("map.json: duplicate node id {}", n.id);
            }
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

    /// Open (not closed) roads leaving `node`, in map order — menu order.
    pub fn outgoing(&self, node: &str) -> Vec<&Road> {
        self.roads
            .iter()
            .filter(|r| r.from == node && !r.closed)
            .collect()
    }

    pub fn road(&self, idx: usize) -> Option<&Road> {
        self.roads.get(idx)
    }

    /// Road indices of the shortest drive from `from` to the nearest of
    /// `targets`; empty if already there, `None` if there is no way.
    pub fn route(&self, from: &str, targets: &[String]) -> Option<Vec<usize>> {
        // Dijkstra; the map is small enough to scan for the next-closest node.
        // Each entry is (miles from the start, road arrived by).
        let mut best = std::collections::BTreeMap::from([(from, (0.0_f64, None::<usize>))]);
        let mut done = std::collections::HashSet::new();
        loop {
            let (mut cur, miles) = best
                .iter()
                .filter(|(n, _)| !done.contains(*n))
                .map(|(&n, &(d, _))| (n, d))
                .min_by(|a, b| a.1.total_cmp(&b.1))?;
            if targets.iter().any(|t| t == cur) {
                let mut path = Vec::new();
                while let Some(&(_, Some(i))) = best.get(cur) {
                    path.push(i);
                    cur = &self.roads[i].from;
                }
                path.reverse();
                return Some(path);
            }
            done.insert(cur);
            for (i, r) in self.roads.iter().enumerate() {
                if r.from != cur || r.closed {
                    continue;
                }
                let total = miles + r.distance_miles;
                let entry = best.entry(r.to.as_str()).or_insert((total, Some(i)));
                if total < entry.0 {
                    *entry = (total, Some(i));
                }
            }
        }
    }

    /// Length of [`MapGraph::route`].
    pub fn miles_to(&self, from: &str, targets: &[String]) -> Option<f64> {
        let path = self.route(from, targets)?;
        Some(path.iter().map(|&i| self.roads[i].distance_miles).sum())
    }

    /// Breadth-first search: is `goal` reachable from `start` over open roads?
    /// Used by tests and for the "can still reach Yellowstone" sanity check.
    #[cfg(test)]
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

/// The point a fraction `t` (0..1) of the way along a polyline, and the index
/// of the segment's end point.
pub fn point_along(path: &[[f64; 2]], t: f64) -> Option<([f64; 2], usize)> {
    let length = |w: &[[f64; 2]]| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]);
    let total: f64 = path.windows(2).map(length).sum();
    let mut left = total * t.clamp(0.0, 1.0);
    for (i, w) in path.windows(2).enumerate() {
        let seg = length(w);
        if left <= seg && seg > 0.0 {
            let k = left / seg;
            let at = [
                w[0][0] + (w[1][0] - w[0][0]) * k,
                w[0][1] + (w[1][1] - w[0][1]) * k,
            ];
            return Some((at, i + 1));
        }
        left -= seg;
    }
    path.last().map(|&p| (p, path.len() - 1))
}

/// Miles driven per day at `hours` pace on `terrain`.
pub fn miles_per_day(hours: f64, terrain: Terrain) -> f64 {
    hours * BASE_MPH * terrain.mph_factor()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_stop_is_connected_both_ways() {
        let map = MapGraph::load();
        for n in &map.nodes {
            assert!(map.reachable(&n.id, &map.start), "{} is a trap", n.id);
            assert!(map.reachable(&map.start, &n.id), "{} is cut off", n.id);
        }
    }

    #[test]
    fn every_road_describes_its_country_the_same_both_ways() {
        let map = MapGraph::load();
        for r in &map.roads {
            let name = format!("{} to {}", r.from, r.to);
            assert!(!r.country.is_empty(), "{name}");
            assert!(r.country.windows(2).all(|w| w[0].1 < w[1].1), "{name}");
            assert!((r.country.last().unwrap().1 - 1.0).abs() < 1e-9, "{name}");
            let back = map
                .roads
                .iter()
                .find(|b| b.from == r.to && b.to == r.from)
                .unwrap();
            for i in 0..50 {
                let at = (f64::from(i) + 0.5) / 50.0 + 0.0013;
                assert_eq!(r.terrain_at(at), back.terrain_at(1.0 - at), "{name} {at}");
            }
        }
        let road = |from: &str, to: &str| {
            map.roads
                .iter()
                .find(|r| r.from == from && r.to == to)
                .unwrap()
        };
        // Sage all the way to Riverton; sage, then the pass, to South Pass City.
        let riverton = road("lander", "riverton");
        assert_eq!(riverton.terrain_at(0.5), Terrain::SagebrushBasin);
        let pass = road("lander", "south_pass_city");
        assert_eq!(pass.terrain_at(0.1), Terrain::SagebrushBasin);
        assert_eq!(pass.terrain_at(0.5), Terrain::MountainPass);
        assert!(pass.changing_to(0.2, 0.1).is_none());
        let (coming, share) = pass.changing_to(0.3, 0.1).unwrap();
        assert_eq!(coming, Terrain::MountainPass);
        assert!((share - 0.5).abs() < 1e-9);
    }

    #[test]
    fn route_takes_the_shortest_way() {
        let map = MapGraph::load();
        let park = [
            "yellowstone_south".to_string(),
            "yellowstone_east".to_string(),
        ];
        // Over Togwotee Pass: 75 + 59 + 23.
        assert_eq!(map.miles_to("lander", &park), Some(157.0));
        assert_eq!(map.route("yellowstone_south", &park), Some(vec![]));
        assert_eq!(map.route("atlantis", &park), None);
        let path = map.route("lander", &park).unwrap();
        assert_eq!(map.roads[path[0]].to, "dubois");
        assert_eq!(map.roads[path[2]].to, "yellowstone_south");
    }

    #[test]
    fn point_along_measures_by_length() {
        let path = [[0.0, 0.0], [1.0, 0.0], [1.0, 3.0]];
        assert_eq!(point_along(&path, 0.0), Some(([0.0, 0.0], 1)));
        assert_eq!(point_along(&path, 0.5), Some(([1.0, 1.0], 2)));
        assert_eq!(point_along(&path, 1.0), Some(([1.0, 3.0], 2)));
        assert_eq!(point_along(&[], 0.5), None);
    }

    #[test]
    fn every_stop_lands_inside_the_map_image() {
        let map = MapGraph::load();
        for n in &map.nodes {
            let (x, y) = map.bounds.project(n.lat, n.lon);
            assert!(
                (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y),
                "{}",
                n.id
            );
        }
    }

    #[test]
    fn every_vehicle_can_cover_any_single_road() {
        let map = MapGraph::load();
        for t in crate::core::data::load_vehicles() {
            for r in &map.roads {
                assert!(
                    r.distance_miles <= t.tank_gallons * t.mpg,
                    "{}: {} {}",
                    t.name,
                    r.from,
                    r.to
                );
            }
        }
    }
}

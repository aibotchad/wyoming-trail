//! Bundled (compile-time embedded) game content.
//!
//! `include_str!` keeps the game fully offline at runtime — no fetch, no network.
//! The raw JSON constants are the single source of truth for content; the
//! `load_*` functions parse them once into the typed tables in `crate::core::defs`.
//!
//! Data-oriented layout: content is plain data (see `defs`), parsed into
//! deterministic tables keyed by id/terrain. Rules modules operate on those
//! tables — they never parse JSON themselves.
//!
//! No web-sys / js-sys / wasm-bindgen: this must compile and test on a native
//! target as well as wasm32.

use std::collections::BTreeMap;

use crate::core::defs::{
    must_parse, CreditDef, EventDef, EventTables, FieldDef, GatherDef, GigDef, ItemIndex, LookDef,
    NameBook, ProfessionDef, RideDef, SceneryBook, SpriteDef, VehicleDef, WardenDef, WeatherDef,
};

/// map.json — the Wyoming road graph (consumed by `map_graph::MapGraph`).
pub const MAP_JSON: &str = include_str!("../../assets/data/map.json");

/// events.json — random-event tables keyed by terrain id.
pub const EVENTS_JSON: &str = include_str!("../../assets/data/events.json");

/// items.json — purchasable / carryable items keyed by id.
pub const ITEMS_JSON: &str = include_str!("../../assets/data/items.json");

#[cfg(test)]
/// npcs.json — encounter definitions keyed by terrain id.
pub const NPCS_JSON: &str = include_str!("../../assets/data/npcs.json");

#[cfg(test)]
/// dialogue.json — dialogue lines keyed by node / npc / special key.
pub const DIALOGUE_JSON: &str = include_str!("../../assets/data/dialogue.json");

/// professions.json — starting professions, in menu order.
pub const PROFESSIONS_JSON: &str = include_str!("../../assets/data/professions.json");

/// credits.json — photo attribution.
pub const CREDITS_JSON: &str = include_str!("../../assets/data/credits.json");

/// road_paths.json — `"from>to": [[lon, lat], …]`, for drawing roads.
pub const ROAD_PATHS_JSON: &str = include_str!("../../assets/data/road_paths.json");

/// names.json — default party names.
pub const NAMES_JSON: &str = include_str!("../../assets/data/names.json");

pub fn load_names() -> NameBook {
    must_parse(NAMES_JSON, "names.json")
}

/// fields.json — photo backdrops for gathering.
pub const FIELDS_JSON: &str = include_str!("../../assets/data/fields.json");

/// weather.json — kinds of weather and when they happen.
pub const WEATHER_JSON: &str = include_str!("../../assets/data/weather.json");

/// hitchhikers.json — what can come of giving someone a ride.
pub const HITCHHIKERS_JSON: &str = include_str!("../../assets/data/hitchhikers.json");

pub fn load_fields() -> Vec<FieldDef> {
    must_parse(FIELDS_JSON, "fields.json")
}

pub fn load_weather() -> Vec<WeatherDef> {
    must_parse(WEATHER_JSON, "weather.json")
}

pub fn load_hitchhikers() -> Vec<EventDef> {
    must_parse(HITCHHIKERS_JSON, "hitchhikers.json")
}

/// gigs.json — odd jobs.
pub const GIGS_JSON: &str = include_str!("../../assets/data/gigs.json");

pub fn load_gigs() -> Vec<GigDef> {
    must_parse(GIGS_JSON, "gigs.json")
}

/// vehicles.json — what can be bought at the start, in menu order.
pub const VEHICLES_JSON: &str = include_str!("../../assets/data/vehicles.json");

pub fn load_vehicles() -> Vec<VehicleDef> {
    must_parse(VEHICLES_JSON, "vehicles.json")
}

/// looks.json — how each character strip is animated.
pub const LOOKS_JSON: &str = include_str!("../../assets/data/looks.json");

/// scenery.json — the drive view for each terrain.
pub const SCENERY_JSON: &str = include_str!("../../assets/data/scenery.json");

pub fn load_looks() -> Vec<LookDef> {
    must_parse(LOOKS_JSON, "looks.json")
}

pub fn load_scenery() -> SceneryBook {
    must_parse(SCENERY_JSON, "scenery.json")
}

/// gather.json — animals to poach and plants to forage.
pub const GATHER_JSON: &str = include_str!("../../assets/data/gather.json");

pub fn load_gather() -> Vec<GatherDef> {
    must_parse(GATHER_JSON, "gather.json")
}

/// rides.json — words and costs for each way of travelling.
pub const RIDES_JSON: &str = include_str!("../../assets/data/rides.json");

pub fn load_rides() -> Vec<RideDef> {
    must_parse(RIDES_JSON, "rides.json")
}

/// wardens.json — what happens to a caught poacher.
pub const WARDENS_JSON: &str = include_str!("../../assets/data/wardens.json");

pub fn load_wardens() -> Vec<WardenDef> {
    must_parse(WARDENS_JSON, "wardens.json")
}

/// sprites.json — where each animation sits in `img/sprites.png`.
pub const SPRITES_JSON: &str = include_str!("../../assets/data/sprites.json");

pub fn load_sprites() -> BTreeMap<String, SpriteDef> {
    must_parse(SPRITES_JSON, "sprites.json")
}

pub fn load_professions() -> Vec<ProfessionDef> {
    must_parse(PROFESSIONS_JSON, "professions.json")
}

pub fn load_credits() -> Vec<CreditDef> {
    must_parse(CREDITS_JSON, "credits.json")
}

pub fn load_road_paths() -> BTreeMap<String, Vec<[f64; 2]>> {
    must_parse(ROAD_PATHS_JSON, "road_paths.json")
}

/// Parse `events.json` into the typed event tables (`{ terrain: [EventDef] }`).
pub fn load_events() -> EventTables {
    must_parse(EVENTS_JSON, "events.json")
}

/// Parse `items.json` into the typed item catalog (`{ id: ItemDef }`).
pub fn load_items() -> ItemIndex {
    must_parse(ITEMS_JSON, "items.json")
}

#[cfg(test)]
/// Parse `npcs.json` into the typed npc tables (`{ terrain: [NpcDef] }`).
pub fn load_npcs() -> crate::core::defs::NpcTables {
    must_parse(NPCS_JSON, "npcs.json")
}

#[cfg(test)]
/// Parse `dialogue.json` into the typed dialogue book (`{ key: [line] }`).
pub fn load_dialogue() -> crate::core::defs::DialogueBook {
    must_parse(DIALOGUE_JSON, "dialogue.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::defs::ItemKind;
    use std::collections::BTreeSet;

    #[test]
    fn all_data_files_parse_to_typed_tables() {
        let events = load_events();
        assert!(
            !events.is_empty(),
            "events.json must define at least one event"
        );
        assert!(events.events("highway").is_some_and(|t| !t.is_empty()));

        let items = load_items();
        assert!(items.get("groceries").is_some(), "store must sell food");
        for kind in [ItemKind::Food, ItemKind::FuelCan, ItemKind::Ammo] {
            let found = items
                .ids()
                .any(|i| items.get(i).is_some_and(|d| d.kind == kind));
            assert!(found, "store must sell {kind:?}");
        }

        let npcs = load_npcs();
        assert!(npcs.npcs("highway").is_some_and(|t| !t.is_empty()));

        let dlg = load_dialogue();
        assert!(dlg.lines("title").is_some_and(|l| !l.is_empty()));
        assert!(dlg
            .lines("gas_station_attendant")
            .is_some_and(|l| !l.is_empty()));
    }

    #[test]
    fn events_only_ask_for_items_the_store_sells() {
        let (events, items) = (load_events(), load_items());
        for table in events.table_names() {
            for e in events.events(table).into_iter().flatten() {
                if let Some(id) = &e.needs {
                    assert!(items.get(id).is_some(), "{}: unknown item {id}", e.name);
                    assert!(e.saved_text.is_some(), "{}: no saved_text", e.name);
                }
            }
        }
    }

    #[test]
    fn every_terrain_has_a_backdrop_and_every_picture_is_credited() {
        use crate::core::map_graph::Terrain::*;
        let fields = load_fields();
        for t in [
            HighPlains,
            SagebrushBasin,
            RiverValley,
            Badlands,
            MountainPass,
            GeyserBasin,
        ] {
            assert!(fields.iter().any(|f| f.terrains.contains(&t)), "{t:?}");
        }
        let img = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/img");
        let credits = load_credits();
        let plants = load_gather().into_iter().filter_map(|g| g.photo);
        for id in fields.into_iter().map(|f| f.id).chain(plants) {
            assert!(img.join(format!("{id}.jpg")).is_file(), "{id}");
            assert!(credits.iter().any(|c| c.id == id), "{id}");
        }
    }

    #[test]
    fn gigs_point_at_real_stops_and_cost_time() {
        let map = crate::core::map_graph::MapGraph::load();
        for g in load_gigs() {
            assert!(g.days > 0 && g.pay > 0.0, "{}", g.id);
            assert!(g.at.iter().all(|id| map.node(id).is_some()), "{}", g.id);
        }
    }

    #[test]
    fn every_month_has_some_weather() {
        let weather = load_weather();
        for month in 0..12 {
            assert!(weather.iter().any(|w| w.months[month] > 0), "{month}");
        }
        assert!(!load_hitchhikers().is_empty());
    }

    #[test]
    fn every_mission_points_at_real_stops() {
        let map = crate::core::map_graph::MapGraph::load();
        for p in load_professions() {
            assert!(!p.goal.steps.is_empty(), "{}", p.name);
            for step in &p.goal.steps {
                assert!(!step.at.is_empty(), "{}", p.name);
                for id in &step.at {
                    assert!(map.node(id).is_some(), "{}: {id}", p.name);
                }
            }
        }
    }

    #[test]
    fn every_stop_has_a_credited_photo_and_road_paths_exist() {
        let map = crate::core::map_graph::MapGraph::load();
        let credits = load_credits();
        let img = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/img");
        assert!(img.join("map.jpg").is_file());
        for n in &map.nodes {
            assert!(img.join(format!("{}.jpg", n.id)).is_file(), "{}", n.id);
            assert!(credits.iter().any(|c| c.id == n.id), "{}", n.id);
        }
        let paths = load_road_paths();
        for r in &map.roads {
            let there = format!("{}>{}", r.from, r.to);
            let back = format!("{}>{}", r.to, r.from);
            assert!(
                paths.contains_key(&there) || paths.contains_key(&back),
                "{there}"
            );
        }
    }

    #[test]
    fn sprite_atlas_fits_the_sheet_and_covers_every_vehicle() {
        let png = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/img/sprites.png"),
        )
        .unwrap();
        // Width and height live at fixed offsets in the PNG header.
        let dim = |at: usize| u32::from_be_bytes(png[at..at + 4].try_into().unwrap());
        let (width, height) = (dim(16), dim(20));
        let atlas = load_sprites();
        for (name, s) in &atlas {
            assert!(s.frames > 0, "{name}");
            assert!(
                s.x + s.w * s.frames <= width && s.y + s.h <= height,
                "{name}"
            );
        }
        for (name, s) in &atlas {
            let [x, _, w, _] = s.art;
            assert!(x + w <= s.w && s.feet() <= s.h, "{name}");
        }
        for t in load_vehicles() {
            let s = atlas.get(&t.sprite).expect("every vehicle has art");
            // Tyres on the cell's bottom edge, so none of them floats.
            assert_eq!(s.feet(), s.h, "{}", t.name);
        }
        for g in load_gather() {
            assert!(atlas.contains_key(&g.sprite), "{}", g.name);
            let flee = g.flee.as_ref().unwrap_or(&g.sprite);
            assert!(atlas.contains_key(flee), "{}", g.name);
        }
        for name in [
            "dust",
            "sage",
            "pine",
            "sheep",
            "hiker",
            "dumpster_full",
            "dumpster_empty",
        ] {
            assert!(atlas.contains_key(name), "{name}");
        }
        for p in load_professions() {
            for female in [false, true] {
                for way in ["up", "right", "down", "left"] {
                    let name = p.hero_sprite(female, way);
                    assert!(atlas.contains_key(&name), "{name}");
                }
            }
        }
    }

    #[test]
    fn npc_tables_track_event_tables() {
        // A terrain is a drivable road state only if it has BOTH an event
        // table and an npc table; the two must stay in lockstep so M2 has a
        // full, consistent encounter set per terrain.
        let events = load_events();
        let npcs = load_npcs();
        let e: BTreeSet<&str> = events.table_names().collect();
        let n: BTreeSet<&str> = npcs.table_names().collect();
        assert_eq!(e, n, "every terrain needs both event and npc tables");
    }
}

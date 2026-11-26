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

// Foundation layer: consumed by M2 (event selection), not yet by game logic —
// silence dead-code lints here, not globally.
#![allow(dead_code)]

use crate::core::defs::{must_parse, DialogueBook, EventTables, ItemIndex, NpcTables};

/// map.json — the Wyoming road graph (consumed by `map_graph::MapGraph`).
pub const MAP_JSON: &str = include_str!("../../assets/data/map.json");

/// events.json — random-event tables keyed by terrain id.
pub const EVENTS_JSON: &str = include_str!("../../assets/data/events.json");

/// items.json — purchasable / carryable items keyed by id.
pub const ITEMS_JSON: &str = include_str!("../../assets/data/items.json");

/// npcs.json — encounter definitions keyed by terrain id.
pub const NPCS_JSON: &str = include_str!("../../assets/data/npcs.json");

/// dialogue.json — dialogue lines keyed by node / npc / special key.
pub const DIALOGUE_JSON: &str = include_str!("../../assets/data/dialogue.json");

/// Parse `events.json` into the typed event tables (`{ terrain: [EventDef] }`).
pub fn load_events() -> EventTables {
    must_parse(EVENTS_JSON, "events.json")
}

/// Parse `items.json` into the typed item catalog (`{ id: ItemDef }`).
pub fn load_items() -> ItemIndex {
    must_parse(ITEMS_JSON, "items.json")
}

/// Parse `npcs.json` into the typed npc tables (`{ terrain: [NpcDef] }`).
pub fn load_npcs() -> NpcTables {
    must_parse(NPCS_JSON, "npcs.json")
}

/// Parse `dialogue.json` into the typed dialogue book (`{ key: [line] }`).
pub fn load_dialogue() -> DialogueBook {
    must_parse(DIALOGUE_JSON, "dialogue.json")
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert!(items.get("fuel").is_some(), "items.json must define 'fuel'");
        assert!(items.get("food").is_some(), "items.json must define 'food'");
        // every item keeps an id-stable identity and a known kind
        assert!(items.get("fuel").unwrap().name.starts_with("Full tank"));

        let npcs = load_npcs();
        assert!(npcs.npcs("highway").is_some_and(|t| !t.is_empty()));

        let dlg = load_dialogue();
        assert!(dlg.lines("title").is_some_and(|l| !l.is_empty()));
        assert!(dlg
            .lines("gas_station_attendant")
            .is_some_and(|l| !l.is_empty()));
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

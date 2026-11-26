//! Bundled (compile-time embedded) game data. `include_str!` keeps the game
//! fully offline at runtime — no fetch, no network.

/// map.json — the Wyoming road graph.
pub const MAP_JSON: &str = include_str!("../../assets/data/map.json");

/// events.json — random event tables keyed by name (see Road::events_table).
#[allow(dead_code)] // first consumer: M2 (random events).
pub const EVENTS_JSON: &str = include_str!("../../assets/data/events.json");

/// items.json — purchasable / carryable items.
#[allow(dead_code)] // first consumer: M3 (shops).
pub const ITEMS_JSON: &str = include_str!("../../assets/data/items.json");

/// npcs.json — encounter definitions keyed by name (see Road::npc_table).
#[allow(dead_code)] // first consumer: M2 (NPC encounters).
pub const NPCS_JSON: &str = include_str!("../../assets/data/npcs.json");

/// dialogue.json — dialogue trees for NPC encounters.
#[allow(dead_code)] // first consumer: M2 (dialogue).
pub const DIALOGUE_JSON: &str = include_str!("../../assets/data/dialogue.json");

/// Parse events.json as { table: [ {name, ...} ] }.
#[allow(dead_code)] // first consumer: M2 (random events).
pub fn load_events() -> std::collections::HashMap<String, Vec<serde_json::Value>> {
    let v: serde_json::Value =
        serde_json::from_str(EVENTS_JSON).expect("events.json: must be valid JSON at build time");
    let obj = v
        .as_object()
        .expect("events.json: top level must be an object");
    obj.iter()
        .map(|(k, v)| {
            let list = v
                .as_array()
                .unwrap_or_else(|| panic!("events.json: table {k} must be an array"));
            (k.clone(), list.clone())
        })
        .collect()
}

/// Parse items.json as { id: { ... } }.
#[allow(dead_code)] // first consumer: M3 (shops).
pub fn load_items() -> std::collections::HashMap<String, serde_json::Value> {
    let v: serde_json::Value =
        serde_json::from_str(ITEMS_JSON).expect("items.json: must be valid JSON at build time");
    v.as_object()
        .expect("items.json: top level must be an object")
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// Parse npcs.json as { table: [ {...} ] }.
#[allow(dead_code)] // first consumer: M2 (NPC encounters).
pub fn load_npcs() -> std::collections::HashMap<String, Vec<serde_json::Value>> {
    let v: serde_json::Value =
        serde_json::from_str(NPCS_JSON).expect("npcs.json: must be valid JSON at build time");
    let obj = v
        .as_object()
        .expect("npcs.json: top level must be an object");
    obj.iter()
        .map(|(k, v)| {
            let list = v
                .as_array()
                .unwrap_or_else(|| panic!("npcs.json: table {k} must be an array"));
            (k.clone(), list.clone())
        })
        .collect()
}

/// Parse dialogue.json as { node_or_npc: [lines] }.
#[allow(dead_code)] // first consumer: M2 (dialogue).
pub fn load_dialogue() -> std::collections::HashMap<String, Vec<String>> {
    let v: serde_json::Value = serde_json::from_str(DIALOGUE_JSON)
        .expect("dialogue.json: must be valid JSON at build time");
    let out = v
        .as_object()
        .expect("dialogue.json: top level must be an object")
        .iter()
        .map(|(k, v)| {
            let lines = v
                .as_array()
                .unwrap_or_else(|| panic!("dialogue.json: entry {k} must be an array of strings"));
            (
                k.clone(),
                lines
                    .iter()
                    .map(|l| {
                        l.as_str()
                            .expect("dialogue.json: lines must be strings")
                            .to_string()
                    })
                    .collect(),
            )
        })
        .collect();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_data_files_parse() {
        // Empty tables are fine for M1 — the files must still be valid JSON
        // objects so M2/M3 rules have a working loader.
        let events = load_events();
        for t in events.values() {
            for e in t {
                e.as_object().expect("events must be objects");
            }
        }
        assert!(!load_items().is_empty());
        assert!(load_npcs().values().all(|t| !t.is_empty() || t.is_empty()));
        assert!(!load_dialogue().is_empty());
    }
}

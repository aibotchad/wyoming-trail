//! Typed content definitions for the JSON data files (events, npcs, dialogue,
//! items).
//!
//! DOP note: these are plain structs/enums laid out so a whole table or
//! collection is one contiguous block — no per-entity behavior, no trait
//! objects. Logic modules (M2 event selection, M3 shop/hunter) operate on
//! slices of these.
//!
//! Enums carry a `#[serde(other)]` fallback so a data file can introduce a new
//! role/kind without breaking an older build (extensibility without a schema
//! migration).

// Foundation layer: nothing outside tests consumes these types yet — M2
// (event selection) and M3 (shop/hunter) will. Silence dead-code lints here,
// not globally.
#![allow(dead_code)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Parse a bundled JSON string into a typed table.
///
/// Panics with a message that names the file — bundled content ships in the
/// repo, so a parse failure is a build/test-time bug, not a runtime input.
pub fn must_parse<T: serde::de::DeserializeOwned>(json: &str, what: &str) -> T {
    serde_json::from_str(json).unwrap_or_else(|e| panic!("{what}: invalid data — {e}"))
}

// ---------- events (terrain tables) ----------------------------------------

/// One encounter, as authored in `assets/data/events.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventDef {
    pub name: String,
    /// Family-friendly narrative shown to the player.
    pub text: String,
    /// Relative selection weight within the table (0 = never picked).
    pub weight: u32,
}

/// All event tables, keyed by table name (terrain id).
///
/// `#[serde(transparent)]` keeps the on-disk shape
/// `{ "table_name": [ { ...event... }, ... ], ... }` intact.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventTables(BTreeMap<String, Vec<EventDef>>);

impl EventTables {
    /// The events for one table, in authored order.
    pub fn events(&self, table: &str) -> Option<&Vec<EventDef>> {
        self.0.get(table)
    }

    /// Every table name.
    pub fn table_names(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }

    pub fn is_empty(&self) -> bool {
        self.0.values().all(Vec::is_empty)
    }

    pub fn len(&self) -> usize {
        self.0.values().map(Vec::len).sum()
    }
}

// ---------- npcs (terrain tables) -------------------------------------------

/// What an npc does in the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NpcRole {
    /// Sells services (fuel, food, repairs…).
    Service,
    /// Gives information or encouragement, sells nothing.
    Helper,
    /// Any role this build predates — kept as data rather than an error.
    #[serde(other)]
    Other,
}

/// One npc, as authored in `assets/data/npcs.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NpcDef {
    pub name: String,
    pub role: NpcRole,
    pub desc: String,
}

/// All npc tables, keyed by table name (terrain id).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NpcTables(BTreeMap<String, Vec<NpcDef>>);

impl NpcTables {
    pub fn npcs(&self, table: &str) -> Option<&Vec<NpcDef>> {
        self.0.get(table)
    }

    pub fn table_names(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }

    pub fn is_empty(&self) -> bool {
        self.0.values().all(Vec::is_empty)
    }
}

// ---------- dialogue (keyed lines) ------------------------------------------

/// All dialogue, keyed by node id, npc id, or a special key (e.g. "title").
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DialogueBook(BTreeMap<String, Vec<String>>);

impl DialogueBook {
    pub fn lines(&self, key: &str) -> Option<&Vec<String>> {
        self.0.get(key)
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

// ---------- items (catalog by id) -------------------------------------------

/// An item's category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    Consumable,
    Medicine,
    Equipment,
    Hunting,
    /// Any kind this build predates — kept as data rather than an error.
    #[serde(other)]
    #[default]
    Other,
}

/// One catalog entry, as authored in `assets/data/items.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemDef {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub price: f64,
    #[serde(default)]
    pub kind: ItemKind,
    #[serde(default)]
    pub desc: String,
    /// Unit count for multi-use items (doses, rounds…), if any.
    #[serde(default)]
    pub count: Option<u32>,
}

/// The item catalog, keyed by id ("fuel", "food", …).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ItemIndex(BTreeMap<String, ItemDef>);

impl ItemIndex {
    pub fn get(&self, id: &str) -> Option<&ItemDef> {
        self.0.get(id)
    }

    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_def_round_trips() {
        let s = r#"{ "name": "Wind", "text": "Crosswinds.", "weight": 2 }"#;
        let e: EventDef = serde_json::from_str(s).unwrap();
        assert_eq!(e.weight, 2);
        assert_eq!(
            serde_json::to_string(&e).unwrap(),
            r#"{"name":"Wind","text":"Crosswinds.","weight":2}"#
        );
    }

    #[test]
    fn unknown_role_becomes_other_instead_of_error() {
        let n: NpcDef =
            serde_json::from_str(r#"{ "name": "X", "role": "bargainer", "desc": "d" }"#).unwrap();
        assert_eq!(n.role, NpcRole::Other);
        let i: ItemDef =
            serde_json::from_str(r#"{ "name": "Y", "price": 1.0, "kind": "craft", "desc": "d" }"#)
                .unwrap();
        assert_eq!(i.kind, ItemKind::Other);
    }

    #[test]
    fn default_kind_is_other() {
        let i: ItemDef = serde_json::from_str(r#"{ "name": "Z", "price": 1.0 }"#).unwrap();
        assert_eq!(i.kind, ItemKind::Other);
        assert_eq!(i.count, None);
    }
}

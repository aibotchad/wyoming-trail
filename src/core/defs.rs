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

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::core::map_graph::{Service, Terrain};

/// Parse a bundled JSON string into a typed table.
///
/// Panics with a message that names the file — bundled content ships in the
/// repo, so a parse failure is a build/test-time bug, not a runtime input.
pub fn must_parse<T: serde::de::DeserializeOwned>(json: &str, what: &str) -> T {
    serde_json::from_str(json).unwrap_or_else(|e| panic!("{what}: invalid data — {e}"))
}

// ---------- events (terrain tables) ----------------------------------------

/// What an event does to the run. Positive numbers add, negative remove.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Effect {
    pub money: f64,
    /// Food servings.
    pub food: i32,
    /// Gallons in the tank.
    pub fuel: f64,
    pub health: f64,
    pub mood: f64,
    /// Whole days lost.
    pub days: u32,
    /// Road miles added (a wrong turn) or saved on the current leg.
    pub miles: f64,
}

/// One encounter, as authored in `assets/data/events.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventDef {
    pub name: String,
    /// What happens. `{name}` is replaced with a random party member.
    pub text: String,
    /// Relative selection weight within the table (0 = never picked).
    pub weight: u32,
    #[serde(default)]
    pub effect: Effect,
    /// Health changes hit only the named member rather than everyone.
    #[serde(default)]
    pub one: bool,
    /// Item id that turns this into the `saved_*` outcome.
    #[serde(default)]
    pub needs: Option<String>,
    #[serde(default)]
    pub saved_text: Option<String>,
    #[serde(default)]
    pub saved_effect: Effect,
    /// Only happens to those travelling this way.
    #[serde(default)]
    pub ride: Option<Ride>,
}

/// A way of getting about.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ride {
    #[default]
    Motor,
    Horse,
}

/// What one way of travelling is called and what it costs (`rides.json`).
/// Texts may hold `{cost}`, `{amount}`, `{miles}` and `{place}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RideDef {
    pub ride: Ride,
    /// "Drive" / "Ride", and its past and "-ing" forms.
    pub verb: String,
    pub past: String,
    pub going: String,
    /// What the tank holds, and what it is measured in.
    pub fuel: String,
    pub unit: String,
    /// One carried refill.
    pub spare: String,
    pub fill: String,
    pub full: String,
    pub full_why: String,
    pub bought: String,
    pub refill: String,
    pub gauge: String,
    pub full_need: String,
    pub camp: String,
    pub camp_ask: String,
    pub camp_night: String,
    pub camp_log: String,
    pub back: String,
    pub dry_warning: String,
    pub rescue: String,
    pub stranded: String,
    /// Price of being rescued, and how much fuel that brings.
    pub rescue_cost: f64,
    pub rescue_amount: f64,
    /// Fuel price relative to the pump price.
    pub price: f64,
    /// Fuel found free overnight when plants are growing.
    pub graze: f64,
}

/// One thing a game warden can do to a poacher (`wardens.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WardenDef {
    pub name: String,
    pub weight: u32,
    pub fine: f64,
    pub text: String,
    /// The ammunition goes too.
    #[serde(default)]
    pub ammo: bool,
    /// Days lost.
    #[serde(default)]
    pub days: u32,
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

    /// Every event on any road.
    pub fn all(&self) -> impl Iterator<Item = &EventDef> {
        self.0.values().flatten()
    }

    /// Every table name.
    #[cfg(test)]
    pub fn table_names(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.0.values().all(Vec::is_empty)
    }
}

// ---------- npcs (terrain tables) -------------------------------------------

/// What an npc does in the game.
#[cfg(test)]
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
#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NpcDef {
    pub name: String,
    pub role: NpcRole,
    pub desc: String,
}

/// All npc tables, keyed by table name (terrain id).
#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NpcTables(BTreeMap<String, Vec<NpcDef>>);

#[cfg(test)]
impl NpcTables {
    pub fn npcs(&self, table: &str) -> Option<&Vec<NpcDef>> {
        self.0.get(table)
    }

    pub fn table_names(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }
}

// ---------- dialogue (keyed lines) ------------------------------------------

/// All dialogue, keyed by node id, npc id, or a special key (e.g. "title").
#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DialogueBook(BTreeMap<String, Vec<String>>);

#[cfg(test)]
impl DialogueBook {
    pub fn lines(&self, key: &str) -> Option<&Vec<String>> {
        self.0.get(key)
    }
}

// ---------- items (catalog by id) -------------------------------------------

/// An item's category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    /// Adds `count` food servings.
    Food,
    /// Holds `gallons`, poured in before the tank runs dry.
    FuelCan,
    /// Rounds for poaching.
    Ammo,
    Consumable,
    Medicine,
    Equipment,
    /// Bought to be owned.
    Collectible,
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
    /// What it is actually good for, told straight.
    #[serde(default)]
    pub helps: String,
    /// Unit count for multi-unit items (food servings…), if any.
    #[serde(default)]
    pub count: Option<u32>,
    /// Only for those travelling this way.
    #[serde(default)]
    pub ride: Option<Ride>,
    /// Only offered to this profession.
    #[serde(default)]
    pub for_job: Option<String>,
    /// Sold only at these stops, whatever their services.
    #[serde(default)]
    pub at: Vec<String>,
    /// Added to a winning score (before the multiplier) for owning it.
    #[serde(default)]
    pub score: u64,
    /// Its picture, where it has one.
    #[serde(default)]
    pub sprite: Option<String>,
    /// Kept after use, so one is all you need.
    #[serde(default)]
    pub reusable: bool,
    /// Only sold to professions that gather this way.
    #[serde(default)]
    pub gather: Option<GatherMode>,
    /// Stops with any of these services sell it.
    #[serde(default)]
    pub sold_at: Vec<Service>,
    /// Fuel held, for a `FuelCan`.
    #[serde(default)]
    pub gallons: f64,
    /// Counts as a repair, which some professions manage without.
    #[serde(default)]
    pub repair: bool,
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
}

// ---------- professions and photo credits ---------------------------------

/// A profession's edge, as multipliers on the ordinary rules. Anything a
/// profession leaves out stays at 1 (no effect).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Perk {
    /// Price per gallon at the pump.
    pub fuel_price: f64,
    /// Price of store goods.
    pub store_price: f64,
    /// How fast driving tires the party.
    pub fatigue: f64,
    /// How much mood is lost when something goes wrong.
    pub mood_loss: f64,
    /// Food brought home from foraging.
    pub forage_yield: f64,
    /// Breakdowns that a repair item fixes cost nothing.
    pub free_repairs: bool,
}

impl Default for Perk {
    fn default() -> Self {
        Perk {
            fuel_price: 1.0,
            store_price: 1.0,
            fatigue: 1.0,
            mood_loss: 1.0,
            forage_yield: 1.0,
            free_repairs: false,
        }
    }
}

/// A vehicle for sale at the start (`vehicles.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehicleDef {
    pub id: String,
    pub name: String,
    pub price: f64,
    pub tank_gallons: f64,
    pub mpg: f64,
    /// Name in the sprite atlas.
    pub sprite: String,
    pub blurb: String,
    #[serde(default)]
    pub ride: Ride,
    /// The most miles it covers in a day at an ordinary pace, if it is slower
    /// than the road allows.
    #[serde(default)]
    pub day_miles: Option<f64>,
    /// Camping in it rests the crew like a motel.
    #[serde(default)]
    pub comfy: bool,
    /// Offered at the car lot.
    #[serde(default = "yes")]
    pub for_sale: bool,
}

/// One entry in `professions.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfessionDef {
    pub name: String,
    pub money: f64,
    /// Final score multiplier.
    pub multiplier: u32,
    #[serde(default)]
    pub perk: Perk,
    pub perk_text: String,
    pub joke: String,
    pub goal: GoalDef,
    /// A vehicle that comes with the job, in place of buying one.
    #[serde(default)]
    pub mount: Option<String>,
    /// Whose names the party goes by.
    #[serde(default)]
    pub crew: Crew,
    /// How this profession gets food in the wild.
    pub gather: GatherMode,
    /// Character sprite families for a man and a woman.
    pub look_m: String,
    pub look_f: String,
}

impl ProfessionDef {
    /// The look id for a man or a woman.
    pub fn look(&self, female: bool) -> &str {
        if female {
            &self.look_f
        } else {
            &self.look_m
        }
    }

    /// Atlas name of the player's character facing `way`
    /// (`up`, `right`, `down` or `left`).
    pub fn hero_sprite(&self, female: bool, way: &str) -> String {
        let sex = if female { "f" } else { "m" };
        format!("hero_{}_{sex}_{way}", self.look(female))
    }
}

/// A photo backdrop for the gathering field (`fields.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDef {
    /// The picture is `img/<id>.jpg`.
    pub id: String,
    pub place: String,
    /// Terrain names (see `map_graph::Terrain`) it suits.
    pub terrains: Vec<Terrain>,
    /// Height of the skyline in the photo, from the top (0..1).
    pub horizon: f64,
    /// Where the skyline sits in the field; nothing walks above it.
    pub sky: f64,
    /// For dumpster diving rather than country.
    #[serde(default)]
    pub scavenge: bool,
    /// Sprites that hide the finds until they are dug into.
    #[serde(default)]
    pub covers: Vec<String>,
}

/// One kind of weather (`weather.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeatherDef {
    pub id: String,
    pub name: String,
    pub text: String,
    /// The same weather as it feels on horseback, if that differs.
    #[serde(default)]
    pub horse_text: Option<String>,
    /// Share of the usual miles covered in a day.
    pub speed: f64,
    #[serde(default)]
    pub mood: f64,
    #[serde(default)]
    pub health: f64,
    /// Added to the chance of a road event.
    #[serde(default)]
    pub risk: f64,
    /// Relative likelihood in each month, January first.
    pub months: [u32; 12],
    /// Further share of speed lost on a mountain pass.
    #[serde(default = "full_size")]
    pub pass_speed: f64,
    /// Worth warning about when picking a month.
    #[serde(default)]
    pub severe: bool,
    #[serde(default)]
    pub overlay: Option<OverlayDef>,
}

/// How a character's sprite strip is played (`looks.json`). Frames index the
/// three-wide strips in the sprite atlas.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LookDef {
    pub id: String,
    /// Frame while standing still.
    pub stand: u32,
    /// Frames of the walk cycle, in order.
    pub walk: Vec<u32>,
    /// Frame shown just after a shot, if the art has one.
    #[serde(default)]
    pub aim: Option<u32>,
    /// The art already carries a gun.
    #[serde(default)]
    pub armed: bool,
}

/// The country beside the road (`scenery.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneryBook {
    /// Oncoming vehicles by sprite name, in order; "" leaves a gap.
    pub traffic: Vec<String>,
    /// Sprite that rushes past in the foreground.
    pub brush: String,
    /// Puff behind a moving vehicle.
    pub exhaust: String,
    /// Portrait of the person asking for a ride.
    pub hitchhiker: String,
    pub terrains: Vec<TerrainLook>,
    /// Between them, every month.
    pub seasons: Vec<SeasonDef>,
}

impl SceneryBook {
    /// The season a month (1..=12) falls in.
    pub fn season(&self, month: u32) -> Option<&SeasonDef> {
        self.seasons.iter().find(|s| s.months.contains(&month))
    }

    pub fn look(&self, terrain: Terrain) -> Option<&TerrainLook> {
        self.terrains.iter().find(|t| t.terrain == terrain)
    }
}

/// What time of year does to the road (`scenery.json`). Pictures and sprites
/// are swapped by name; anything not named keeps its summer self.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeasonDef {
    pub id: String,
    pub name: String,
    /// Months (1..=12) it covers.
    pub months: Vec<u32>,
    /// Do wild plants grow?
    #[serde(default = "yes")]
    pub forage: bool,
    /// Ground colour for every kind of country.
    #[serde(default)]
    pub ground: Option<String>,
    /// A colour washed over the usual ground.
    #[serde(default)]
    pub ground_wash: Option<String>,
    /// A colour washed over the gathering field's photo.
    #[serde(default)]
    pub field_wash: Option<String>,
    /// Background layer swaps.
    #[serde(default)]
    pub layers: BTreeMap<String, String>,
    /// Roadside sprite swaps; "" fills the gaps.
    #[serde(default)]
    pub sprites: BTreeMap<String, String>,
    /// Traffic that stays home.
    #[serde(default)]
    pub parked_traffic: Vec<String>,
}

fn yes() -> bool {
    true
}

impl SeasonDef {
    pub fn layer<'a>(&'a self, name: &'a str) -> &'a str {
        self.layers.get(name).map_or(name, String::as_str)
    }

    pub fn sprite<'a>(&'a self, name: &'a str) -> &'a str {
        self.sprites.get(name).map_or(name, String::as_str)
    }
}

/// How one kind of country looks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrainLook {
    pub terrain: Terrain,
    pub sky: String,
    /// A picture stretched over the sky (`img/bg_<name>.png`).
    #[serde(default)]
    pub sky_image: Option<String>,
    pub ground: String,
    /// Ground colour of the gathering field, shown until its photo loads.
    pub field_ground: String,
    /// Parallax layers, back to front.
    pub layers: Vec<LayerDef>,
    /// Sprites beside the road; "" leaves a gap.
    pub roadside: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerDef {
    /// File is `img/bg_<image>.png`.
    pub image: String,
    /// Share of the road's speed: farther layers move less.
    pub parallax: f64,
    /// Height relative to the scene.
    pub height: f64,
    /// Own slow movement in pixels per second.
    #[serde(default)]
    pub drift: f64,
    /// Hangs from the top (clouds) instead of standing on the horizon.
    #[serde(default)]
    pub from_top: bool,
}

/// What a kind of weather looks like over the road.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OverlayDef {
    pub tint: String,
    /// Falling or blowing bits: how many, and their colour, size and speed.
    pub count: u32,
    pub color: String,
    pub width: f64,
    pub height: f64,
    pub fall: f64,
    pub slant: f64,
}

/// Default first names for the party (`names.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NameBook {
    pub source: String,
    pub wyoming: NamePool,
    pub hippie: NamePool,
}

impl NameBook {
    pub fn pool(&self, crew: Crew) -> &NamePool {
        match crew {
            Crew::Wyoming => &self.wyoming,
            Crew::Hippie => &self.hippie,
        }
    }

    /// Was this name handed out by the game rather than typed by the player?
    pub fn is_default(&self, name: &str) -> bool {
        [&self.wyoming, &self.hippie]
            .iter()
            .any(|p| p.men.iter().chain(&p.women).any(|n| n == name))
    }
}

/// First names for one kind of crew.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamePool {
    pub men: Vec<String>,
    pub women: Vec<String>,
}

/// Which crowd a profession travels with.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Crew {
    #[default]
    Wyoming,
    Hippie,
}

/// The two ways to get food off the land.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatherMode {
    Poach,
    /// Dumpster diving: what foragers do when nothing grows.
    Scavenge,
    Forage,
}

/// One animal or plant in `gather.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatherDef {
    pub name: String,
    pub mode: GatherMode,
    /// Food servings a plant yields.
    #[serde(default)]
    pub food: u32,
    /// Pounds of meat on an animal.
    #[serde(default)]
    pub pounds: u32,
    /// Walking speed in field units per second (animals).
    #[serde(default)]
    pub speed: f64,
    /// Relative chance to appear.
    pub weight: u32,
    pub sprite: String,
    /// Sprite while running away; defaults to `sprite`.
    #[serde(default)]
    pub flee: Option<String>,
    /// Drawing and hit size relative to the sprite.
    #[serde(default = "full_size")]
    pub size: f64,
    #[serde(default)]
    pub poison: bool,
    /// Photo of the real species: `img/<photo>.jpg`.
    #[serde(default)]
    pub photo: Option<String>,
    pub fact: String,
}

fn full_size() -> f64 {
    1.0
}

/// The celebration at the end of a mission.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinaleDef {
    pub scene: FinaleScene,
    /// Sprites round the circle, left to right.
    pub drummers: Vec<String>,
    /// Sprites who appear as the groove builds.
    pub guests: Vec<String>,
    /// The button that joins in.
    pub invite: String,
    /// Said once the groove is complete.
    pub won: String,
}

/// Where the celebration happens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinaleScene {
    /// Round a fire under the full moon.
    Canyon,
    /// In the condo, among the collection.
    Condo,
}

/// A profession's mission: reach each step's stop, in order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoalDef {
    pub title: String,
    pub steps: Vec<GoalStep>,
    /// Days allowed, counting the day you leave.
    pub days: u32,
    /// Why it's over when the days run out.
    pub late: String,
    /// What the days-left count is called, if not the usual.
    #[serde(default)]
    pub countdown: Option<String>,
    /// A scene to play once the mission is done.
    #[serde(default)]
    pub finale: Option<FinaleDef>,
}

fn town_with_food() -> Service {
    Service::Food
}

/// An odd job in town (`gigs.json`): costs days, pays cash.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GigDef {
    pub id: String,
    pub name: String,
    pub days: u32,
    pub pay: f64,
    #[serde(default)]
    pub fatigue: f64,
    #[serde(default)]
    pub mood: f64,
    #[serde(default)]
    pub health: f64,
    /// Food servings thrown in.
    #[serde(default)]
    pub food: u32,
    /// Only offered at these stops; empty means any stop with `needs`.
    #[serde(default)]
    pub at: Vec<String>,
    #[serde(default = "town_with_food")]
    pub needs: Service,
    pub pitch: String,
    pub done: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoalStep {
    /// Node ids; arriving at any one of them completes the step.
    pub at: Vec<String>,
    /// What to do, as shown to the player.
    pub task: String,
    /// Shown on completion. `{vehicle}` is replaced with the vehicle's name.
    pub done: String,
}

/// Attribution for one bundled photo (`credits.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreditDef {
    /// Node id; the photo is `img/<id>.jpg`.
    pub id: String,
    pub file: String,
    pub author: String,
    pub license: String,
    pub license_url: String,
    pub source: String,
}

/// One animation strip in the sprite sheet (`sprites.json`): `frames` cells
/// of `w`×`h` pixels, left to right from (`x`, `y`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpriteDef {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub frames: u32,
    /// [x, y, width, height] within a cell of what can be seen.
    pub art: [u32; 4],
}

impl SpriteDef {
    /// Rows from the top of a cell to the soles of the feet (or tyres).
    pub fn feet(&self) -> u32 {
        self.art[1] + self.art[3]
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
        assert_eq!(e.effect, Effect::default());
        let again: EventDef = serde_json::from_str(&serde_json::to_string(&e).unwrap()).unwrap();
        assert_eq!(again, e);
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

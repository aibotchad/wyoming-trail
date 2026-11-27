//! All balance numbers live here — no magic numbers in game logic (see Rules).

/// Base highway speed in miles per hour, before terrain factors.
pub const BASE_MPH: f64 = 45.0;

/// Health lost per day for each starving member.
pub const STARVE_HEALTH_LOSS: f64 = 10.0;
/// Extra fatigue per day for each starving member.
pub const STARVE_FATIGUE_GAIN: f64 = 20.0;

/// Health lost per day for a member at extreme fatigue (>90).
pub const EXHAUSTED_HEALTH_LOSS: f64 = 5.0;
/// Fatigue above which members are considered exhausted.
pub const EXHAUSTED_FATIGUE: f64 = 90.0;
/// Fatigue below which members naturally recover a little health.
pub const WELL_RESTED_FATIGUE: f64 = 40.0;
/// Health recovered per day for well-rested members.
pub const REST_HEALTH_GAIN: f64 = 3.0;

/// Share of a full tank included with a truck purchase.
pub const START_FUEL_FRACTION: f64 = 0.25;

/// Chance of a random event on a drive day, plus extra per road risk point.
pub const EVENT_BASE_CHANCE: f64 = 0.35;
pub const EVENT_RISK_STEP: f64 = 0.06;

/// A night at a motel (whole party).
pub const MOTEL_COST: f64 = 90.0;
pub const MOTEL_HEALTH_GAIN: f64 = 20.0;
pub const MOTEL_FATIGUE_RECOVERY: f64 = 45.0;
pub const MOTEL_MOOD_GAIN: f64 = 10.0;
/// A free night in the truck.
pub const CAMP_HEALTH_GAIN: f64 = 6.0;
pub const CAMP_FATIGUE_RECOVERY: f64 = 20.0;
pub const CAMP_MOOD_GAIN: f64 = -4.0;

/// One poaching or foraging round.
pub const GATHER_SECONDS: f64 = 45.0;
/// Most food servings a forager can carry back from a round.
pub const GATHER_CARRY: u32 = 30;
/// Most meat a poacher can carry back, however much was shot.
pub const MEAT_CARRY_LB: u32 = 200;
/// Pounds of meat in one food serving.
pub const POUNDS_PER_SERVING: u32 = 4;
/// Rounds every poacher starts with.
pub const STARTING_AMMO: u32 = 10;
pub const WALK_SPEED: f64 = 75.0;
pub const BULLET_SPEED: f64 = 320.0;
pub const SHOT_COOLDOWN: f64 = 0.35;
/// How close a shot must pass to a full-size animal.
pub const HIT_RADIUS: f64 = 17.0;
/// Speed-up once the shooting starts.
pub const FLEE_MULTIPLIER: f64 = 1.8;
pub const SPAWN_SECONDS: f64 = 1.2;
/// How close a forager must be to pick a plant.
pub const PICK_REACH: f64 = 18.0;
/// Chance a game warden catches a poacher, out in the country and somewhere
/// hunting is banned (towns and parks).
pub const WARDEN_CHANCE: f64 = 0.11;
pub const WARDEN_PROTECTED_CHANCE: f64 = 0.4;
/// Chance a forager in a town or park meets a ranger, and the fine.
pub const RANGER_CHANCE: f64 = 0.2;
pub const RANGER_FINE: f64 = 90.0;
/// Chance the night manager catches a dumpster diver, and what it costs in mood.
pub const MANAGER_CHANCE: f64 = 0.12;
pub const MANAGER_MOOD_LOSS: f64 = 8.0;
/// Health lost by whoever eats each poisonous find.
pub const POISON_HEALTH_LOSS: f64 = 25.0;

/// Chance, on an uneventful drive day, that someone needs a ride.
pub const HITCHHIKER_CHANCE: f64 = 0.2;
/// Days per month on the trip calendar.
pub const DAYS_PER_MONTH: u32 = 30;
/// Departure month if none was chosen (June).
pub const DEFAULT_MONTH: u32 = 6;

/// Cash for a serving of wild food at a back door.
pub const WILD_FOOD_PRICE: f64 = 3.0;
/// Chance that selling wild meat gets a poacher reported.
pub const SELLING_RISK: f64 = 0.1;
/// Share of the shop price a pawn counter pays for gear.
pub const PAWN_SHARE: f64 = 0.5;

/// A road's scenery rating (0-5) above this lifts mood; below it, lowers it.
pub const SCENERY_NEUTRAL: f64 = 2.5;
pub const SCENERY_MOOD: f64 = 0.5;
/// Animals in the poaching field at once, and plants in the foraging one.
pub const HERD_SIZE: usize = 4;
pub const PLANT_PATCH: usize = 12;
/// A tap this close to a plant's centre means that plant.
pub const TAP_SNAP: f64 = 24.0;
/// Closest two plants may grow, and how clear they keep of the forager.
pub const PLANT_SPACING: f64 = 33.0;
pub const PLANT_CLEARANCE: f64 = 40.0;
/// Plants and animals stay at least this far above the bottom edge, clear of
/// the message strip drawn there.
pub const FIELD_FLOOR: f64 = 40.0;
/// And plants this far from the sides, so none is cut off.
pub const PLANT_EDGE: f64 = 22.0;
/// The gatherer stops this far above it, so their feet stay in view.
pub const WALK_FLOOR: f64 = 20.0;
/// Seconds a dug-up bit of rubbish (or worse) stays out before it sinks away.
pub const SHOW_BAD_SECONDS: f64 = 3.0;
/// Seconds a dumpster find is announced across the screen.
pub const FLASH_SECONDS: f64 = 1.5;
/// Seconds the leap into the dumpster takes to watch.
pub const JUMP_SECONDS: f64 = 1.0;
/// Seconds a picked plant's photo stays up.
pub const PHOTO_SECONDS: f64 = 3.0;

/// The most travellers, and the furthest day, a saved game may claim.
pub const MAX_PARTY: usize = 4;
pub const MAX_DAY: u32 = 1_000;

/// Finished missions kept on the high-score list.
pub const TOP_SCORES: usize = 10;

/// Nights in the pause menu's longer rest.
pub const LONG_REST: u32 = 3;

/// Driving on by itself stops if anyone is this poorly.
pub const PULL_OVER_HEALTH: f64 = 35.0;

/// Longest party member name.
pub const NAME_MAX_CHARS: usize = 14;

/// Final score weights (before the profession multiplier).
pub const SCORE_PER_SURVIVOR: f64 = 500.0;
pub const SCORE_PER_AVG_HEALTH: f64 = 5.0;
pub const SCORE_PER_DOLLAR: f64 = 0.25;
pub const SCORE_PER_PLACE: f64 = 100.0;
pub const SCORE_PER_DAY: f64 = -20.0;

/// A driving pace.
#[derive(Debug, Clone, Copy)]
pub struct PaceDef {
    pub name: &'static str,
    /// Hours driven that day.
    pub hours: f64,
    /// Fatigue gained (negative = recovered) per member.
    pub fatigue_delta: f64,
    /// Mood delta per member.
    pub mood_delta: f64,
    /// What choosing it means, in the player's words.
    pub note: &'static str,
}

/// Relaxed → standard → rushed, in menu order.
pub const PACES: [PaceDef; 3] = [
    PaceDef {
        name: "Relaxed",
        hours: 4.0,
        fatigue_delta: -12.0,
        mood_delta: 2.0,
        note: "Fewer miles; the crew rests up.",
    },
    PaceDef {
        name: "Standard",
        hours: 6.0,
        fatigue_delta: 6.0,
        mood_delta: 0.0,
        note: "A little tiring.",
    },
    PaceDef {
        name: "Rushed",
        hours: 8.0,
        fatigue_delta: 20.0,
        mood_delta: -2.0,
        note: "More miles; tiring and grumpy.",
    },
];

/// A ration level for the day.
#[derive(Debug, Clone, Copy)]
pub struct MealDef {
    pub name: &'static str,
    /// Food servings eaten per person per day.
    pub servings: u32,
    /// Mood delta per member.
    pub mood_delta: f64,
    /// What choosing it means, in the player's words.
    pub note: &'static str,
}

/// Stingy → normal → generous, in menu order.
pub const MEALS: [MealDef; 3] = [
    MealDef {
        name: "Gas station scraps",
        servings: 1,
        mood_delta: -3.0,
        note: "Saves food; sours the mood.",
    },
    MealDef {
        name: "Square meals",
        servings: 2,
        mood_delta: 0.0,
        note: "Nobody complains.",
    },
    MealDef {
        name: "All-you-can-eat",
        servings: 3,
        mood_delta: 5.0,
        note: "Eats food fast; lifts the mood.",
    },
];

/// Clamp a 0-100 stat to bounds.
pub fn clamp0100(v: f64) -> f64 {
    v.clamp(0.0, 100.0)
}

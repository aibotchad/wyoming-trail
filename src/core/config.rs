//! All balance numbers live here — no magic numbers in game logic (see Rules).

/// Base highway speed in miles per hour, before terrain factors.
pub const BASE_MPH: f64 = 45.0;

/// Default fuel price in dollars/gallon. Remote stations sell higher (M3).
#[allow(dead_code)] // first consumer: M3 (gas stations).
pub const FUEL_PRICE_PER_GAL: f64 = 3.85;

/// Meals each living party member eats per day.
pub const MEALS_PER_DAY: u32 = 1;

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

/// Starting money in dollars.
pub const STARTING_MONEY: f64 = 3_000.0;

/// Starting food (meals) if not adjusted at outfitting.
pub const STARTING_FOOD: u32 = 40;

/// Months used for the in-game calendar (1 = January).
#[allow(dead_code)] // first consumer: M5 (calendar).
pub const MONTHS_PER_YEAR: u32 = 12;
/// Days per month (simplified calendar).
pub const DAYS_PER_MONTH: u32 = 30;

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
}

/// Relaxed → standard → rushed, in menu order.
pub const PACES: [PaceDef; 3] = [
    PaceDef {
        name: "Relaxed",
        hours: 4.0,
        fatigue_delta: -12.0,
        mood_delta: 2.0,
    },
    PaceDef {
        name: "Standard",
        hours: 6.0,
        fatigue_delta: 6.0,
        mood_delta: 0.0,
    },
    PaceDef {
        name: "Rushed",
        hours: 8.0,
        fatigue_delta: 20.0,
        mood_delta: -2.0,
    },
];

/// A meal option for the day.
#[derive(Debug, Clone, Copy)]
pub struct MealDef {
    pub name: &'static str,
    /// Cost per person in dollars.
    pub cost_per_person: f64,
    /// Mood delta per member.
    pub mood_delta: f64,
}

/// Cheap → normal → diner, in menu order.
pub const MEALS: [MealDef; 3] = [
    MealDef {
        name: "Cheap",
        cost_per_person: 5.0,
        mood_delta: -3.0,
    },
    MealDef {
        name: "Normal",
        cost_per_person: 12.0,
        mood_delta: 0.0,
    },
    MealDef {
        name: "Diner",
        cost_per_person: 25.0,
        mood_delta: 5.0,
    },
];

/// A truck the player can choose at outfitting in Cheyenne.
#[derive(Debug, Clone, Copy)]
pub struct TruckDef {
    pub name: &'static str,
    pub tank_gallons: f64,
    pub mpg: f64,
    /// Starting engine condition, 0-100.
    pub condition: f64,
}

pub const TRUCKS: [TruckDef; 3] = [
    TruckDef {
        name: "Older pickup",
        tank_gallons: 18.0,
        mpg: 15.0,
        condition: 55.0,
    },
    TruckDef {
        name: "Standard pickup",
        tank_gallons: 25.0,
        mpg: 19.0,
        condition: 75.0,
    },
    TruckDef {
        name: "Diesel workhorse",
        tank_gallons: 35.0,
        mpg: 24.0,
        condition: 90.0,
    },
];

/// Clamp a 0-100 stat to bounds.
pub fn clamp0100(v: f64) -> f64 {
    v.clamp(0.0, 100.0)
}

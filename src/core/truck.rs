//! The truck: fuel, mileage, tank, condition.

use serde::{Deserialize, Serialize};

use crate::core::config::START_FUEL_FRACTION;
use crate::core::defs::VehicleDef;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Truck {
    /// Id in `vehicles.json`.
    pub id: String,
    pub name: String,
    /// Gallons currently in the tank.
    pub fuel_gallons: f64,
    /// Tank capacity in gallons.
    pub tank_gallons: f64,
    /// Miles per gallon.
    pub mpg: f64,
}

impl Truck {
    /// New truck from an outfitting definition, partly fuelled.
    pub fn from_def(def: &VehicleDef) -> Self {
        Truck {
            id: def.id.clone(),
            name: def.name.clone(),
            fuel_gallons: def.tank_gallons * START_FUEL_FRACTION,
            tank_gallons: def.tank_gallons,
            mpg: def.mpg,
        }
    }

    /// Gallons needed to drive `miles`.
    pub fn gallons_for(&self, miles: f64) -> f64 {
        miles / self.mpg
    }

    /// Miles the truck can still make on current fuel.
    pub fn current_range_miles(&self) -> f64 {
        self.fuel_gallons * self.mpg
    }

    /// Add fuel up to the tank size; returns gallons actually added.
    pub fn add_fuel(&mut self, gallons: f64) -> f64 {
        let added = gallons.min(self.tank_gallons - self.fuel_gallons).max(0.0);
        self.fuel_gallons += added;
        added
    }

    /// Consume gallons of fuel; returns gallons actually used (won't go < 0).
    pub fn drain(&mut self, gallons: f64) -> f64 {
        let used = gallons.min(self.fuel_gallons);
        self.fuel_gallons -= used;
        used
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::load_vehicles;

    #[test]
    fn new_truck_is_partly_fuelled() {
        let t = Truck::from_def(&load_vehicles()[1]);
        assert_eq!(t.fuel_gallons, t.tank_gallons * START_FUEL_FRACTION);
    }

    #[test]
    fn fuel_stays_within_the_tank() {
        let mut t = Truck::from_def(&load_vehicles()[0]);
        t.add_fuel(1000.0);
        assert_eq!(t.fuel_gallons, t.tank_gallons);
        assert_eq!(t.drain(1000.0), t.tank_gallons);
        assert_eq!(t.fuel_gallons, 0.0);
    }

    #[test]
    fn range_math() {
        let mut t = Truck::from_def(&load_vehicles()[1]);
        t.add_fuel(t.tank_gallons);
        // 25 gal * 19 mpg = 475 mi
        assert!((t.current_range_miles() - 475.0).abs() < 1e-6);
    }
}

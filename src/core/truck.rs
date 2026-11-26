//! The truck: fuel, mileage, tank, condition.

use serde::{Deserialize, Serialize};

use crate::core::config::TruckDef;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Truck {
    pub name: String,
    /// Gallons currently in the tank.
    pub fuel_gallons: f64,
    /// Tank capacity in gallons.
    pub tank_gallons: f64,
    /// Miles per gallon.
    pub mpg: f64,
    /// Engine condition, 0-100 (breakdown risk in M3; display only for M1).
    pub condition: f64,
    /// Spare tires carried.
    pub spare_tires: u32,
}

impl Truck {
    /// New truck from an outfitting definition, full tank.
    pub fn from_def(def: TruckDef) -> Self {
        Truck {
            name: def.name.to_string(),
            fuel_gallons: def.tank_gallons,
            tank_gallons: def.tank_gallons,
            mpg: def.mpg,
            condition: def.condition,
            spare_tires: 1,
        }
    }

    /// Gallons needed to drive `miles`.
    pub fn gallons_for(&self, miles: f64) -> f64 {
        miles / self.mpg
    }

    /// Miles the truck can still make on current fuel.
    #[allow(dead_code)] // first consumer: M2 (HUD/M3 fuel purchase).
    pub fn current_range_miles(&self) -> f64 {
        self.fuel_gallons * self.mpg
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
    use crate::core::config::TRUCKS;

    #[test]
    fn new_truck_is_full() {
        let t = Truck::from_def(TRUCKS[1]);
        assert_eq!(t.fuel_gallons, t.tank_gallons);
        assert_eq!(t.tank_gallons, 25.0);
    }

    #[test]
    fn drain_never_negative() {
        let mut t = Truck::from_def(TRUCKS[0]);
        let used = t.drain(1000.0);
        assert_eq!(used, t.tank_gallons);
        assert_eq!(t.fuel_gallons, 0.0);
        assert_eq!(t.drain(1.0), 0.0);
    }

    #[test]
    fn range_math() {
        let t = Truck::from_def(TRUCKS[1]);
        // 25 gal * 19 mpg = 475 mi
        assert!((t.current_range_miles() - 475.0).abs() < 1e-6);
    }
}

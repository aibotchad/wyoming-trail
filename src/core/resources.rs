//! Consumables and money the party carries.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Resources {
    pub money: f64,
    /// Meals of food.
    pub food: u32,
    /// Bottles/packs of water (M3).
    pub water: u32,
    /// Shells/cartridges (M3 hunt).
    pub ammo: u32,
    pub hunting_license: bool,
    /// First-aid doses.
    pub medicine: u32,
    /// One roadside emergency kit.
    pub emergency_kit: bool,
    pub cooler_ice: bool,
}

impl Resources {
    /// Try to pay; advances only on success.
    pub fn pay(&mut self, amount: f64) -> bool {
        if amount < 0.0 || self.money < amount {
            return false;
        }
        self.money -= amount;
        true
    }

    /// Take meals of food if we have them.
    pub fn take_food(&mut self, meals: u32) -> bool {
        if self.food < meals {
            return false;
        }
        self.food -= meals;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pays_only_when_affordable() {
        let mut r = Resources {
            money: 100.0,
            ..Default::default()
        };
        assert!(r.pay(99.0));
        assert!(!r.pay(100.0));
        assert_eq!(r.money, 1.0);
    }

    #[test]
    fn food_taken_when_available() {
        let mut r = Resources {
            food: 3,
            ..Default::default()
        };
        assert!(r.take_food(3));
        assert!(!r.take_food(1));
        assert_eq!(r.food, 0);
    }
}

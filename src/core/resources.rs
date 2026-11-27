//! Consumables and money the party carries.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Resources {
    pub money: f64,
    /// Food servings.
    pub food: u32,
    /// How many of those servings are wild (poached or foraged), and so can
    /// be sold.
    #[serde(default)]
    pub wild: u32,
    /// Item id → units carried.
    pub inventory: BTreeMap<String, u32>,
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

    /// Lose up to `amount` dollars (fines, scams) without going negative.
    pub fn lose_money(&mut self, amount: f64) {
        self.money = (self.money - amount).max(0.0);
    }

    pub fn count(&self, id: &str) -> u32 {
        self.inventory.get(id).copied().unwrap_or(0)
    }

    pub fn add_item(&mut self, id: &str, units: u32) {
        *self.inventory.entry(id.to_string()).or_insert(0) += units;
    }

    /// Use up one unit if we have it.
    pub fn use_item(&mut self, id: &str) -> bool {
        match self.inventory.get_mut(id) {
            Some(n) if *n > 0 => {
                *n -= 1;
                true
            }
            _ => false,
        }
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
        r.lose_money(50.0);
        assert_eq!(r.money, 0.0);
    }

    #[test]
    fn items_run_out() {
        let mut r = Resources::default();
        r.add_item("duct_tape", 1);
        assert!(r.use_item("duct_tape"));
        assert!(!r.use_item("duct_tape"));
        assert!(!r.use_item("banjo"));
    }
}

//! What the party carries and its daily choices.

use super::*;
use crate::core::defs::{Ride, RideDef, VehicleDef, WeatherDef};

impl Game {
    /// The current profession's edge.
    pub fn perk(&self) -> Perk {
        self.job().map_or_else(Perk::default, |j| j.perk)
    }

    /// The vehicle (or horse) in hand.
    pub fn vehicle(&self) -> Option<&VehicleDef> {
        let id = &self.state.truck.id;
        self.content.vehicles.iter().find(|v| v.id == *id)
    }

    pub fn ride(&self) -> Ride {
        self.vehicle().map_or(Ride::Motor, |v| v.ride)
    }

    /// Words and costs for the way the party is travelling.
    pub fn ride_def(&self) -> &RideDef {
        let ride = self.ride();
        let found = self.content.rides.iter().find(|r| r.ride == ride);
        // A test makes sure every way of travelling is described.
        found.unwrap_or(&self.content.rides[0])
    }

    /// What the weather feels like, the way the party is travelling.
    pub fn weather_text<'a>(&self, w: &'a WeatherDef) -> &'a str {
        match (&w.horse_text, self.ride()) {
            (Some(text), Ride::Horse) => text,
            _ => &w.text,
        }
    }

    /// The collectibles owned, as (id, definition).
    pub fn collection(&self) -> Vec<(&str, &ItemDef)> {
        self.items_of(ItemKind::Collectible)
            .filter(|(id, _)| self.state.resources.count(id) > 0)
            .collect()
    }

    /// Items of one kind, as (id, definition).
    pub(super) fn items_of(&self, kind: ItemKind) -> impl Iterator<Item = (&str, &ItemDef)> {
        let items = &self.content.items;
        items
            .ids()
            .filter_map(|id| items.get(id).map(|d| (id, d)))
            .filter(move |(_, d)| d.kind == kind)
    }

    /// Rounds left.
    pub fn ammo(&self) -> u32 {
        self.items_of(ItemKind::Ammo)
            .map(|(id, _)| self.state.resources.count(id))
            .sum()
    }

    /// Set the rounds left (held under the first ammunition item).
    pub(super) fn set_ammo(&mut self, rounds: u32) {
        let ids: Vec<String> = self
            .items_of(ItemKind::Ammo)
            .map(|(id, _)| id.to_string())
            .collect();
        for (i, id) in ids.into_iter().enumerate() {
            let held = if i == 0 { rounds } else { 0 };
            self.state.resources.inventory.insert(id, held);
        }
    }

    /// Full fuel cans carried.
    pub fn fuel_cans(&self) -> u32 {
        self.items_of(ItemKind::FuelCan)
            .map(|(id, _)| self.state.resources.count(id))
            .sum()
    }

    /// Miles the party can cover on the tank plus the cans.
    pub fn range_miles(&self) -> f64 {
        let spare: f64 = self
            .items_of(ItemKind::FuelCan)
            .map(|(id, d)| f64::from(self.state.resources.count(id)) * d.gallons)
            .sum();
        self.state.truck.current_range_miles() + spare * self.state.truck.mpg
    }

    /// Open one can, if any, and return the gallons in it.
    pub(super) fn use_fuel_can(&mut self) -> Option<f64> {
        let (id, gallons) = self
            .items_of(ItemKind::FuelCan)
            .find(|(id, _)| self.state.resources.count(id) > 0)
            .map(|(id, d)| (id.to_string(), d.gallons))?;
        self.state.resources.use_item(&id);
        Some(gallons)
    }

    pub fn cycle_pace(&mut self) {
        self.state.pace_index = (self.state.pace_index + 1) % PACES.len();
    }

    pub fn cycle_meal(&mut self) {
        self.state.meal_index = (self.state.meal_index + 1) % MEALS.len();
    }

    /// Food servings the party eats in a day at the chosen rations.
    pub fn daily_food(&self) -> u32 {
        let meal = MEALS[self.state.meal_index.min(MEALS.len() - 1)];
        self.state.party.in_trip_count() as u32 * meal.servings
    }
}

//! The party: four members with health / fatigue / mood and trip status.

use serde::{Deserialize, Serialize};

use crate::core::config::clamp0100;

/// Where a member stands in the trip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemberStatus {
    Healthy,
    Sick,
    Injured,
    /// Left the trip (health hit 0 — family-friendly, nobody "dies").
    LeftTrip,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartyMember {
    pub name: String,
    /// 0-100.
    pub health: f64,
    /// 0-100; high fatigue hurts.
    pub fatigue: f64,
    /// 0-100; flavor + affects choices in M2/M3.
    pub mood: f64,
    pub status: MemberStatus,
}

impl PartyMember {
    pub fn new(name: &str) -> Self {
        PartyMember {
            name: name.to_string(),
            health: 100.0,
            fatigue: 0.0,
            mood: 70.0,
            status: MemberStatus::Healthy,
        }
    }

    /// True if this member counts toward the party / can do things.
    pub fn in_trip(&self) -> bool {
        self.status != MemberStatus::LeftTrip
    }

    /// Apply a health delta; if health hits 0 the member has to leave the
    /// trip. Returns true if they left (screen-worthy event — logged by caller).
    pub fn apply_health(&mut self, delta: f64) -> bool {
        if !self.in_trip() {
            return false;
        }
        self.health = clamp0100(self.health + delta);
        if self.health <= 0.0 {
            self.status = MemberStatus::LeftTrip;
            return true;
        }
        if self.status == MemberStatus::Healthy {
            self.status = if delta < 0.0 {
                MemberStatus::Injured
            } else {
                MemberStatus::Healthy
            };
        }
        false
    }

    /// Apply a fatigue delta (clamped 0-100).
    pub fn apply_fatigue(&mut self, delta: f64) {
        self.fatigue = clamp0100(self.fatigue + delta);
    }

    /// Apply a mood delta (clamped 0-100).
    pub fn apply_mood(&mut self, delta: f64) {
        self.mood = clamp0100(self.mood + delta);
    }
}

/// The whole party — 4 members.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Party {
    pub members: Vec<PartyMember>,
}

impl Party {
    pub fn new(names: &[&str]) -> Self {
        Party {
            members: names.iter().map(|n| PartyMember::new(n)).collect(),
        }
    }

    /// Members who are still on the trip.
    pub fn in_trip(&self) -> Vec<&PartyMember> {
        self.members.iter().filter(|m| m.in_trip()).collect()
    }

    pub fn in_trip_count(&self) -> usize {
        self.in_trip().len()
    }

    /// True when the trip can no longer go on (everyone left).
    pub fn all_left(&self) -> bool {
        self.members
            .iter()
            .all(|m| m.status == MemberStatus::LeftTrip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_members_start_healthy() {
        let p = Party::new(&["A", "B", "C", "D"]);
        assert_eq!(p.members.len(), 4);
        assert_eq!(p.in_trip_count(), 4);
        for m in &p.members {
            assert_eq!(m.health, 100.0);
            assert_eq!(m.status, MemberStatus::Healthy);
        }
    }

    #[test]
    fn health_zero_means_left_trip_not_death() {
        let mut m = PartyMember::new("A");
        assert!(!m.apply_health(-10.0));
        assert!(m.apply_health(-99.0)); // leaves the trip
        assert_eq!(m.status, MemberStatus::LeftTrip);
        assert!(!m.in_trip());
    }

    #[test]
    fn stats_stay_in_bounds() {
        let mut m = PartyMember::new("A");
        m.apply_fatigue(500.0);
        m.apply_fatigue(-500.0);
        m.apply_mood(-500.0);
        assert_eq!(m.fatigue, 0.0);
        assert_eq!(m.mood, 0.0);
    }
}

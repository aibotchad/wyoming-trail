//! The trip's day-by-day record, which the graphs are drawn from.

use super::*;
use crate::core::state::DaySample;

impl Game {
    /// Note how things stand today. Call after anything that changes them.
    pub fn record(&mut self) {
        // Nothing to chart until the shopping starts.
        if self.state.screen.is_setup() {
            return;
        }
        let today = DaySample {
            day: self.state.day,
            miles: 0.0,
            money: self.state.resources.money,
            food: self.state.resources.food,
        };
        let history = &mut self.state.history;
        // Nothing in the record may lie ahead of today.
        history.retain(|d| d.day <= today.day);
        // Days that went by at once (a job, a tow) keep the figures before.
        while let Some(&last) = history.last().filter(|l| l.day + 1 < today.day) {
            history.push(DaySample {
                day: last.day + 1,
                miles: 0.0,
                ..last
            });
        }
        match history.last_mut() {
            Some(last) if last.day == today.day => {
                last.money = today.money;
                last.food = today.food;
            }
            _ => history.push(today),
        }
    }

    /// Add miles driven to a day already in the record.
    pub(super) fn log_miles(&mut self, day: u32, miles: f64) {
        if let Some(sample) = self.state.history.iter_mut().find(|s| s.day == day) {
            sample.miles += miles;
        }
    }
}

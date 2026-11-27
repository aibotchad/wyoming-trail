//! The profession's goal, its deadline, the calendar and the weather.

use super::*;
use crate::core::defs::SeasonDef;
use crate::core::scores::HighScore;

/// Where one step of the mission stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalState {
    Done,
    Current,
    Later,
}

impl Game {
    pub fn job(&self) -> Option<&ProfessionDef> {
        self.content.professions.get(self.state.profession)
    }

    /// The mission step still to be done.
    pub fn current_step(&self) -> Option<&GoalStep> {
        self.job()?.goal.steps.get(self.state.goal_step)
    }

    /// This run's line for the high-score list, once the mission is done.
    pub fn high_score(&self) -> Option<HighScore> {
        let won = self.state.screen == Screen::Win && self.state.score.total > 0;
        let (job, driver) = (self.job()?, self.state.party.members.first()?);
        won.then(|| HighScore {
            name: driver.name.clone(),
            job: job.name.clone(),
            score: self.state.score.total,
            days: self.state.score.days_traveled,
            survivors: self.state.score.party_survivors,
            run: self.state.seed,
        })
    }

    /// Every step of the mission, and whether it has been met.
    pub fn goals(&self) -> Vec<(&str, GoalState)> {
        let at = self.state.goal_step;
        let steps = self.job().map_or(&[][..], |j| &j.goal.steps);
        let state = |i: usize| match i.cmp(&at) {
            std::cmp::Ordering::Less => GoalState::Done,
            std::cmp::Ordering::Equal => GoalState::Current,
            std::cmp::Ordering::Greater => GoalState::Later,
        };
        let each = steps.iter().enumerate();
        each.map(|(i, s)| (s.task.as_str(), state(i))).collect()
    }

    /// Stops that would complete the current step.
    pub fn targets(&self) -> &[String] {
        self.current_step().map_or(&[], |s| &s.at)
    }

    /// Days left to finish the mission, counting today.
    pub fn days_left(&self) -> u32 {
        self.job()
            .map_or(0, |j| j.goal.days.saturating_sub(self.state.day - 1))
    }

    /// True once more days have passed than the mission allows. The last day
    /// itself still counts.
    pub(super) fn overdue(&self) -> bool {
        self.job().is_some_and(|j| self.state.day - 1 > j.goal.days)
    }

    /// End the run if the mission's days are used up. Arriving in time has
    /// already won by now.
    pub(super) fn check_deadline(&mut self) {
        let over = matches!(self.state.screen, Screen::Win | Screen::GameOver);
        if over || self.days_left() > 0 {
            return;
        }
        if let Some(job) = self.job() {
            self.state.ending_reason = job.goal.late.clone();
            self.finish(Screen::GameOver);
        }
    }

    /// Total shortest-road distance of a profession's whole mission.
    pub fn mission_miles(&self, profession: usize) -> Option<f64> {
        let mut at = self.content.map.start.clone();
        let mut miles = 0.0;
        for step in &self.content.professions.get(profession)?.goal.steps {
            let path = self.content.map.route(&at, &step.at)?;
            for &i in &path {
                miles += self.content.map.roads[i].distance_miles;
                at = self.content.map.roads[i].to.clone();
            }
        }
        Some(miles)
    }

    /// The season on a day of this trip.
    pub fn season(&self, day: u32) -> Option<&SeasonDef> {
        self.content.scenery.season(self.date(day).0)
    }

    /// Today's month (1..=12) and day of the month.
    pub fn date(&self, day: u32) -> (u32, u32) {
        let passed = day.saturating_sub(1);
        let month = (self.state.start_month.max(1) - 1 + passed / DAYS_PER_MONTH) % 12 + 1;
        (month, passed % DAYS_PER_MONTH + 1)
    }

    /// The statewide weather on a day of this trip: seeded, so the forecast
    /// shown is the weather driven through.
    pub fn weather_on(&self, day: u32) -> Option<&WeatherDef> {
        let month = self.date(day).0 as usize - 1;
        let mut rng = Rng::for_day(self.state.seed ^ 0x57EA, day);
        let total: u32 = self.content.weather.iter().map(|w| w.months[month]).sum();
        let mut roll = (rng.unit() * f64::from(total)) as u32;
        self.content.weather.iter().find(|w| {
            if roll < w.months[month] {
                true
            } else {
                roll -= w.months[month];
                false
            }
        })
    }

    /// The likeliest weather of a month (1..=12), severe kinds always named.
    pub fn month_outlook(&self, month: u32) -> Vec<&str> {
        let i = (month.clamp(1, 12) - 1) as usize;
        let mut likely: Vec<&WeatherDef> = self
            .content
            .weather
            .iter()
            .filter(|w| w.months[i] > 0)
            .collect();
        likely.sort_by_key(|w| std::cmp::Reverse(w.months[i]));
        likely
            .iter()
            .enumerate()
            .filter(|(rank, w)| *rank < 3 || w.severe)
            .map(|(_, w)| w.name.as_str())
            .collect()
    }
}

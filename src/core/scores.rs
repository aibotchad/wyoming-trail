//! The top ten: the best finished missions, kept from run to run. The list is
//! stored as text outside the game, so reading it trusts nothing.

use serde::{Deserialize, Serialize};

use crate::core::config::{MAX_PARTY, NAME_MAX_CHARS, TOP_SCORES};

/// Longest job title a stored line may carry.
const JOB_MAX_CHARS: usize = 60;

/// One finished mission.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HighScore {
    /// The driver.
    pub name: String,
    pub job: String,
    pub score: u64,
    pub days: u32,
    pub survivors: usize,
    /// The run's seed, which keeps one run from being listed twice.
    pub run: u64,
}

impl HighScore {
    fn is_sound(&self) -> bool {
        self.score > 0
            && !self.name.trim().is_empty()
            && self.name.chars().count() <= NAME_MAX_CHARS
            && self.job.chars().count() <= JOB_MAX_CHARS
            && (1..=MAX_PARTY).contains(&self.survivors)
    }
}

/// Never more than [`TOP_SCORES`] lines, best first; equal scores stay in the
/// order they were earned.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TopTen {
    entries: Vec<HighScore>,
}

impl TopTen {
    /// Read a stored list. Lines that make no sense are dropped, the rest kept.
    pub fn parse(text: &str) -> Self {
        let lines: Vec<serde_json::Value> = serde_json::from_str(text).unwrap_or_default();
        let mut list = TopTen::default();
        for line in lines {
            if let Ok(entry) = serde_json::from_value::<HighScore>(line) {
                list.add(entry);
            }
        }
        list
    }

    /// The text to store; [`TopTen::parse`] reads it back.
    pub fn to_text(&self) -> String {
        // Plain strings and numbers, so serialising cannot fail.
        serde_json::to_string(&self.entries).unwrap_or_else(|_| "[]".into())
    }

    pub fn entries(&self) -> &[HighScore] {
        &self.entries
    }

    /// Where a run stands, counting from 1, if it is listed.
    pub fn rank(&self, run: u64, score: u64) -> Option<usize> {
        let found = self
            .entries
            .iter()
            .position(|e| e.run == run && e.score == score);
        found.map(|i| i + 1)
    }

    /// List a finished run if it is good enough, and return its rank. Adding
    /// the same run again changes nothing.
    pub fn add(&mut self, entry: HighScore) -> Option<usize> {
        if !entry.is_sound() {
            return None;
        }
        if let Some(rank) = self.rank(entry.run, entry.score) {
            return Some(rank);
        }
        let at = self.entries.partition_point(|e| e.score >= entry.score);
        if at >= TOP_SCORES {
            return None;
        }
        self.entries.insert(at, entry);
        self.entries.truncate(TOP_SCORES);
        Some(at + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(run: u64, score: u64) -> HighScore {
        HighScore {
            name: format!("Driver {run}"),
            job: "Rodeo Clown".into(),
            score,
            days: 5,
            survivors: 4,
            run,
        }
    }

    fn scores(list: &TopTen) -> Vec<u64> {
        list.entries().iter().map(|e| e.score).collect()
    }

    #[test]
    fn keeps_the_best_ten_in_order() {
        let mut list = TopTen::default();
        for run in 1..=25 {
            // A scrambled order of 25 distinct scores.
            list.add(entry(run, (run * 7) % 26 * 100));
            assert!(list.entries().len() <= TOP_SCORES);
            assert!(list.entries().windows(2).all(|w| w[0].score >= w[1].score));
        }
        let expected: Vec<u64> = (16..=25).rev().map(|n| n * 100).collect();
        assert_eq!(scores(&list), expected);
        // Too low to place, just good enough, and the new best.
        assert_eq!(list.add(entry(100, 100)), None);
        assert_eq!(list.add(entry(101, 1650)), Some(10));
        assert_eq!(list.add(entry(102, 9000)), Some(1));
        assert_eq!(list.entries().len(), TOP_SCORES);
    }

    #[test]
    fn ties_favour_whoever_got_there_first() {
        let mut list = TopTen::default();
        assert_eq!(list.add(entry(1, 500)), Some(1));
        assert_eq!(list.add(entry(2, 500)), Some(2));
        for run in 3..=10 {
            list.add(entry(run, 500));
        }
        assert_eq!(list.add(entry(11, 500)), None, "a full list of equals");
        assert_eq!(list.entries()[0].run, 1);
    }

    #[test]
    fn one_run_is_listed_once() {
        let mut list = TopTen::default();
        assert_eq!(list.add(entry(1, 500)), Some(1));
        assert_eq!(list.add(entry(2, 900)), Some(1));
        assert_eq!(list.add(entry(1, 500)), Some(2));
        assert_eq!(list.entries().len(), 2);
    }

    #[test]
    fn survives_the_trip_through_storage() {
        let mut list = TopTen::default();
        list.add(entry(u64::MAX, 700));
        list.add(entry(2, 300));
        assert_eq!(TopTen::parse(&list.to_text()), list);
    }

    #[test]
    fn junk_in_storage_costs_only_the_junk() {
        for junk in ["", "nonsense", "{}", "null", "17", "[1,2,3]"] {
            assert_eq!(TopTen::parse(junk), TopTen::default(), "{junk}");
        }
        let good = serde_json::to_string(&entry(1, 500)).unwrap();
        let zero = serde_json::to_string(&entry(2, 0)).unwrap();
        let mut long = entry(3, 800);
        long.name = "x".repeat(500);
        let long = serde_json::to_string(&long).unwrap();
        let mixed = format!("[null, {good}, {zero}, {long}, {{\"name\": 4}}]");
        assert_eq!(scores(&TopTen::parse(&mixed)), [500]);
        // An over-long, unsorted list is cut and put in order.
        let many: Vec<HighScore> = (1..=40).map(|n| entry(n, n)).collect();
        let read = TopTen::parse(&serde_json::to_string(&many).unwrap());
        assert_eq!(scores(&read), (31..=40).rev().collect::<Vec<u64>>());
    }
}

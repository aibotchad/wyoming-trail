//! The top ten on screen, and where the browser keeps it. The list is per
//! browser: each phone or computer has its own.

use web_sys::Storage;

use crate::core::scores::TopTen;

use super::page::{Action, Block, Page, Tone};
use super::stats::thousands;
use super::App;

const SCORES_KEY: &str = "wyoming-trail.scores.v1";

/// Storage can be unavailable (private browsing).
fn storage() -> Option<Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

/// The stored list, or `None` if there is nowhere to read it from.
pub(super) fn load() -> Option<TopTen> {
    let text = storage()?.get_item(SCORES_KEY).ok()?;
    Some(TopTen::parse(&text.unwrap_or_default()))
}

impl App {
    /// If the run has just been completed, list it and store the list.
    pub(super) fn keep_score(&mut self) {
        let Some(entry) = self.game.high_score() else {
            return;
        };
        // Read afresh, so another tab's scores are not written over. With no
        // storage at all, `load` fails and the list in memory stands.
        if let Some(stored) = load() {
            self.scores = stored;
        }
        if self.scores.add(entry).is_none() {
            return;
        }
        let saved =
            storage().is_some_and(|s| s.set_item(SCORES_KEY, &self.scores.to_text()).is_ok());
        if !saved {
            self.warn("This browser won't store the top ten; it lasts until you close it.".into());
        }
    }

    /// Wipe the list, if the player is sure.
    pub(super) fn clear_scores(&mut self) {
        let sure = web_sys::window().is_some_and(|w| {
            w.confirm_with_message("Erase the whole top ten list?")
                .unwrap_or(false)
        });
        if sure {
            self.scores = TopTen::default();
            if let Some(s) = storage() {
                let _ = s.remove_item(SCORES_KEY);
            }
        }
    }

    pub(super) fn scores_page(&self) -> Page {
        let mut p = Page {
            id: "scores".into(),
            back: Some(Action::Back),
            ..Page::default()
        };
        p.head.push(Block::Text("TOP TEN".into(), Tone::Heading));
        p.head.push(Block::Text(
            "Finished missions on this device. Harder jobs score more.".into(),
            Tone::Dim,
        ));
        p.button("Back", "", Action::Back);
        if self.scores.entries().is_empty() {
            p.text(
                "Nobody yet. Finish a mission to get on the list.",
                Tone::Body,
            );
            return p;
        }
        for (i, e) in self.scores.entries().iter().enumerate() {
            // The run on screen, if it is listed, stands out.
            let this_run = e.run == self.game.state.seed && e.score == self.game.state.score.total;
            let tone = if this_run { Tone::Mission } else { Tone::Body };
            p.text(
                format!("{}. {}  —  {}", i + 1, e.name, thousands(e.score)),
                tone,
            );
            p.text(
                format!(
                    "{} · {} · {} survived",
                    e.job,
                    super::page::days(e.days),
                    e.survivors
                ),
                Tone::Dim,
            );
        }
        p.body.push(Block::Gap);
        p.button("Clear the list", "Asks first", Action::ClearScores);
        p
    }
}

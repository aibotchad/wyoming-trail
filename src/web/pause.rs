//! The pause menu: stop the clock, rest, change pace and rations, and find
//! the way to everything else. Also the page on how to play.

use crate::core::config::{LONG_REST, MEALS, MOTEL_COST, PACES};
use crate::core::state::Screen;

use super::page::{Action, Block, Page, Tone, View};
use super::App;

impl App {
    /// Is there a run on screen that can be paused?
    pub(super) fn can_pause(&self) -> bool {
        let screen = self.game.state.screen;
        match self.view {
            View::Game => !screen.is_setup() && !screen.is_over(),
            View::Gather => self.round.is_some(),
            _ => false,
        }
    }

    /// A gathering round is waiting behind the menu.
    fn mid_round(&self) -> bool {
        self.history.contains(&View::Gather)
    }

    pub(super) fn pause_page(&self) -> Page {
        let s = &self.game.state;
        let mut p = Page {
            id: "pause".into(),
            back: Some(Action::Back),
            ..Page::default()
        };
        p.head.push(Block::Text("PAUSED".into(), Tone::Heading));
        let here = match &s.en_route {
            Some(e) => self.game.content.map.node(&e.destination),
            None => self.game.node(),
        };
        let place = here.map_or("", |n| n.name.as_str());
        let road = if s.en_route.is_some() {
            "On the road to"
        } else {
            "At"
        };
        p.head.push(Block::Text(
            format!(
                "{road} {place} · day {} · {} days left",
                s.day,
                self.game.days_left()
            ),
            Tone::Dim,
        ));
        p.button("Resume", "Back to it", Action::Back);

        p.text("REST", Tone::Section);
        if self.mid_round() {
            p.text(
                "You're in the middle of an outing. Rest when you're done.",
                Tone::Dim,
            );
        } else if self.game.can_camp() {
            self.crew_rest(&mut p);
            p.button(
                self.game.ride_def().camp.clone(),
                self.camp_detail(),
                Action::Rest(false),
            );
            p.button(
                format!("Rest up {LONG_REST} days"),
                "Free. A proper recovery, but the clock runs.",
                Action::Camp(LONG_REST),
            );
            let motel = s.en_route.is_none() && self.game.node().is_some_and(|n| n.services.motel);
            if motel {
                let broke = s.resources.money < MOTEL_COST;
                let note = if broke { "  (can't afford)" } else { "" };
                p.button(
                    format!("Motel night — ${MOTEL_COST:.0}{note}"),
                    "Uses a day. Heals and rests the crew well.",
                    Action::Rest(true),
                );
            }
        } else {
            p.text(
                "No resting in the middle of this. Finish it first.",
                Tone::Dim,
            );
        }

        if !self.mid_round() && !matches!(s.screen, Screen::Event | Screen::Hitchhiker) {
            p.text("HABITS", Tone::Section);
            let pace = PACES[s.pace_index % PACES.len()];
            let meal = MEALS[s.meal_index % MEALS.len()];
            p.button(
                format!("Pace: {}", pace.name),
                format!("{} hours a day. {}", pace.hours, pace.note),
                Action::CyclePace,
            );
            p.button(
                format!("Rations: {}", meal.name),
                format!("{} per person a day. {}", meal.servings, meal.note),
                Action::CycleMeal,
            );
        }

        p.text("MORE", Tone::Section);
        p.button("Stats", "Bar graphs of the trip so far", Action::ShowStats);
        p.button("Map", "Look around", Action::ShowMap);
        p.button(
            "How to play",
            "Controls and what to watch",
            Action::ShowHelp,
        );
        p.button("Top ten", "The best finished missions", Action::ShowScores);
        let leaving = if self.mid_round() {
            "Your trip is saved; this outing is not"
        } else {
            "Your trip is saved"
        };
        p.button("Main menu", leaving, Action::MainMenu);
        p
    }

    /// How the crew is holding up, in a line each.
    fn crew_rest(&self, p: &mut Page) {
        for m in self.game.state.party.members.iter().filter(|m| m.in_trip()) {
            let tone = if m.health < 40.0 || m.fatigue > 70.0 {
                Tone::Warn
            } else {
                Tone::Dim
            };
            p.text(
                format!(
                    "{}: health {:.0}, tired {:.0}, mood {:.0}",
                    m.name, m.health, m.fatigue, m.mood
                ),
                tone,
            );
        }
    }

    pub(super) fn help_page(&self) -> Page {
        let mut p = Page {
            id: "help".into(),
            back: Some(Action::Back),
            ..Page::default()
        };
        p.head
            .push(Block::Text("HOW TO PLAY".into(), Tone::Heading));
        p.button("Back", "", Action::Back);
        let controls = if self.touch {
            "Tap to choose. Drag or flick to scroll. MENU pauses."
        } else {
            "Click, or use arrows, Enter and number keys. P pauses, M is the map, S the stats."
        };
        p.text(controls, Tone::Label);
        for line in [
            "Finish every goal on your list before the days run out.",
            "Watch food and fuel. Running dry costs a rescue, or ends the trip.",
            "Tired people lose health. Camp, take a motel room or ease the pace.",
            "Poaching fills the cooler fast, but a game warden can fine you heavily and \
             take your ammunition. Towns and parks are watched hardest.",
            "Foraging is quieter: know the death camas, and mind rangers near towns.",
            "In winter nothing grows, so foragers dive dumpsters behind stores.",
            "Odd jobs and selling wild food earn cash, and cash is what gets you out of trouble.",
        ] {
            p.text(line, Tone::Body);
        }
        p
    }
}

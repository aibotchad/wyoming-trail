//! What goes on the stats page. Words and numbers only; `charts` draws them.

use crate::core::game::GoalState;
use crate::core::party::PartyMember;
use crate::core::state::DaySample;

use super::page::{Action, Bar, Block, DayColumn, Page, Tone};
use super::App;

/// "1,455".
pub(super) fn thousands(whole: u64) -> String {
    let digits = whole.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// "$1,455".
fn dollars(amount: f64) -> String {
    format!("${}", thousands(amount.round().max(0.0) as u64))
}

fn bar(label: impl Into<String>, value: f64, max: f64, figure: impl Into<String>) -> Bar {
    Bar {
        label: label.into(),
        value,
        max,
        figure: figure.into(),
    }
}

impl App {
    pub(super) fn stats_page(&self) -> Page {
        let game = &self.game;
        let s = &game.state;
        let mut p = Page {
            id: "stats".into(),
            back: Some(Action::Back),
            ..Page::default()
        };
        p.head.push(Block::Text("TRIP STATS".into(), Tone::Heading));
        let job = game.job().map_or("", |j| j.name.as_str());
        p.head
            .push(Block::Text(format!("{job} · day {}", s.day), Tone::Dim));
        p.button("Back to the trip", "", Action::Back);

        let goals = game.goals();
        let met = goals.iter().filter(|g| g.1 == GoalState::Done).count();
        let total = game.job().map_or(0, |j| j.goal.days);
        let used = total.saturating_sub(game.days_left());
        let eats = game.daily_food().max(1);
        let food_days = f64::from(s.resources.food) / f64::from(eats);
        let left = f64::from(game.days_left());
        p.body.push(Block::Bars {
            title: "HOW IT'S GOING".into(),
            rows: vec![
                bar(
                    "Goals met",
                    met as f64,
                    goals.len() as f64,
                    format!("{met} of {}", goals.len()),
                ),
                bar(
                    "Days used",
                    f64::from(used),
                    f64::from(total),
                    format!("{used} of {total}"),
                ),
                bar(
                    game.ride_def().gauge.clone(),
                    s.truck.fuel_gallons,
                    s.truck.tank_gallons,
                    format!("{:.0} mi", s.truck.current_range_miles()),
                ),
                bar(
                    "Food",
                    food_days,
                    left.max(1.0),
                    format!("{food_days:.1} days"),
                ),
            ],
        });
        // Each measure of the crew, with the word for a worrying value.
        type Measure = fn(&PartyMember) -> (f64, &'static str);
        let measures: [(&str, Measure); 3] = [
            ("CREW HEALTH (of 100)", |m| {
                (m.health, if m.health < 40.0 { " low" } else { "" })
            }),
            ("TIREDNESS (lower is better)", |m| {
                (m.fatigue, if m.fatigue > 70.0 { " worn out" } else { "" })
            }),
            ("MOOD (of 100)", |m| {
                (m.mood, if m.mood < 30.0 { " low" } else { "" })
            }),
        ];
        for (title, read) in measures {
            let each = s.party.members.iter().map(|m| {
                if !m.in_trip() {
                    return bar(&m.name, 0.0, 100.0, "left");
                }
                let (value, note) = read(m);
                bar(&m.name, value, 100.0, format!("{value:.0}{note}"))
            });
            p.body.push(Block::Bars {
                title: title.into(),
                rows: each.collect(),
            });
        }
        let graph =
            |title: &str, value: fn(&DaySample) -> f64, show: fn(f64) -> String| Block::Columns {
                title: title.into(),
                columns: s
                    .history
                    .iter()
                    .map(|d| DayColumn {
                        day: d.day,
                        value: value(d),
                        figure: show(value(d)),
                    })
                    .collect(),
            };
        if s.history.is_empty() {
            p.text("The daily graphs start once you're on the road.", Tone::Dim);
        } else {
            let graphs = [
                graph(
                    "MILES DRIVEN EACH DAY",
                    |d| d.miles,
                    |v| format!("{v:.0} mi"),
                ),
                graph("CASH EACH DAY", |d| d.money, dollars),
                graph(
                    "FOOD IN STOCK EACH DAY",
                    |d| f64::from(d.food),
                    |v| format!("{v:.0}"),
                ),
            ];
            // Beside the summary where there's room, under it where there isn't.
            if self.width < super::draw::WIDE {
                p.body.extend(graphs);
            } else {
                p.side.extend(graphs);
            }
        }
        p.button("Back to the trip", "", Action::Back);
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_reads_with_thousands_separators() {
        assert_eq!(dollars(0.0), "$0");
        assert_eq!(dollars(999.4), "$999");
        assert_eq!(dollars(1454.91), "$1,455");
        assert_eq!(dollars(1_234_567.0), "$1,234,567");
    }
}

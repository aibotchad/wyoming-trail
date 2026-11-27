//! Spending money: the yes / no question before every purchase, and the cash
//! badge that stays in the top-right corner wherever things are for sale.

use crate::core::game::{Purchase, Quote};
use crate::core::state::Screen;

use super::draw::{font, BG, WIDE};
use super::page::{days as day_count, Action, Hotspot, View};
use super::ui::{PANEL, PARCHMENT, SLOT, TAN};
use super::App;

/// A "yes" this soon after the question appears is a double tap, not an answer.
const SETTLE_MS: f64 = 350.0;
/// Size of the cash badge.
pub(super) const BADGE: (f64, f64) = (190.0, 66.0);

pub(super) struct Confirm {
    /// What "yes" will do.
    action: Action,
    quote: Quote,
    /// Highlighted answer: 0 yes, 1 no.
    choice: usize,
    asked_at: f64,
}

fn purchase_of(action: &Action) -> Option<Purchase> {
    match action {
        Action::BuyTruck(i) => Some(Purchase::Truck(*i)),
        Action::Buy(id) => Some(Purchase::Item(id.clone())),
        Action::FillTank => Some(Purchase::Fuel),
        Action::Rest(true) => Some(Purchase::Motel),
        // Free, but it spends a day, so it asks too.
        Action::Rest(false) => Some(Purchase::Camp(1)),
        Action::Camp(nights) => Some(Purchase::Camp(*nights)),
        Action::SellFood(n) => Some(Purchase::SellFood(*n)),
        Action::SellItem(id) => Some(Purchase::SellItem(id.clone())),
        Action::Work(id) => Some(Purchase::Work(id.clone())),
        _ => None,
    }
}

impl App {
    /// Handle the confirmation round-trip. Returns the action to carry out
    /// now, or `None` if there is nothing more to do.
    pub(super) fn confirm_step(&mut self, action: Action) -> Option<Action> {
        match action {
            Action::ConfirmNo => {
                self.confirm = None;
                None
            }
            Action::ConfirmYes => {
                let ready = self
                    .confirm
                    .as_ref()
                    .is_some_and(|c| js_sys::Date::now() - c.asked_at >= SETTLE_MS);
                if !ready {
                    return None;
                }
                self.confirm.take().map(|c| c.action)
            }
            other => {
                let Some(purchase) = purchase_of(&other) else {
                    return Some(other);
                };
                match self.game.quote(&purchase) {
                    Ok(quote) => {
                        self.toast = None;
                        self.confirm = Some(Confirm {
                            action: other,
                            quote,
                            choice: 1,
                            asked_at: js_sys::Date::now(),
                        });
                    }
                    Err(why) => self.warn(why),
                }
                None
            }
        }
    }

    pub(super) fn confirm_key(&mut self, key: &str) {
        let answer = match key {
            "y" | "Y" | "1" => Some(Action::ConfirmYes),
            "n" | "N" | "2" | "Escape" => Some(Action::ConfirmNo),
            "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown" | "Tab" => {
                if let Some(c) = self.confirm.as_mut() {
                    c.choice = 1 - c.choice;
                }
                None
            }
            "Enter" | " " => self.confirm.as_ref().map(|c| {
                if c.choice == 0 {
                    Action::ConfirmYes
                } else {
                    Action::ConfirmNo
                }
            }),
            _ => None,
        };
        if let Some(action) = answer {
            self.perform(action);
        }
    }

    /// Is this a screen where money can be spent?
    pub(super) fn shopping(&self) -> bool {
        self.view == View::Game
            && self.drive.is_none()
            && matches!(
                self.game.state.screen,
                Screen::Outfit
                    | Screen::Store
                    | Screen::Trade
                    | Screen::Work
                    | Screen::Town
                    | Screen::RoadChoice
            )
    }

    /// Where the cash badge sits, as (x, y, w, h).
    pub(super) fn badge_rect(&self) -> (f64, f64, f64, f64) {
        (self.width - BADGE.0 - 10.0, 8.0, BADGE.0, BADGE.1)
    }

    /// On a narrow screen the badge has a solid strip to itself, which the
    /// page scrolls under: this is the strip's height.
    pub(super) fn badge_strip(&self) -> f64 {
        if self.width < WIDE {
            BADGE.1 + 14.0
        } else {
            0.0
        }
    }

    /// The money in hand, fixed to the top-right corner.
    pub(super) fn cash_badge(&mut self) {
        let (x, y, w, h) = self.badge_rect();
        if self.badge_strip() > 0.0 {
            self.ctx.set_fill_style_str(BG);
            self.ctx
                .fill_rect(0.0, 0.0, self.width - 8.0, self.badge_strip());
        }
        self.nine(PARCHMENT, (x, y, w, h), true);
        self.ctx.set_fill_style_str("#7a4a24");
        self.ctx.set_font(&font(18.0));
        let _ = self.ctx.fill_text("YOUR CASH", x + 14.0, y + 23.0);
        self.ctx.set_fill_style_str("#2a1608");
        self.ctx.set_font(&font(34.0));
        let money = format!("${:.2}", self.game.state.resources.money);
        let _ = self
            .ctx
            .fill_text_with_max_width(&money, x + 14.0, y + 54.0, w - 26.0);
    }

    /// The yes / no panel over a dimmed page. It takes over all clicks.
    pub(super) fn confirm_box(&mut self) {
        let Some((quote, choice)) = self.confirm.as_ref().map(|c| (c.quote.clone(), c.choice))
        else {
            return;
        };
        let Quote { what, cost, days } = quote;
        // Sleeping, buying, selling or hiring on.
        let (title, question, price, yes) = if cost == 0.0 && days > 0 {
            let question = if days > 1 {
                format!("Camp and rest up for {}?", day_count(days))
            } else {
                self.game.ride_def().camp_ask.clone()
            };
            (
                if days > 1 {
                    "REST UP?"
                } else {
                    "SPEND THE NIGHT?"
                },
                question,
                "Free".to_string(),
                "Yes, rest",
            )
        } else if cost >= 0.0 {
            (
                "CONFIRM PURCHASE",
                format!("Buy {what}?"),
                format!("Price: ${cost:.2}"),
                "Yes, buy it",
            )
        } else if days == 0 {
            (
                "CONFIRM SALE",
                format!("Sell {what}?"),
                format!("You get: ${:.2}", -cost),
                "Yes, sell it",
            )
        } else {
            (
                "TAKE THE JOB?",
                format!("{what}: {} of your life.", day_count(days)),
                format!("Pays: ${:.2}", -cost),
                "Yes, clock in",
            )
        };
        self.layout.hotspots.clear();
        self.ctx.set_fill_style_str("rgba(8,6,4,0.72)");
        self.ctx.fill_rect(0.0, 0.0, self.width, self.height);

        let w = (self.width - 24.0).min(520.0);
        let inner = w - 56.0;
        self.ctx.set_font(&font(28.0));
        let lines = self.wrap(&question, inner);
        let money = self.game.state.resources.money;
        // Anything that takes days says what that leaves for the mission.
        let left = self.game.days_left().saturating_sub(days);
        let time = (days > 0).then(|| match left {
            0 => "No days left after: you would fail!".to_string(),
            _ => format!("Takes {}. Days left after: {left}", day_count(days)),
        });
        let extra = if time.is_some() && cost != 0.0 {
            30.0
        } else {
            0.0
        };
        let h = 214.0 + extra + 32.0 * lines.len() as f64;
        // Below the badge if there is room, and never off the bottom.
        let y = ((self.height - h) / 2.0)
            .max(BADGE.1 + 12.0)
            .min(self.height - h - 4.0)
            .max(0.0);
        let x = (self.width - w) / 2.0;
        self.nine(PANEL, (x, y, w, h), true);

        let mut line_y = y + 40.0;
        let text = |app: &App, s: &str, size: f64, colour: &str, at: f64| {
            app.ctx.set_font(&font(size));
            app.ctx.set_fill_style_str(colour);
            let _ = app.ctx.fill_text_with_max_width(s, x + 28.0, at, inner);
        };
        text(self, title, 22.0, "#e8b04a", line_y);
        line_y += 36.0;
        for l in lines.iter() {
            text(self, l, 28.0, "#ede6d6", line_y);
            line_y += 32.0;
        }
        text(self, &price, 26.0, "#ff8ad0", line_y + 4.0);
        let after = format!("Cash after: ${:.2}", (money - cost).max(0.0));
        // Sleeping costs nothing, so there is no cash line to show.
        if cost != 0.0 {
            text(self, &after, 22.0, "#b0a48c", line_y + 32.0);
        }
        if let Some(time) = &time {
            let colour = if left <= 1 { "#f08a6a" } else { "#e8b04a" };
            let at = if cost != 0.0 { 62.0 } else { 34.0 };
            text(self, time, 22.0, colour, line_y + at);
        }

        let button_w = (inner - 16.0) / 2.0;
        let button_y = y + h - 74.0;
        let answers = [(yes, Action::ConfirmYes), ("No", Action::ConfirmNo)];
        for (i, (label, action)) in answers.into_iter().enumerate() {
            let bx = x + 28.0 + i as f64 * (button_w + 16.0);
            let hovered = self.pointer.is_some_and(|(px, py)| {
                (bx..=bx + button_w).contains(&px) && (button_y..=button_y + 52.0).contains(&py)
            });
            // The highlight is the keyboard's cursor. On a touch screen it
            // would look like an answer already picked.
            let lit = hovered || (!self.touch && choice == i);
            let skin = if lit {
                PARCHMENT
            } else if self.touch && i == 0 {
                TAN
            } else {
                SLOT
            };
            self.nine(skin, (bx, button_y, button_w, 52.0), true);
            let lit = lit || (self.touch && i == 0);
            self.ctx.set_font(&font(26.0));
            self.ctx
                .set_fill_style_str(if lit { "#2a1608" } else { "#ede6d6" });
            let tw = self
                .ctx
                .measure_text(label)
                .map(|m| m.width())
                .unwrap_or(0.0);
            let _ = self.ctx.fill_text_with_max_width(
                label,
                bx + ((button_w - tw) / 2.0).max(8.0),
                button_y + 34.0,
                button_w - 16.0,
            );
            self.layout.hotspots.push(Hotspot {
                x: bx,
                // Hotspots are kept in scrolled page coordinates.
                y: button_y + self.scroll,
                w: button_w,
                h: 52.0,
                button: None,
                action,
            });
        }
    }
}

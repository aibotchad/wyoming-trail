//! Cards: buttons with enough to say that it needs setting out, under bold
//! headings.

use super::draw::{line_height, Column, ACCENT, BLOCK_GAP, TEXT};
use super::page::{Action, Hotspot};
use super::ui::{PARCHMENT, SLOT, TAN};
use super::App;

const TITLE: f64 = 29.0;
const SUMMARY: f64 = 22.0;
const HEADER: f64 = 19.0;
const BODY: f64 = 21.0;
/// Space inside the card's edges, and between sections.
const PAD: f64 = 18.0;
const BETWEEN: f64 = 9.0;

/// What a card says.
pub(super) struct Card<'a> {
    pub title: &'a str,
    pub summary: &'a str,
    pub sections: &'a [(&'static str, String)],
}

/// Colours for the title, headings, text and rules.
struct Inks {
    title: &'static str,
    header: &'static str,
    body: &'static str,
    rule: &'static str,
}

impl App {
    pub(super) fn card(
        &mut self,
        card: &Card,
        action: &Action,
        index: usize,
        col: &mut Column,
        view_height: f64,
    ) {
        // Room for the number key, where there is a keyboard.
        let indent = if self.touch { PAD } else { 40.0 };
        let inner = col.w - indent - PAD;
        let height = |app: &App, text: &str, size: f64| app.text_height(text, size, inner);
        let mut h =
            PAD * 2.0 + height(self, card.title, TITLE) + height(self, card.summary, SUMMARY);
        for (header, body) in card.sections {
            h += BETWEEN * 2.0 + height(self, header, HEADER) + height(self, body, BODY);
        }

        let top = col.y - self.scroll;
        let hovered = self.pointer.is_some_and(|(px, py)| {
            px >= col.x && px <= col.x + col.w && py >= top && py <= top + h && py < view_height
        });
        // The selection is the keyboard's cursor.
        let selected = !self.touch && self.selected == index;
        let skin = if selected {
            PARCHMENT
        } else if hovered {
            TAN
        } else {
            SLOT
        };
        self.nine(skin, (col.x, top, col.w, h), true);
        let inks = if selected || hovered {
            Inks {
                title: "#2a1608",
                header: "#7a3c10",
                body: "#3a2010",
                rule: "#b08a5c",
            }
        } else {
            Inks {
                title: "#ffffff",
                header: ACCENT,
                body: TEXT,
                rule: "#7a5436",
            }
        };
        if index < 9 && !self.touch {
            let number = (index + 1).to_string();
            self.bold(&number, col.x + 15.0, col.y + PAD, 20.0, TITLE, inks.header);
        }
        let x = col.x + indent;
        let mut y = col.y + PAD;
        y += self.bold(card.title, x, y, inner, TITLE, inks.title);
        y += self.regular(card.summary, x, y, inner, SUMMARY, inks.body);
        for (header, body) in card.sections {
            y += BETWEEN;
            let line = y - self.scroll - 2.0;
            if self.on_screen(line, 2.0) {
                self.ctx.set_fill_style_str(inks.rule);
                self.ctx.fill_rect(x, line, inner, 2.0);
            }
            y += BETWEEN;
            y += self.bold(header, x, y, inner, HEADER, inks.header);
            y += self.regular(body, x, y, inner, BODY, inks.body);
        }
        self.layout.hotspots.push(Hotspot {
            x: col.x,
            y: col.y,
            w: col.w,
            h,
            button: Some(index),
            action: action.clone(),
        });
        col.y += h + BLOCK_GAP + 4.0;
    }

    fn text_height(&self, text: &str, size: f64, w: f64) -> f64 {
        self.ctx.set_font(&super::draw::font(size));
        line_height(size) * self.wrap(text, w - 2.0).len() as f64
    }

    fn regular(&mut self, text: &str, x: f64, y: f64, w: f64, size: f64, ink: &str) -> f64 {
        self.ctx.set_font(&super::draw::font(size));
        self.paragraph(text, x, y, w - 2.0, size, ink)
    }

    /// The pixel font comes in one weight, so bold is the text struck twice, a
    /// pixel or so apart. Returns the height used.
    pub(super) fn bold(&mut self, text: &str, x: f64, y: f64, w: f64, size: f64, ink: &str) -> f64 {
        let shift = if size < 26.0 { 1.0 } else { 1.5 };
        self.regular(text, x, y, w - shift, size, ink);
        self.regular(text, x + shift, y, w - shift, size, ink)
    }
}

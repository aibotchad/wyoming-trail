//! The short message popped up over the bottom of the page, so that feedback
//! is seen however far the page is scrolled.

use super::draw::font;
use super::ui::PANEL;
use super::App;

/// Time on screen: a base, plus reading time for each character, capped.
const BASE_MS: f64 = 2_500.0;
const PER_CHAR_MS: f64 = 45.0;
const LONGEST_MS: f64 = 9_000.0;

pub(super) struct Toast {
    text: String,
    /// Something went wrong, as opposed to news.
    warning: bool,
    until: f64,
}

fn lifetime(text: &str) -> f64 {
    (BASE_MS + PER_CHAR_MS * text.chars().count() as f64).min(LONGEST_MS)
}

impl Toast {
    pub fn expired(&self) -> bool {
        js_sys::Date::now() >= self.until
    }
}

impl App {
    fn pop_up(&mut self, text: String, warning: bool) {
        let until = js_sys::Date::now() + lifetime(&text);
        self.toast = Some(Toast {
            text,
            warning,
            until,
        });
    }

    /// News about what just happened.
    pub(super) fn say(&mut self, text: String) {
        if !text.is_empty() {
            self.pop_up(text, false);
        }
    }

    /// Why something didn't happen.
    pub(super) fn warn(&mut self, why: String) {
        self.pop_up(why, true);
    }

    pub(super) fn on_toast(&self, px: f64, py: f64) -> bool {
        self.layout
            .toast
            .is_some_and(|(x, y, w, h)| (x..=x + w).contains(&px) && (y..=y + h).contains(&py))
    }

    /// Paint the message just above the status bar.
    pub(super) fn toast_box(&mut self) {
        let Some((text, warning)) = self.toast.as_ref().map(|t| (t.text.clone(), t.warning)) else {
            return;
        };
        let w = (self.width - 20.0).min(560.0);
        self.ctx.set_font(&font(22.0));
        let lines = self.wrap(&text, w - 40.0);
        let h = 28.0 + 25.0 * lines.len() as f64;
        let x = (self.width - w) / 2.0;
        let y = (self.layout.view_height - h - 8.0).max(0.0);
        self.nine(PANEL, (x, y, w, h), true);
        self.ctx.set_font(&font(22.0));
        self.ctx
            .set_fill_style_str(if warning { "#ff9a78" } else { "#ede6d6" });
        for (i, line) in lines.iter().enumerate() {
            let at = y + 33.0 + 25.0 * i as f64;
            let _ = self
                .ctx
                .fill_text_with_max_width(line, x + 20.0, at, w - 40.0);
        }
        self.layout.toast = Some((x, y, w, h));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn longer_messages_stay_up_longer_but_not_forever() {
        assert!(lifetime("You're broke.") > BASE_MS);
        assert!(lifetime(&"x".repeat(60)) > lifetime("x"));
        assert_eq!(lifetime(&"x".repeat(1000)), LONGEST_MS);
    }
}

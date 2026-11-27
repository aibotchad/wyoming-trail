//! Canvas painting: flows a [`Page`](super::page::Page) into one or two
//! columns, wraps text, draws photos and the map, and records hotspots.

use std::rc::Rc;

use web_sys::HtmlImageElement;

use crate::core::game::GoalState;

use super::cards::Card;
use super::page::{Block, Hotspot, Layout, Tone, View};
use super::shop::BADGE;
use super::ui::{PANEL, PARCHMENT, SLOT, TAN};
use super::App;

/// Family declared in `index.html`.
pub(super) const FONT: &str = "Jersey15";

pub(super) const BG: &str = "#14110d";
const ROW: &str = "#262017";
const ROW_HOVER: &str = "#3a3022";
pub(super) const ACCENT: &str = "#e8b04a";
pub(super) const TEXT: &str = "#ede6d6";
pub(super) const DIM: &str = "#b0a48c";
const JOKE: &str = "#a9d18e";
const WARN: &str = "#f08a6a";
pub(super) const MISSION: &str = "#ff8ad0";

/// Below this width everything stacks in one column.
pub(super) const WIDE: f64 = 880.0;
const MAX_WIDTH: f64 = 1400.0;
pub(super) const BLOCK_GAP: f64 = 8.0;
/// Things this far beyond the window are still drawn, for frames that stick out.
const CULL: f64 = 24.0;
/// Wrapped paragraphs remembered before the cache is emptied.
const WRAP_CACHE: usize = 1_500;

/// A column being filled top to bottom.
#[derive(Clone, Copy)]
pub(super) struct Column {
    pub(super) x: f64,
    pub(super) y: f64,
    pub(super) w: f64,
}

/// A map node under the pointer, drawn last so it sits on top.
pub(super) struct Tooltip {
    pub(super) id: String,
    pub(super) x: f64,
    pub(super) y: f64,
}

pub(super) fn line_height(size: f64) -> f64 {
    (size * 1.12).round()
}

pub(super) fn font(px: f64) -> String {
    // The bundled pixel font; the fallbacks cover the odd missing symbol.
    format!("{px}px {FONT}, 'DejaVu Sans Mono', monospace")
}

impl App {
    fn status_lines(&self) -> Vec<String> {
        let s = &self.game.state;
        if self.view != View::Game || s.screen.is_setup() {
            return Vec::new();
        }
        let (month, date) = self.game.date(s.day);
        let left = self.game.days_left();
        let wide = self.width >= WIDE;
        let goals = self.game.goals();
        let met = goals.iter().filter(|g| g.1 == GoalState::Done).count();
        let goals = format!("Goals {met}/{}", goals.len());
        let when = if wide {
            format!(
                "Day {} ({month}/{date})  ·  {left} days left  ·  {goals}",
                s.day
            )
        } else {
            format!("{goals}  ·  {left} days")
        };
        let a = format!(
            "{when}  ·  ${:.0}  ·  {} food",
            s.resources.money, s.resources.food
        );
        let ride = self.game.ride_def();
        let mut fuel = ride.fuel.clone();
        if let Some(first) = fuel.get_mut(0..1) {
            first.make_ascii_uppercase();
        }
        let gas = format!(
            "{fuel} {:.1}/{} {} ({:.0} mi)",
            s.truck.fuel_gallons,
            s.truck.tank_gallons,
            ride.unit,
            s.truck.current_range_miles()
        );
        let spares = format!("{} {}(s)", self.game.fuel_cans(), ride.spare);
        let b = if wide {
            format!("{gas}  ·  {spares}  ·  {} aboard", s.party.in_trip_count())
        } else {
            format!("{gas}  ·  {spares}")
        };
        if wide {
            vec![format!("{a}  ·  {b}")]
        } else {
            vec![a, b]
        }
    }

    pub(super) fn render(&mut self) {
        let status = self.status_lines();
        let bar = if status.is_empty() {
            0.0
        } else if self.width < WIDE {
            // Tall enough for two buttons, one over the other.
            (24.0 + 26.0 * status.len() as f64).max(92.0)
        } else {
            24.0 + 26.0 * status.len() as f64
        };
        let view_height = self.height - bar;

        // Lay out and paint in one pass; if the scroll turned out to be out of
        // range (content shrank), clamp it and paint again.
        let content = self.paint(view_height);
        let max_scroll = (content - view_height).max(0.0);
        if self.scroll < 0.0 || self.scroll > max_scroll {
            self.scroll = self.scroll.clamp(0.0, max_scroll);
            self.paint(view_height);
        }

        if max_scroll > 0.0 {
            let thumb = (view_height * view_height / content).max(30.0);
            let top = (view_height - thumb) * self.scroll / max_scroll;
            self.ctx.set_fill_style_str(ROW_HOVER);
            self.ctx.fill_rect(self.width - 5.0, 0.0, 4.0, view_height);
            // Bright enough to say there is more to scroll to.
            self.ctx.set_fill_style_str(DIM);
            self.ctx.fill_rect(self.width - 5.0, top, 4.0, thumb);
        }
        if !status.is_empty() {
            self.nine(PANEL, (0.0, view_height, self.width, bar), true);
            let text_right = self.bar_buttons(view_height, bar);
            let size = if self.width < WIDE { 21.0 } else { 23.0 };
            self.ctx.set_font(&font(size));
            self.ctx.set_fill_style_str(ACCENT);
            for (i, line) in status.iter().enumerate() {
                let y = view_height + 33.0 + 26.0 * i as f64;
                let _ = self
                    .ctx
                    .fill_text_with_max_width(line, 18.0, y, text_right - 26.0);
            }
        }
        if self.shopping() {
            self.cash_badge();
        }
        if self.confirm.is_none() {
            self.toast_box();
        }
        // Last, so nothing is painted over its buttons.
        self.confirm_box();
    }

    /// The ways to the menu and the graphs that are always in reach, at the
    /// right of the status bar. Returns where they begin.
    fn bar_buttons(&mut self, top: f64, bar: f64) -> f64 {
        let (menu, stats) = if self.width < WIDE {
            let (w, h) = (74.0, (bar - 22.0) / 2.0);
            let x = self.width - w - 12.0;
            ((x, top + 9.0, w, h), (x, top + 13.0 + h, w, h))
        } else {
            let (w, h) = (96.0, bar - 20.0);
            let x = self.width - w - 14.0;
            ((x, top + 10.0, w, h), (x - w - 8.0, top + 10.0, w, h))
        };
        let can_pause = self.can_pause();
        if can_pause {
            self.chip("MENU", menu);
            self.layout.menu_button = Some(menu);
        }
        self.chip("STATS", stats);
        self.layout.stats_button = Some(stats);
        stats.0.min(menu.0)
    }

    fn chip(&mut self, label: &str, area: (f64, f64, f64, f64)) {
        let (x, y, w, h) = area;
        self.nine(TAN, area, true);
        self.ctx.set_font(&font(22.0));
        self.ctx.set_fill_style_str("#2a1608");
        self.ctx.set_text_align("center");
        let _ = self
            .ctx
            .fill_text_with_max_width(label, x + w / 2.0, y + h / 2.0 + 7.0, w - 8.0);
        self.ctx.set_text_align("left");
    }

    /// Paint the page at the current scroll; returns the content height.
    fn paint(&mut self, view_height: f64) -> f64 {
        self.ctx.set_fill_style_str(BG);
        self.ctx.fill_rect(0.0, 0.0, self.width, self.height);
        self.layout = Layout {
            hotspots: Vec::new(),
            view_height,
            field: None,
            toast: None,
            animated: false,
            stats_button: None,
            menu_button: None,
        };

        // Borrowed out of the app while it is painted; put back below.
        let page = std::mem::take(&mut self.page);
        let wide = self.width >= WIDE;
        let margin = if wide { 28.0 } else { 14.0 };
        let total = (self.width - margin * 2.0).min(MAX_WIDTH);
        let left = (self.width - total) / 2.0;
        let mut button = 0;
        let mut tooltip = None;

        // Keep the top-right corner free for the cash badge.
        let (badge_w, badge_h) = if self.shopping() { BADGE } else { (0.0, 0.0) };
        let mut head = Column {
            x: left,
            y: if wide {
                margin
            } else {
                margin.max(badge_h + 16.0)
            },
            w: if wide && badge_w > 0.0 {
                total - badge_w - 12.0
            } else {
                total
            },
        };
        for b in &page.head {
            self.block(b, &mut head, view_height, &mut button, &mut tooltip);
        }
        // Columns below start clear of the badge.
        head.y = (head.y + BLOCK_GAP).max(if badge_h > 0.0 { badge_h + 20.0 } else { 0.0 });

        let bottom = if wide && !page.side.is_empty() {
            // The map screen gives nearly everything to the map.
            let share = if self.view == View::Map { 0.24 } else { 0.5 };
            let gutter = 28.0;
            let body_w = (total - gutter) * share;
            let mut body = Column {
                x: left,
                y: head.y,
                w: body_w,
            };
            let mut side = Column {
                x: left + body_w + gutter,
                y: head.y,
                w: total - gutter - body_w,
            };
            for b in &page.body {
                self.block(b, &mut body, view_height, &mut button, &mut tooltip);
            }
            for b in &page.side {
                self.block(b, &mut side, view_height, &mut button, &mut tooltip);
            }
            body.y.max(side.y)
        } else {
            // Photos first, then the choices, then the (tall) map.
            let (maps, photos): (Vec<&Block>, Vec<&Block>) =
                page.side.iter().partition(|b| matches!(b, Block::Map));
            let mut col = Column { w: total, ..head };
            for b in photos.into_iter().chain(&page.body).chain(maps) {
                self.block(b, &mut col, view_height, &mut button, &mut tooltip);
            }
            col.y
        };

        self.page = page;
        if let Some(t) = tooltip {
            self.tooltip(&t);
        }
        bottom + margin
    }

    /// Does any of the screen span from `top`, `h` tall, show in the window?
    pub(super) fn on_screen(&self, top: f64, h: f64) -> bool {
        top + h >= -CULL && top <= self.height + CULL
    }

    /// Break `text` into lines no wider than `w` in the current font. Asking
    /// the browser to measure words is slow, so answers are remembered.
    pub(super) fn wrap(&self, text: &str, w: f64) -> Rc<[String]> {
        let key = format!("{}|{w:.1}|{text}", self.ctx.font());
        if let Some(lines) = self.wrapped.borrow().get(&key) {
            return lines.clone();
        }
        let lines: Rc<[String]> = self.measure_lines(text, w).into();
        let mut cache = self.wrapped.borrow_mut();
        if cache.len() >= WRAP_CACHE {
            cache.clear();
        }
        cache.insert(key, lines.clone());
        lines
    }

    fn measure_lines(&self, text: &str, w: f64) -> Vec<String> {
        let width = |s: &str| self.ctx.measure_text(s).map(|t| t.width()).unwrap_or(0.0);
        let mut lines = Vec::new();
        let mut line = String::new();
        for word in text.split(' ').flat_map(|w| w.split_inclusive('\n')) {
            let breaks = word.ends_with('\n');
            let word = word.trim();
            if word.is_empty() {
                continue;
            }
            let joined = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if line.is_empty() || width(&joined) <= w {
                line = joined;
            } else {
                lines.push(std::mem::replace(&mut line, word.to_string()));
            }
            if breaks {
                lines.push(std::mem::take(&mut line));
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
        lines
    }

    /// Draw wrapped text with its top at content position `y`; returns height.
    pub(super) fn paragraph(
        &self,
        text: &str,
        x: f64,
        y: f64,
        w: f64,
        size: f64,
        color: &str,
    ) -> f64 {
        let line_h = line_height(size);
        self.ctx.set_fill_style_str(color);
        let lines = self.wrap(text, w);
        for (i, line) in lines.iter().enumerate() {
            let baseline = y - self.scroll + line_h * i as f64 + (size * 0.85).round();
            if self.on_screen(baseline - size, size * 1.4) {
                let _ = self.ctx.fill_text_with_max_width(line, x, baseline, w);
            }
        }
        line_h * lines.len() as f64
    }

    fn block(
        &mut self,
        block: &Block,
        col: &mut Column,
        view_height: f64,
        button: &mut usize,
        tooltip: &mut Option<Tooltip>,
    ) {
        let narrow = self.width < WIDE;
        match block {
            Block::Gap => col.y += 10.0,
            Block::Card {
                title,
                summary,
                sections,
                action,
            } => {
                let index = *button;
                *button += 1;
                let card = Card {
                    title,
                    summary,
                    sections,
                };
                self.card(&card, action, index, col, view_height);
            }
            Block::Text(text, tone) => {
                let (size, color) = match tone {
                    Tone::Banner => (if narrow { 32.0 } else { 46.0 }, WARN),
                    Tone::Heading => (if narrow { 32.0 } else { 40.0 }, ACCENT),
                    Tone::Body => (23.0, TEXT),
                    Tone::Dim => (21.0, DIM),
                    Tone::Label => (21.0, ACCENT),
                    Tone::Joke => (23.0, JOKE),
                    Tone::Mission => (25.0, MISSION),
                    Tone::GoalMet => (20.0, JOKE),
                    Tone::GoalLater => (20.0, DIM),
                    Tone::Section => (if narrow { 30.0 } else { 34.0 }, ACCENT),
                    Tone::Warn => (23.0, WARN),
                };
                if matches!(tone, Tone::Section) {
                    col.y += 8.0;
                }
                // Titles are bold; running text stays crisp.
                self.ctx.set_font(&font(size));
                col.y += match tone {
                    Tone::Banner | Tone::Heading | Tone::Label | Tone::Section => {
                        self.bold(text, col.x, col.y, col.w, size, color)
                    }
                    _ => self.paragraph(text, col.x, col.y, col.w, size, color),
                };
                col.y += if matches!(tone, Tone::Banner) {
                    14.0
                } else {
                    4.0
                };
            }
            Block::Button {
                label,
                detail,
                action,
            } => {
                let index = *button;
                *button += 1;
                // Number keys are no use to a finger, so no room is kept for
                // them there.
                let indent = if self.touch { 18.0 } else { 40.0 };
                let inner = col.w - indent - 16.0;
                self.ctx.set_font(&font(26.0));
                let label_h = line_height(26.0) * self.wrap(label, inner).len() as f64;
                self.ctx.set_font(&font(20.0));
                let detail_h = line_height(20.0) * self.wrap(detail, inner).len() as f64;
                let h = (label_h + detail_h + 22.0).max(52.0);

                let screen_y = col.y - self.scroll;
                let hovered = self.pointer.is_some_and(|(px, py)| {
                    px >= col.x
                        && px <= col.x + col.w
                        && py >= screen_y
                        && py <= screen_y + h
                        && py < view_height
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
                self.nine(skin, (col.x, screen_y, col.w, h), true);
                // Dark ink on the light skins, light on the dark one.
                let (ink, faded, digit) = if selected || hovered {
                    ("#3a2010", "#5c3c26", "#8a4a1c")
                } else {
                    (TEXT, DIM, ACCENT)
                };
                self.ctx.set_font(&font(22.0));
                self.ctx.set_fill_style_str(digit);
                // Only the first nine have a number key.
                if index < 9 && !self.touch {
                    let number = (index + 1).to_string();
                    let _ = self.ctx.fill_text(&number, col.x + 15.0, screen_y + 31.0);
                }

                let top = col.y + if detail.is_empty() { 12.0 } else { 9.0 };
                self.ctx.set_font(&font(26.0));
                self.paragraph(label, col.x + indent, top, inner, 26.0, ink);
                self.ctx.set_font(&font(20.0));
                self.paragraph(
                    detail,
                    col.x + indent,
                    top + label_h + 2.0,
                    inner,
                    20.0,
                    faded,
                );

                self.layout.hotspots.push(Hotspot {
                    x: col.x,
                    y: col.y,
                    w: col.w,
                    h,
                    button: Some(index),
                    action: action.clone(),
                });
                col.y += h + BLOCK_GAP;
            }
            Block::Photo(id) => self.photo(id, col, view_height),
            Block::Finale(scene) => self.finale(scene, col, view_height),
            Block::Bars { title, rows } => self.bars(title, rows, col),
            Block::Columns { title, columns } => self.columns(title, columns, col),
            Block::Map => self.map(col, view_height, tooltip),
            Block::Scene => self.scene(col, view_height),
            Block::Field => self.field(col, view_height),
            Block::Specimen {
                sprite,
                frame,
                note,
            } => self.specimen(sprite, *frame, note, col),
            Block::Species {
                photo,
                sprite,
                note,
            } => self.species(photo, sprite, note, col),
        }
    }

    /// Fetch (once) a bundled file from `img/`; `None` until loaded.
    pub(super) fn image(&mut self, name: &str) -> Option<HtmlImageElement> {
        if !self.images.contains_key(name) {
            let img = HtmlImageElement::new().ok()?;
            img.set_onload(Some(&self.redraw));
            img.set_src(&format!("img/{name}"));
            self.images.insert(name.to_string(), img);
        }
        self.images
            .get(name)
            .filter(|i| i.complete() && i.natural_width() > 0)
            .cloned()
    }

    fn photo(&mut self, id: &str, col: &mut Column, view_height: f64) {
        let image = self.image(&format!("{id}.jpg"));
        let aspect = image.as_ref().map_or(1.5, |i| {
            f64::from(i.natural_width()) / f64::from(i.natural_height().max(1))
        });
        let max_h = if self.width < WIDE {
            (view_height * 0.32).max(140.0)
        } else {
            (view_height * 0.45).max(200.0)
        };
        let h = (col.w / aspect).min(max_h);
        let w = h * aspect;
        // Leave room for the frame.
        col.y += 6.0;
        let x = col.x + (col.w - w) / 2.0;
        let y = col.y - self.scroll;
        match image.filter(|_| self.on_screen(y, h)) {
            Some(img) => {
                let _ = self
                    .ctx
                    .draw_image_with_html_image_element_and_dw_and_dh(&img, x, y, w, h);
            }
            None => {
                self.ctx.set_fill_style_str(ROW);
                self.ctx.fill_rect(x, y, w, h);
            }
        }
        self.frame((x, y, w, h));
        col.y += h + 13.0;
        // Attribution travels with the picture.
        if let Some(c) = self.game.content.credits.iter().find(|c| c.id == id) {
            let caption = format!("Photo: {} · {} · Wikimedia Commons", c.author, c.license);
            self.ctx.set_font(&font(15.0));
            col.y += self.paragraph(&caption, x, col.y, w, 15.0, DIM);
        }
        col.y += BLOCK_GAP;
    }

    /// Name and photo of the hovered map dot, kept inside the window.
    fn tooltip(&mut self, t: &Tooltip) {
        let Some(name) = self.game.content.map.node(&t.id).map(|n| n.name.clone()) else {
            return;
        };
        let image = self.image(&format!("{}.jpg", t.id));
        let w = 230.0;
        let image_h = image.as_ref().map_or(0.0, |i| {
            (w - 12.0) * f64::from(i.natural_height()) / f64::from(i.natural_width().max(1))
        });
        let image_h = image_h.min(170.0);
        self.ctx.set_font(&font(23.0));
        let text_h = line_height(23.0) * self.wrap(&name, w - 12.0).len() as f64;
        let h = image_h + text_h + 14.0;
        let x = (t.x + 14.0).min(self.width - w - 8.0).max(8.0);
        let y = if t.y + 14.0 + h > self.layout.view_height {
            (t.y - 14.0 - h).max(8.0)
        } else {
            t.y + 14.0
        };
        self.nine(PANEL, (x - 6.0, y - 6.0, w + 12.0, h + 12.0), true);
        // `paragraph` works in content coordinates.
        self.paragraph(
            &name,
            x + 6.0,
            y + self.scroll + 5.0,
            w - 12.0,
            23.0,
            ACCENT,
        );
        if let Some(img) = image {
            let _ = self.ctx.draw_image_with_html_image_element_and_dw_and_dh(
                &img,
                x + 6.0,
                y + text_h + 8.0,
                w - 12.0,
                image_h,
            );
        }
    }
}

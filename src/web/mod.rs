// Thin browser layer: canvas rendering, keyboard/mouse input, and
// localStorage saves. All game rules live in `crate::core`; this module only
// reads state and calls the public `Game` mutators.
//
// UI model (M1): a list of full-width menu rows.
//   Keyboard: ArrowUp/Down (or j/k) move selection, Enter/Space activates,
//             digits 1-9 pick immediately.
//   Mouse:    hover + click.
//   Touch:    tap (fires a click on most browsers; see note at attach time).

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, Window};

use crate::core::config::{MEALS, PACES, TRUCKS};
use crate::core::game::{DayReport, Game};
use crate::core::map_graph::{MapGraph, Node, Services};
use crate::core::party::MemberStatus;
use crate::core::state::{GameState, Screen};

// ---------- palette / layout -------------------------------------------------

const BG: &str = "#10131a";
const ROW: &str = "#1a2130";
const ROW_HOVER: &str = "#25314b";
const ACCENT: &str = "#d9a441";
const TEXT: &str = "#e9e7e0";
const DIM: &str = "#96a0b5";

const SAVE_KEY: &str = "wyoming-trail.v1";
const MARGIN: f64 = 28.0;
const ROW_H: f64 = 46.0;
const ROW_GAP: f64 = 10.0;
const HUD_H: f64 = 36.0;
const LINE_H: f64 = 26.0;

// ---------- buttons ----------------------------------------------------------

#[derive(Clone)]
pub enum Action {
    NewRun,
    Continue,
    ChooseTruck(usize),
    Depart,
    ChooseRoad(String, String), // (from, to) node ids
    ChoosePace(usize),
    ChooseMeal(usize),
    DriveDay,
    ContinueJourney,
    PlayAgain,
}

pub struct Button {
    pub label: String,
    pub detail: Option<String>,
    pub action: Action,
}

// ---------- app --------------------------------------------------------------

pub struct App {
    pub game: Game,
    pub canvas: HtmlCanvasElement,
    pub ctx: CanvasRenderingContext2d,
    pub title: bool,
    pub has_save: bool,
    pub report: Option<DayReport>,
    pub heading: String,
    pub sub: String,
    pub lines: Vec<String>,
    pub buttons: Vec<Button>,
    pub selected: usize,
    pub hover: Option<usize>,
    pub btn_top: f64,
    pub width: f64,
    pub height: f64,
}

// ---------- entry ------------------------------------------------------------

pub fn run(canvas: HtmlCanvasElement) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let ctx_obj = match canvas.get_context("2d") {
        Ok(Some(o)) => o,
        _ => return,
    };
    let ctx = match ctx_obj.dyn_into::<CanvasRenderingContext2d>() {
        Ok(c) => c,
        Err(_) => return,
    };

    let app = Rc::new(RefCell::new(App::new(window.clone(), canvas, ctx)));
    attach_listeners(&window, app.clone());

    let mut a = app.borrow_mut();
    a.refresh();
    a.render();
}

// ---------- small canvas helpers (isolated for signature drift) ---------------

/// `innerWidth`/`innerHeight` return `Result<JsValue, JsValue>` in web-sys.
fn window_px(w: &Window, width_side: bool) -> f64 {
    let res = if width_side {
        w.inner_width()
    } else {
        w.inner_height()
    };
    res.ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(if width_side { 1280.0 } else { 800.0 })
}

// ---------- App impl ----------------------------------------------------------

impl App {
    fn new(window: Window, canvas: HtmlCanvasElement, ctx: CanvasRenderingContext2d) -> Self {
        App {
            game: Game::new_run(0),
            canvas,
            ctx,
            title: true,
            has_save: load_save(&window).is_some(),
            report: None,
            heading: String::new(),
            sub: String::new(),
            lines: Vec::new(),
            buttons: Vec::new(),
            selected: 0,
            hover: None,
            btn_top: 0.0,
            width: window_px(&window, true).max(320.0),
            height: window_px(&window, false).max(240.0),
        }
    }

    // ---------- save / load (localStorage, graceful) -------------------------

    fn save(&self, window: &Window) {
        if self.title {
            return;
        }
        let Ok(json) = serde_json::to_string(&self.game.state) else {
            return;
        };
        let Ok(Some(storage)) = window.local_storage() else {
            return;
        };
        let _ = storage.set_item(SAVE_KEY, &json);
    }

    // ---------- text layout ----------------------------------------------------

    fn max_text_w(&self) -> f64 {
        self.width - MARGIN * 2.0
    }

    fn measure(&self, s: &str) -> f64 {
        self.ctx.measure_text(s).map(|t| t.width()).unwrap_or(0.0)
    }
}

pub fn load_save(window: &Window) -> Option<Game> {
    let Ok(Some(storage)) = window.local_storage() else {
        return None;
    };
    let raw = storage.get_item(SAVE_KEY).ok().flatten()?;
    let Ok(state) = serde_json::from_str::<GameState>(&raw) else {
        return None;
    };
    Some(Game {
        map: MapGraph::load(),
        state,
    })
}

fn attach_listeners(window: &Window, app: Rc<RefCell<App>>) {
    let mut a = app.borrow_mut();
    a.resize();
    let canvas = a.canvas.clone();
    drop(a);

    // mouse click (touch taps surface as clicks in modern browsers)
    let c = app.clone();
    let onclick = Closure::<dyn Fn(JsValue)>::new(move |v: JsValue| {
        if let Some(me) = v.dyn_ref::<web_sys::MouseEvent>() {
            let x = me.client_x();
            let y = me.client_y();
            let mut a = c.borrow_mut();
            a.click_point(x as f64, y as f64);
            a.render();
        }
    });
    canvas.set_onclick(onclick.as_ref().dyn_ref());

    // mouse move (hover)
    let c = app.clone();
    let onmove = Closure::<dyn Fn(JsValue)>::new(move |v: JsValue| {
        if let Some(me) = v.dyn_ref::<web_sys::MouseEvent>() {
            let y = me.client_y();
            let mut a = c.borrow_mut();
            if a.hover_needs_update(y as f64) {
                a.render();
            }
        }
    });
    canvas.set_onmousemove(onmove.as_ref().dyn_ref());

    // keyboard
    let c = app.clone();
    let onkey = Closure::<dyn Fn(JsValue)>::new(move |v: JsValue| {
        if let Some(ke) = v.dyn_ref::<web_sys::KeyboardEvent>() {
            let key = ke.key();
            let mut a = c.borrow_mut();
            a.handle_key(&key);
            a.render();
        }
    });
    window.set_onkeydown(onkey.as_ref().dyn_ref());

    // resize
    let c = app.clone();
    let onresize = Closure::<dyn Fn(JsValue)>::new(move |_v: JsValue| {
        let mut a = c.borrow_mut();
        a.resize();
        a.render();
    });
    window.set_onresize(onresize.as_ref().dyn_ref());
}

impl App {
    // ---------- sizing -------------------------------------------------------

    fn resize(&mut self) {
        if let Some(w) = web_sys::window() {
            self.width = window_px(&w, true).max(320.0);
            self.height = window_px(&w, false).max(240.0);
        }
        self.canvas.set_width(self.width as u32);
        self.canvas.set_height(self.height as u32);
        self.layout_buttons();
    }

    // ---------- layout -------------------------------------------------------

    fn layout_buttons(&mut self) {
        let n = self.buttons.len().max(1) as f64;
        let total = n * ROW_H + (n - 1.0).max(0.0) * ROW_GAP;
        self.btn_top = (self.height - HUD_H - 14.0 - total).max(0.0);
    }

    fn row_rect(&self, i: usize) -> (f64, f64, f64, f64) {
        let y = self.btn_top + (i as f64) * (ROW_H + ROW_GAP);
        (MARGIN, y, self.max_text_w(), ROW_H)
    }

    fn row_at(&self, y: f64) -> Option<usize> {
        (0..self.buttons.len()).find(|i| {
            let (_, ry, _, rh) = self.row_rect(*i);
            y >= ry - ROW_GAP / 2.0 && y <= ry + rh + ROW_GAP / 2.0
        })
    }

    fn hover_needs_update(&mut self, y: f64) -> bool {
        let new = self.row_at(y);
        let changed = new != self.hover;
        self.hover = new;
        changed
    }

    // ---------- input --------------------------------------------------------

    fn click_point(&mut self, x: f64, y: f64) {
        if let Some(i) = self.row_at(y) {
            let (rx, _, rw, _) = self.row_rect(i);
            if x >= rx - 4.0 && x <= rx + rw + 4.0 {
                self.activate(i);
            }
        }
    }

    fn handle_key(&mut self, key: &str) {
        match key {
            "ArrowUp" | "k" => self.shift(-1),
            "ArrowDown" | "j" => self.shift(1),
            "Enter" | " " => {
                self.activate(self.selected);
            }
            _ => {
                if key.len() == 1 {
                    if let Ok(n) = key.parse::<usize>() {
                        if (1..=9).contains(&n) && n <= self.buttons.len() {
                            self.selected = n - 1;
                            self.activate(n - 1);
                        }
                    }
                }
            }
        }
    }

    fn shift(&mut self, delta: isize) {
        let n = self.buttons.len();
        if n == 0 {
            return;
        }
        if delta > 0 {
            self.selected = (self.selected + 1) % n;
        } else {
            self.selected = (self.selected + n - 1) % n;
        }
    }

    // ---------- action dispatch ----------------------------------------------

    fn activate(&mut self, i: usize) {
        let Some(btn) = self.buttons.get(i) else {
            return;
        };
        let action = btn.action.clone();
        self.do_action(&action);
        let title = self.title;
        if !title {
            if let Some(w) = web_sys::window() {
                self.save(&w);
            }
        }
        self.refresh();
        self.render();
    }

    fn do_action(&mut self, a: &Action) {
        match a {
            Action::NewRun => {
                let seed = rand::random::<u64>();
                self.game = Game::new_run(seed);
                self.title = false;
            }
            Action::Continue => {
                let Some(w) = web_sys::window() else {
                    return;
                };
                if let Some(g) = load_save(&w) {
                    self.game = g;
                    self.title = false;
                }
            }
            Action::ChooseTruck(idx) => {
                let _ = self.game.choose_truck(*idx);
            }
            Action::Depart => {
                self.game.state.screen = Screen::RoadChoice;
            }
            Action::ChooseRoad(from, to) => {
                let Some(idx) = self
                    .game
                    .map
                    .roads
                    .iter()
                    .position(|r| r.from == *from && r.to == *to)
                else {
                    return;
                };
                let _ = self.game.set_off(idx);
            }
            Action::ChoosePace(idx) => {
                self.game.state.pace_index = *idx;
            }
            Action::ChooseMeal(idx) => {
                self.game.state.meal_index = *idx;
            }
            Action::DriveDay => {
                if let Ok(rep) = self.game.drive_day() {
                    self.report = Some(rep);
                }
            }
            Action::ContinueJourney => self.game.continue_journey(),
            Action::PlayAgain => {
                let seed = rand::random::<u64>();
                self.game = Game::new_run(seed);
                self.title = false;
            }
        }
    }

    // ---------- screen builders ----------------------------------------------

    fn push_btn(&mut self, label: &str, detail: Option<String>, action: Action) {
        self.buttons.push(Button {
            label: label.to_string(),
            detail,
            action,
        });
    }

    fn refresh(&mut self) {
        self.report = None;
        if self.title {
            self.build_title();
        } else {
            match self.game.state.screen {
                Screen::Outfit => self.build_outfit(),
                Screen::RoadChoice => self.build_road_choice(),
                Screen::Travel => self.build_travel(),
                Screen::Town => self.build_town(),
                Screen::Win => self.build_end(true),
                Screen::GameOver => self.build_end(false),
                _ => {
                    // Event / Encounter / RoadClosure / Hunt screens are M2+;
                    // for M1 they can't be produced by the core. If one somehow
                    // exists, show road choice from the current node rather
                    // than leaving the player stuck.
                    self.game.state.screen = Screen::RoadChoice;
                    self.build_road_choice();
                }
            }
        }
        if self.selected >= self.buttons.len() && !self.buttons.is_empty() {
            self.selected = self.buttons.len() - 1;
        }
        if self.buttons.is_empty() {
            self.selected = 0;
        }
        self.hover = None;
        self.layout_buttons();
    }

    fn node(&self, id: &str) -> Option<&Node> {
        self.game.map.node(id)
    }

    fn services_text(svc: &Services) -> String {
        let mut parts = Vec::new();
        if svc.fuel {
            parts.push("fuel");
        }
        if svc.food {
            parts.push("food");
        }
        if svc.motel {
            parts.push("motel");
        }
        if svc.mechanic {
            parts.push("mechanic");
        }
        if svc.outfitter {
            parts.push("outfitter");
        }
        if parts.is_empty() {
            "long quiet stretch — no services".to_string()
        } else {
            parts.join(" · ")
        }
    }

    fn build_title(&mut self) {
        self.heading = "WYOMING TRAIL".into();
        self.sub = "A road-trip RPG across modern Wyoming — from Cheyenne to Yellowstone.".into();
        let mut lines = vec![
            "Four friends, one pickup, and the long road west.".to_string(),
            "Watch the fuel and the food, pick a pace, and get everyone to the park.".to_string(),
        ];
        if self.has_save {
            lines.push(String::new());
            lines.push("'Continue' picks up the trip you saved on this device.".into());
        }
        self.lines = lines;
        self.buttons.clear();
        self.push_btn("New run", None, Action::NewRun);
        if self.has_save {
            self.push_btn("Continue trip", None, Action::Continue);
        }
        self.selected = 0;
    }

    fn build_outfit(&mut self) {
        let st = &self.game.state;
        let member_lines: Vec<String> = st
            .party
            .members
            .iter()
            .map(|m| {
                format!(
                    "• {} — {}% ready",
                    m.name,
                    (100.0 - m.fatigue).round() as u32
                )
            })
            .collect();
        let chosen_name = st.truck.name.clone();
        let truck_specs: Vec<(String, Option<String>, usize)> = TRUCKS
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let label = if t.name == chosen_name {
                    format!("✓ {}", t.name)
                } else {
                    t.name.to_string()
                };
                let detail = Some(format!(
                    "tank {} gal · {} mpg · {}% shape",
                    t.tank_gallons, t.mpg, t.condition
                ));
                (label, detail, i)
            })
            .collect();

        self.heading = "CHEYENNE, WY — OUTFITTING".into();
        self.sub = "Pick the truck you'll drive. Everyone's ready to roll.".into();
        self.lines = member_lines;
        self.buttons.clear();
        for (label, detail, i) in truck_specs {
            self.push_btn(&label, detail, Action::ChooseTruck(i));
        }
        self.push_btn("Set out on the road →", None, Action::Depart);
        self.selected = self.buttons.len() - 1;
    }

    fn build_road_choice(&mut self) {
        let st = &self.game.state;
        let cur = st.current_node.clone();
        let heading: String = self
            .node(&cur)
            .map(|n| n.name.clone())
            .unwrap_or_else(|| cur.to_uppercase());
        let sub: String = self
            .node(&cur)
            .map(|n| format!("{}  ·  {}", n.kind, Self::services_text(&n.services)))
            .unwrap_or_default();
        let history: Option<String> = self
            .node(&cur)
            .and_then(|n| (!n.history.is_empty()).then(|| n.history.clone()));
        let en_note = st.en_route.as_ref().map(|e| {
            format!(
                "Still {} mi to your destination if you push on.",
                e.remaining_miles
            )
        });
        let candidates: Vec<(String, Option<String>, String, String)> = self
            .game
            .map
            .roads
            .iter()
            .filter(|r| r.from == cur && !r.closed)
            .map(|r| {
                let dest = self
                    .node(&r.to)
                    .map(|n| n.name.clone())
                    .unwrap_or_else(|| r.to.clone());
                let label = format!("{}  →  {}", r.highway, dest);
                let warn = if r.seasonal_risk > 2 {
                    "  ⚠ seasonal"
                } else {
                    ""
                };
                let detail = Some(format!(
                    "{} mi · {}{}",
                    r.distance_miles,
                    r.terrain.label(),
                    warn
                ));
                (label, detail, r.from.clone(), r.to.clone())
            })
            .collect();

        let mut lines = Vec::new();
        if let Some(h) = history {
            lines.push(h);
        }
        if let Some(note) = en_note {
            lines.push(note);
        }
        self.heading = heading;
        self.sub = sub;
        self.lines = lines;
        self.buttons.clear();
        for (label, detail, from, to) in candidates {
            self.push_btn(&label, detail, Action::ChooseRoad(from, to));
        }
        if self.buttons.is_empty() {
            self.lines
                .push("No open roads left from here — a map bug (M1).".into());
            self.push_btn("Start a new run", None, Action::PlayAgain);
        }
        self.selected = 0;
    }

    fn build_travel(&mut self) {
        let st = &self.game.state;
        let sub = match st.en_route.as_ref() {
            Some(e) => format!(
                "to {}  ·  {} mi of {} left",
                self.node(&e.destination)
                    .map(|n| n.name.clone())
                    .unwrap_or_else(|| e.destination.clone()),
                e.remaining_miles,
                e.total_miles
            ),
            None => String::new(),
        };
        let mut lines = Vec::new();
        if let Some(rep) = &self.report {
            lines.push(format!(
                "Drove {} mi today.  Used {:.1} gal of fuel.",
                rep.miles_driven, rep.fuel_used_gallons
            ));
            lines.push(format!("Fuel left: {:.1} gal.  ", st.truck.fuel_gallons));
            if rep.stranded {
                lines.push("Out of fuel — you're stranded. (handled with roadside help)".into());
            }
            if rep.meals_eaten > 0 {
                lines.push(format!("Eaten {} meal(s) today.", rep.meals_eaten));
            }
            lines.push(String::new());
        }
        lines.push(format!("Abroad & well: {} of 4.", st.party.in_trip_count()));
        let pace_index = st.pace_index;
        let meal_index = st.meal_index;
        let pace_specs: Vec<(String, Option<String>, usize)> = PACES
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let label = if i == pace_index {
                    format!("{}  ✓ pace", p.name)
                } else {
                    format!("{}  pace", p.name)
                };
                let detail = Some(format!("{} h/day · fatigue {:+}", p.hours, p.fatigue_delta));
                (label, detail, i)
            })
            .collect();
        let meal_specs: Vec<(String, Option<String>, usize)> = MEALS
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let label = if i == meal_index {
                    format!("{}  ✓", m.name)
                } else {
                    m.name.to_string()
                };
                let detail = Some(format!("${:.2}/person", m.cost_per_person));
                (label, detail, i)
            })
            .collect();

        self.heading = "ON THE ROAD".into();
        self.sub = sub;
        self.lines = lines;
        self.buttons.clear();
        for (label, detail, i) in pace_specs {
            self.push_btn(&label, detail, Action::ChoosePace(i));
        }
        for (label, detail, i) in meal_specs {
            self.push_btn(&label, detail, Action::ChooseMeal(i));
        }
        self.push_btn("Drive the day", None, Action::DriveDay);
        self.selected = self.buttons.len() - 1;
    }

    fn build_town(&mut self) {
        let st = &self.game.state;
        let cur = st.current_node.clone();
        let heading: String = self
            .node(&cur)
            .map(|n| n.name.clone())
            .unwrap_or_else(|| cur.to_uppercase());
        let sub: String = self
            .node(&cur)
            .map(|n| format!("You've arrived.  {}", Self::services_text(&n.services)))
            .unwrap_or_else(|| "You've arrived.".to_string());
        let history: Option<String> = self
            .node(&cur)
            .and_then(|n| (!n.history.is_empty()).then(|| n.history.clone()));
        let mut lines = Vec::new();
        if let Some(h) = history {
            lines.push(h);
        }
        lines.push(String::new());
        for m in &st.party.members {
            if m.status == MemberStatus::LeftTrip {
                lines.push(format!("• {} — left the trip earlier.", m.name));
            } else {
                lines.push(format!(
                    "• {} — health {}% · tiredness {}%",
                    m.name, m.health, m.fatigue
                ));
            }
        }
        let fuel = st.truck.fuel_gallons;
        if fuel < 8.0 {
            lines.push(String::new());
            lines.push(format!(
                "Fuel is low ({:.1} gal) — refuel at the next station.",
                fuel
            ));
        }
        self.heading = heading;
        self.sub = sub;
        self.lines = lines;
        self.buttons.clear();
        self.push_btn("Continue on the trail →", None, Action::ContinueJourney);
        self.selected = 0;
    }

    fn build_end(&mut self, win: bool) {
        let st = &self.game.state;
        self.heading = if win {
            "YELLOWSTONE NATIONAL PARK".into()
        } else {
            "THE TRIP ENDS".into()
        };
        self.sub = if win {
            "You made it!  The geysers are doing their thing ahead.".into()
        } else if st.ending_reason.is_empty() {
            "The trail ran out of ways to keep going.".into()
        } else {
            st.ending_reason.clone()
        };
        self.lines = vec![
            format!("Days on the road: {}", st.day),
            format!("Party still aboard: {} of 4", st.party.in_trip_count()),
            format!("Money left: ${:.2}", st.resources.money),
            format!("Landmarks visited: {}", st.visited.len()),
            String::new(),
            if win {
                "Good trip. Run it again with a fresh seed if you like."
            } else {
                "No one got hurt — everyone made it home for the long debrief."
            }
            .to_string(),
        ];
        self.buttons.clear();
        self.push_btn("Play again (new run)", None, Action::PlayAgain);
        self.selected = 0;
    }

    // ---------- rendering ------------------------------------------------------

    fn render(&mut self) {
        let ctx = &self.ctx;
        ctx.set_fill_style_str(BG);
        ctx.fill_rect(0.0, 0.0, self.width, self.height);

        let mut y = MARGIN;

        ctx.set_font("bold 20px monospace");
        ctx.set_fill_style_str(ACCENT);
        let _ = ctx.fill_text(&self.heading, MARGIN, y + 16.0);
        y += 30.0;

        if !self.sub.is_empty() {
            ctx.set_font("14px monospace");
            ctx.set_fill_style_str(DIM);
            let _ = ctx.fill_text(&self.sub, MARGIN, y + 10.0);
            y += LINE_H;
        }

        ctx.set_font("14px monospace");
        ctx.set_fill_style_str(TEXT);
        for line in &self.lines {
            if y + LINE_H > self.btn_top - 10.0 {
                break;
            }
            if line.is_empty() {
                y += LINE_H * 0.5;
                continue;
            }
            let _ = ctx.fill_text(line, MARGIN, y + 12.0);
            y += LINE_H;
        }

        for (i, b) in self.buttons.iter().enumerate() {
            let (x, ry, w, h) = self.row_rect(i);
            let hovered = self.hover == Some(i);
            let selected = self.selected == i;
            ctx.set_fill_style_str(if hovered { ROW_HOVER } else { ROW });
            ctx.fill_rect(x, ry, w, h);

            let num = (i + 1).to_string();
            let mid = ry + h / 2.0 + 5.0;

            ctx.set_font("14px monospace");
            ctx.set_fill_style_str(if selected { ACCENT } else { DIM });
            let _ = ctx.fill_text(&num, x + 12.0, mid);

            ctx.set_fill_style_str(if selected { ACCENT } else { TEXT });
            let _ = ctx.fill_text(&b.label, x + 36.0, mid);

            if let Some(d) = &b.detail {
                ctx.set_font("13px monospace");
                ctx.set_fill_style_str(DIM);
                let w_txt = self.measure(d);
                let _ = ctx.fill_text(d, x + w - 8.0 - w_txt, mid);
            }
        }

        // HUD strip
        let hud = format!(
            "Day {}  ·  ${:.0}  ·  {} food  ·  {:.1} gal  ·  {} aboard",
            self.game.state.day,
            self.game.state.resources.money,
            self.game.state.resources.food,
            self.game.state.truck.fuel_gallons,
            self.game.state.party.in_trip_count()
        );
        ctx.set_font("13px monospace");
        ctx.set_fill_style_str(DIM);
        let _ = ctx.fill_text(&hud, MARGIN, self.height - 10.0);
    }
}

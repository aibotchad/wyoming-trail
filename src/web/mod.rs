//! Thin browser layer: input, localStorage saves and action dispatch. All game
//! rules live in `crate::core`; `screens` turns state into a [`page::Page`]
//! and `draw` paints it on the canvas.
//!
//! Controls: click/tap buttons and map dots, drag or wheel to scroll, arrows
//! (or j/k) + Enter, digits 1-9, M for the map, Esc to go back.

mod cards;
mod charts;
mod draw;
mod field;
mod finale;
mod mapview;
mod page;
mod pause;
mod scene;
mod scores;
mod screens;
mod shop;
mod stats;
mod toast;
mod ui;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement, Window};

use crate::core::content::Content;
use crate::core::game::Game;
use crate::core::gather::{Session, LANDSCAPE, PORTRAIT};
use crate::core::state::{GameState, Screen};

use crate::core::scores::TopTen;
use field::Controls;
use page::{Action, Layout, Page, View};
use scene::Drive;
use shop::Confirm;
use toast::Toast;

const SAVE_KEY: &str = "wyoming-trail.v5";
/// Animation redraw interval.
const TICK_MS: i32 = 33;
/// Pointer travel (px) beyond which a press is a scroll, not a tap. Fingers
/// wobble more than a mouse.
const DRAG_SLOP: f64 = 8.0;
const FINGER_SLOP: f64 = 12.0;
/// A flicked page keeps this share of its speed each tick, and stops below
/// this speed (px a second).
const FLING_KEEP: f64 = 0.92;
const FLING_STOP: f64 = 40.0;
/// A finger that rests this long before lifting didn't flick.
const FLING_PAUSE_MS: f64 = 100.0;
/// The length of time `FLING_KEEP` is measured over.
const FLING_STEP: f64 = 1.0 / 30.0;
/// Canvas pixels per screen pixel, at most. Sharper than this costs more to
/// paint than anyone can see.
const MAX_DENSITY: f64 = 2.0;

/// An active mouse/finger press.
struct Press {
    /// The pointer that made it; other fingers are ignored.
    id: i32,
    x: f64,
    y: f64,
    scroll: f64,
    dragging: bool,
    /// Last height and time seen, for the flick speed.
    last_y: f64,
    last_at: f64,
    /// How fast the page is being dragged, in px a second.
    speed: f64,
}

pub struct App {
    game: Game,
    view: View,
    /// Where "Back" returns to from the map, a place card or the credits.
    history: Vec<View>,
    has_save: bool,
    /// The message popped up over the page.
    toast: Option<Toast>,
    /// Last input was a finger: hide keyboard hints and cursors.
    touch: bool,
    /// Speed of a flicked page still coasting, in px a second, and when it
    /// last moved.
    fling: f64,
    fling_at: f64,
    /// The gathering clock is running.
    round_started: bool,
    /// Driving on, day after day, until told to stop or something comes up.
    cruising: bool,
    /// The drum circle, at the end of a mission that has one.
    circle: finale::Circle,
    /// The top ten, as last read.
    scores: TopTen,

    /// The drive currently being animated.
    drive: Option<Drive>,
    /// The poaching or foraging round being played.
    round: Option<Session>,
    controls: Controls,
    /// A purchase waiting for yes or no.
    confirm: Option<Confirm>,

    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    images: HashMap<String, HtmlImageElement>,
    /// Paragraphs already broken into lines.
    wrapped: RefCell<HashMap<String, Rc<[String]>>>,
    /// Re-renders once an image finishes loading.
    redraw: js_sys::Function,
    /// Paints at the browser's next refresh, once however often it is asked.
    frame: js_sys::Function,
    frame_due: bool,

    page: Page,
    layout: Layout,
    selected: usize,
    scroll: f64,
    press: Option<Press>,
    /// Pointer position in canvas pixels, for hover effects.
    pointer: Option<(f64, f64)>,
    width: f64,
    height: f64,
}

pub fn run(canvas: HtmlCanvasElement) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(Some(ctx)) = canvas.get_context("2d") else {
        return;
    };
    let Ok(ctx) = ctx.dyn_into::<CanvasRenderingContext2d>() else {
        return;
    };

    // The callbacks need the app, and the app holds the callbacks.
    let slot: Slot = Rc::default();
    let redraw = call_back(&slot, App::request_frame);
    let frame = call_back(&slot, App::next_frame);

    let content = Rc::new(Content::load());
    let app = Rc::new(RefCell::new(App {
        game: Game::new(content.clone(), 0),
        view: View::Title,
        history: Vec::new(),
        has_save: load_save(&window, content).is_some(),
        toast: None,
        touch: window
            .match_media("(pointer: coarse)")
            .ok()
            .flatten()
            .is_some_and(|m| m.matches()),
        fling: 0.0,
        fling_at: 0.0,
        round_started: false,
        cruising: false,
        circle: finale::Circle::default(),
        scores: scores::load().unwrap_or_default(),
        drive: None,
        round: None,
        controls: Controls::default(),
        confirm: None,
        canvas,
        ctx,
        images: HashMap::new(),
        wrapped: RefCell::default(),
        redraw,
        frame,
        frame_due: false,
        page: Page::default(),
        layout: Layout::default(),
        selected: 0,
        scroll: 0.0,
        press: None,
        pointer: None,
        width: 800.0,
        height: 600.0,
    }));
    *slot.borrow_mut() = Some(app.clone());
    attach_listeners(&window, &app);

    let mut a = app.borrow_mut();
    a.resize();
    a.refresh();
    a.render();
}

/// Where callbacks find the app once it exists.
type Slot = Rc<RefCell<Option<Rc<RefCell<App>>>>>;

/// A long-lived function the browser can call to run `method` on the app.
fn call_back(slot: &Slot, method: fn(&mut App)) -> js_sys::Function {
    let slot = slot.clone();
    let closure = Closure::<dyn Fn()>::new(move || {
        if let Some(app) = slot.borrow().as_ref() {
            if let Ok(mut a) = app.try_borrow_mut() {
                method(&mut a);
            }
        }
    });
    closure.into_js_value().unchecked_into()
}

fn load_save(window: &Window, content: Rc<Content>) -> Option<Game> {
    let storage = window.local_storage().ok().flatten()?;
    let raw = storage.get_item(SAVE_KEY).ok().flatten()?;
    let state = serde_json::from_str::<GameState>(&raw).ok()?;
    Game::from_state(content, state)
}

/// Register `handler` as a long-lived event callback.
fn listener<E: JsCast + 'static>(
    app: &Rc<RefCell<App>>,
    handler: impl Fn(&mut App, E) + 'static,
) -> js_sys::Function {
    let app = app.clone();
    let closure = Closure::<dyn Fn(JsValue)>::new(move |v: JsValue| {
        if let (Ok(event), Ok(mut a)) = (v.dyn_into::<E>(), app.try_borrow_mut()) {
            handler(&mut a, event);
        }
    });
    // The browser keeps calling it, so it must never be dropped.
    closure.into_js_value().unchecked_into()
}

fn attach_listeners(window: &Window, app: &Rc<RefCell<App>>) {
    use web_sys::{Event, KeyboardEvent, PointerEvent, WheelEvent};
    let canvas = app.borrow().canvas.clone();

    let down = listener(app, |a, e: PointerEvent| {
        let at = (f64::from(e.client_x()), f64::from(e.client_y()));
        let touch = e.pointer_type() == "touch";
        if touch != a.touch {
            a.touch = touch;
            a.refresh();
            a.render();
        }
        // Touching a coasting page stops it, and does nothing else.
        let coasting = std::mem::take(&mut a.fling) != 0.0;
        if a.field_press(e.pointer_id(), at) || !e.is_primary() {
            return;
        }
        a.press = Some(Press {
            id: e.pointer_id(),
            x: at.0,
            y: at.1,
            scroll: a.scroll,
            dragging: coasting,
            last_y: at.1,
            last_at: js_sys::Date::now(),
            speed: 0.0,
        });
    });
    canvas.set_onpointerdown(Some(&down));

    let moved = listener(app, |a, e: PointerEvent| {
        let (x, y) = (f64::from(e.client_x()), f64::from(e.client_y()));
        let touch = e.pointer_type() == "touch";
        // Fingers don't hover.
        a.pointer = (!touch).then_some((x, y));
        if a.field_drag(e.pointer_id(), (x, y)) {
            return;
        }
        if let Some(p) = a.press.as_mut().filter(|p| p.id == e.pointer_id()) {
            let slop = if touch { FINGER_SLOP } else { DRAG_SLOP };
            p.dragging |= (x - p.x).hypot(y - p.y) > slop;
            if p.dragging {
                a.scroll = p.scroll - (y - p.y);
                let now = js_sys::Date::now();
                if now > p.last_at {
                    let latest = (p.last_y - y) / (now - p.last_at) * 1000.0;
                    p.speed = 0.6 * latest + 0.4 * p.speed;
                    (p.last_y, p.last_at) = (y, now);
                }
            }
        } else if touch {
            return;
        }
        a.request_frame();
    });
    canvas.set_onpointermove(Some(&moved));

    let up = listener(app, |a, e: PointerEvent| {
        let at = (f64::from(e.client_x()), f64::from(e.client_y()));
        if a.field_release(e.pointer_id(), at) {
            a.render();
            return;
        }
        let Some(p) = a.press.take_if(|p| p.id == e.pointer_id()) else {
            return;
        };
        if !p.dragging {
            a.click(at.0, at.1);
        } else if e.pointer_type() == "touch" && js_sys::Date::now() - p.last_at < FLING_PAUSE_MS {
            a.fling = p.speed;
            a.fling_at = js_sys::Date::now();
        }
        a.render();
    });
    canvas.set_onpointerup(Some(&up));

    let cancel = listener(app, |a, e: PointerEvent| {
        a.field_cancel(e.pointer_id());
        a.press.take_if(|p| p.id == e.pointer_id());
        a.pointer = None;
        a.render();
    });
    canvas.set_onpointercancel(Some(&cancel));

    let wheel = listener(app, |a, e: WheelEvent| {
        // delta_mode 0 is pixels; anything else counts lines or pages.
        let unit = if e.delta_mode() == 0 { 1.0 } else { 24.0 };
        a.scroll += e.delta_y() * unit;
        a.fling = 0.0;
        a.request_frame();
    });
    window.set_onwheel(Some(&wheel));

    let key = listener(app, |a, e: KeyboardEvent| {
        if e.ctrl_key() || e.meta_key() || e.alt_key() {
            return;
        }
        if a.touch {
            a.touch = false;
            a.refresh();
            a.render();
        }
        a.fling = 0.0;
        if a.field_key(&e.key(), e.repeat()) {
            return;
        }
        // A held Enter must not buy ten of something.
        if e.repeat() && matches!(e.key().as_str(), "Enter" | " ") {
            return;
        }
        a.key(&e.key());
        a.render();
    });
    window.set_onkeydown(Some(&key));

    let released = listener(app, |a, e: KeyboardEvent| a.controls.release(&e.key()));
    window.set_onkeyup(Some(&released));

    // Keys let go while the window is away would otherwise stick.
    let blur = listener(app, |a, _: Event| {
        a.controls = Controls::default();
        a.cruising = false;
        if a.playing() {
            a.perform(Action::Pause);
        }
    });
    window.set_onblur(Some(&blur));

    let app_tick = app.clone();
    let tick = Closure::<dyn Fn()>::new(move || {
        if let Ok(mut a) = app_tick.try_borrow_mut() {
            a.tick();
        }
    });
    let _ = window.set_interval_with_callback_and_timeout_and_arguments_0(
        tick.as_ref().unchecked_ref(),
        TICK_MS,
    );
    tick.forget();

    let resize = listener(app, |a, _: Event| {
        a.resize();
        a.refresh();
        a.render();
    });
    window.set_onresize(Some(&resize));
}

impl App {
    /// The size the canvas is shown at, which is what pointer positions are
    /// measured against.
    fn shown_size(&self) -> (f64, f64) {
        (
            f64::from(self.canvas.client_width().max(1)),
            f64::from(self.canvas.client_height().max(1)),
        )
    }

    fn resize(&mut self) {
        let Some(window) = web_sys::window() else {
            return;
        };
        (self.width, self.height) = self.shown_size();
        self.wrapped.borrow_mut().clear();
        // Back the canvas at device resolution so text stays crisp.
        let dpr = window.device_pixel_ratio().clamp(1.0, MAX_DENSITY);
        self.canvas.set_width((self.width * dpr) as u32);
        self.canvas.set_height((self.height * dpr) as u32);
        let _ = self.ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0);
    }

    /// Rebuild the page after a state change. Selection and scroll survive
    /// while we stay on the same page (e.g. buying several things).
    fn refresh(&mut self) {
        let page = self.build_page();
        if page.id != self.page.id {
            self.selected = page.default_selected;
            self.scroll = 0.0;
        }
        self.selected = self.selected.min(page.button_count().saturating_sub(1));
        self.page = page;
    }

    fn click(&mut self, x: f64, y: f64) {
        // A pop-up message is dismissed by tapping it.
        if self.confirm.is_none() && self.on_toast(x, y) {
            self.toast = None;
            return;
        }
        let on_stats = self
            .layout
            .stats_button
            .is_some_and(|(bx, by, w, h)| (bx..=bx + w).contains(&x) && (by..=by + h).contains(&y));
        if self.confirm.is_none() && on_stats {
            self.perform(Action::ShowStats);
            return;
        }
        let on_menu = self
            .layout
            .menu_button
            .is_some_and(|(bx, by, w, h)| (bx..=bx + w).contains(&x) && (by..=by + h).contains(&y));
        if self.confirm.is_none() && on_menu {
            self.perform(Action::Pause);
            return;
        }
        // The confirmation box sits above everything and has its own buttons.
        if self.confirm.is_none() && self.covered(x, y) {
            return;
        }
        if let Some(action) = self.layout.hit(x, y + self.scroll).cloned() {
            if let Some(i) = self.layout.button_at(x, y + self.scroll) {
                self.selected = i;
            }
            self.perform(action);
        }
    }

    /// Is this screen point hidden by the status bar or the cash badge?
    fn covered(&self, x: f64, y: f64) -> bool {
        let under_badge = self.shopping() && {
            let (bx, by, w, h) = self.badge_rect();
            y < self.badge_strip() || (bx..=bx + w).contains(&x) && (by..=by + h).contains(&y)
        };
        y >= self.layout.view_height || under_badge
    }

    fn key(&mut self, key: &str) {
        if self.confirm.is_some() {
            self.confirm_key(key);
            return;
        }
        let n = self.page.button_count();
        match key {
            "ArrowUp" | "k" if n > 0 => self.select((self.selected + n - 1) % n),
            "ArrowDown" | "j" if n > 0 => self.select((self.selected + 1) % n),
            "PageUp" => self.scroll -= self.height * 0.8,
            "PageDown" => self.scroll += self.height * 0.8,
            "Enter" | " " => self.press_button(self.selected),
            "Escape" => {
                if let Some(back) = self.page.back.clone() {
                    self.perform(back);
                } else if self.can_pause() {
                    self.perform(Action::Pause);
                }
            }
            "p" | "P" if self.view == View::Pause => self.perform(Action::Back),
            "p" | "P" if self.can_pause() => self.perform(Action::Pause),
            "m" | "M" if !matches!(self.view, View::Map | View::Gather) && self.drive.is_none() => {
                self.perform(Action::ShowMap)
            }
            "s" | "S" if self.has_stats() => self.perform(Action::ShowStats),
            _ => {
                if let Ok(digit @ 1..=9) = key.parse::<usize>() {
                    self.press_button(digit - 1);
                }
            }
        }
    }

    /// Move the keyboard selection and scroll it into view.
    fn select(&mut self, index: usize) {
        self.selected = index;
        if let Some((top, bottom)) = self.layout.button_span(index) {
            let view = self.layout.view_height;
            if top < self.scroll {
                self.scroll = top - 8.0;
            } else if bottom > self.scroll + view {
                self.scroll = bottom - view + 8.0;
            }
        }
    }

    fn press_button(&mut self, index: usize) {
        if let Some(action) = self.page.action(index).cloned() {
            self.selected = index;
            self.perform(action);
        }
    }

    /// Advance animations; redraws only while something is moving.
    fn tick(&mut self) {
        // Phone browsers change the window without always saying so.
        if self.shown_size() != (self.width, self.height) {
            self.resize();
            self.refresh();
            self.render();
        }
        if self.toast.as_ref().is_some_and(Toast::expired) {
            self.toast = None;
            self.render();
        }
        if self.view == View::Gather {
            self.step_field();
            return;
        }
        if self.drive.as_ref().is_some_and(Drive::finished) {
            self.drive = None;
            self.drive_on();
            return;
        }
        // Painted straight from the timer: measured much cheaper than handing
        // these steady repaints to the next screen refresh.
        if self.layout.animated && !self.frame_due {
            self.render();
        }
    }

    /// A day's drive has been watched: start the next, or say why not.
    fn drive_on(&mut self) {
        let screen = self.game.state.screen;
        if self.cruising && screen == Screen::Travel && self.view == View::Game {
            match self.game.reason_to_stop() {
                Some(why) => {
                    self.cruising = false;
                    self.refresh();
                    self.warn(format!("Pulled over. {why}"));
                }
                None => {
                    self.perform(Action::DriveDay);
                    return;
                }
            }
        } else if !matches!(screen, Screen::Event | Screen::Hitchhiker) {
            // Arrived, or the trip is over. An event only interrupts.
            self.cruising = false;
        }
        self.refresh();
        self.render();
    }

    /// Ask for a paint at the next screen refresh.
    fn request_frame(&mut self) {
        if self.frame_due {
            return;
        }
        let window = web_sys::window();
        self.frame_due = window.is_some_and(|w| w.request_animation_frame(&self.frame).is_ok());
    }

    /// The screen is about to refresh: move a coasting page and paint.
    fn next_frame(&mut self) {
        self.frame_due = false;
        if self.fling == 0.0 {
            self.render();
            return;
        }
        let now = js_sys::Date::now();
        let dt = ((now - self.fling_at) / 1000.0).clamp(0.0, 0.05);
        self.fling_at = now;
        let wanted = self.scroll + self.fling * dt;
        self.scroll = wanted;
        self.fling *= FLING_KEEP.powf(dt / FLING_STEP);
        self.render();
        // Rendering clamps at either end of the page.
        if self.scroll != wanted || self.fling.abs() < FLING_STOP {
            self.fling = 0.0;
        } else {
            self.request_frame();
        }
    }

    /// Is the run itself, or its pause menu, what's on screen? Only then may
    /// it change, so only then is it recorded and saved.
    fn run_showing(&self) -> bool {
        matches!(self.view, View::Game | View::Pause)
    }

    /// A run past its setup questions, which has figures worth graphing.
    fn has_stats(&self) -> bool {
        self.view == View::Game && !self.game.state.screen.is_setup()
    }

    fn perform(&mut self, action: Action) {
        // A drumbeat changes nothing in the run.
        if action == Action::Drum {
            self.circle.strike(js_sys::Date::now());
            self.refresh();
            self.render();
            return;
        }
        // A graph's figure changes nothing: just show it.
        if let Action::Reveal(figure) = action {
            self.say(figure);
            self.render();
            return;
        }
        // Anything that costs money asks first.
        let action = match self.confirm_step(action) {
            Some(a) => a,
            None => {
                self.refresh();
                self.render();
                return;
            }
        };
        self.toast = None;
        self.fling = 0.0;
        let page = self.page.id.clone();
        let log = self.game.state.log.clone();
        let failed = self.apply(action).err();
        // A rest from the pause menu can end the trip: show the ending.
        if self.view == View::Pause && self.game.state.screen.is_over() {
            self.view = View::Game;
            self.history.clear();
        }
        if self.run_showing() {
            self.game.record();
        }
        self.save();
        self.refresh();
        match failed {
            Some(why) => self.warn(why),
            // News on the same page may be scrolled out of sight: pop it up.
            // The road shows its own news; a pop-up a day would be noise.
            None if self.page.id == page && page != "driving" && self.game.state.log != log => {
                self.say(self.game.state.log.join("  "));
            }
            None => {}
        }
        self.render();
    }

    fn open(&mut self, view: View) {
        self.history.push(std::mem::replace(&mut self.view, view));
    }

    fn apply(&mut self, action: Action) -> Result<(), String> {
        let window = web_sys::window().ok_or("no window")?;
        // Any action ends a drive animation early, and all but answering what
        // came up on the road ends driving on.
        self.drive = None;
        self.cruising &= matches!(
            action,
            Action::DriveDay | Action::ChooseRoad(_) | Action::ContinueJourney | Action::PickUp(_)
        );
        match action {
            Action::NewRun => {
                let sure = !self.has_save
                    || window
                        .confirm_with_message("Start over and erase your saved trip?")
                        .unwrap_or(true);
                if sure {
                    self.start_run();
                }
            }
            Action::Continue => {
                let content = self.game.content.clone();
                self.game = load_save(&window, content).ok_or("That save can't be loaded.")?;
                self.view = View::Game;
                self.history.clear();
            }
            Action::PlayAgain => self.start_run(),
            Action::MainMenu => {
                self.view = View::Title;
                self.history.clear();
            }
            Action::ShowMap => self.open(View::Map),
            Action::ShowPlace(id) => self.open(View::Place(id)),
            Action::ShowCredits => self.open(View::Credits),
            Action::ShowScores => {
                // Another tab may have added to it.
                if let Some(stored) = scores::load() {
                    self.scores = stored;
                }
                self.open(View::Scores);
            }
            Action::ClearScores => self.clear_scores(),
            Action::Pause => {
                if self.can_pause() {
                    self.open(View::Pause);
                }
            }
            Action::ShowHelp => self.open(View::Help),
            Action::Camp(nights) => self.game.camp(nights)?,
            Action::ShowStats => {
                if self.view != View::Stats {
                    self.open(View::Stats);
                }
            }
            // Handled by `perform`.
            Action::Reveal(_) | Action::Drum => {}
            Action::Back => {
                self.view = self.history.pop().unwrap_or(View::Title);
                // Keys and fingers from before a pause don't carry over.
                if self.view == View::Gather {
                    self.controls = Controls::default();
                }
            }
            Action::ChooseProfession(i) => self.game.choose_profession(i)?,
            Action::Rename(i) => {
                let current = self.game.state.party.members.get(i);
                let current = current.map(|m| m.name.clone()).unwrap_or_default();
                let answer = window
                    .prompt_with_message_and_default("Name this traveller:", &current)
                    .ok()
                    .flatten();
                if let Some(name) = answer {
                    self.game.set_name(i, &name);
                }
            }
            Action::ConfirmParty => self.game.confirm_party()?,
            Action::BuyTruck(i) => self.game.buy_truck(i)?,
            Action::Buy(id) => self.game.buy(&id)?,
            Action::FillTank => self.game.fill_tank()?,
            Action::OpenStore => self.game.open_store()?,
            Action::LeaveStore => self.game.leave_store()?,
            Action::Rest(motel) => self.game.rest(motel)?,
            Action::ChooseRoad(i) => {
                self.game.set_off(i)?;
                // Choosing from a map card drops straight back into the game.
                self.view = View::Game;
                self.history.clear();
                self.set_out()?;
            }
            Action::CyclePace => {
                self.game.cycle_pace();
            }
            Action::CycleMeal => {
                self.game.cycle_meal();
            }
            Action::DriveDay => {
                self.drive_a_day()?;
                self.cruising = true;
            }
            Action::StopDriving => {}
            // Clearing the animation above is all it takes.
            Action::SkipDrive => {}
            Action::TurnBack => self.game.turn_back()?,
            Action::SetupBack => self.game.setup_back()?,
            Action::SetFemale(female) => self.game.set_female(female)?,
            Action::ChooseMonth(m) => self.game.choose_month(m)?,
            Action::PickUp(yes) => {
                self.game.pick_up(yes)?;
                self.resume_driving()?;
            }
            Action::OpenTrade => self.game.open_trade()?,
            Action::OpenWork => self.game.open_work()?,
            Action::SellFood(n) => self.game.sell_food(n)?,
            Action::SellItem(id) => self.game.sell_item(&id)?,
            Action::Work(id) => self.game.work(&id)?,
            // Both are dealt with before an action gets this far.
            Action::ConfirmYes | Action::ConfirmNo => {}
            Action::StartGather => {
                let shape = if self.width < self.height {
                    PORTRAIT
                } else {
                    LANDSCAPE
                };
                self.round = Some(self.game.start_gather(shape)?);
                self.controls = Controls::default();
                self.round_started = false;
                self.view = View::Gather;
            }
            Action::BeginGather => self.begin_round(),
            Action::CancelGather => {
                self.round = None;
                self.view = View::Game;
            }
            Action::FinishGather => {
                self.view = View::Game;
                if let Some(round) = self.round.take() {
                    self.game.finish_gather(&round)?;
                }
            }
            Action::ContinueJourney => {
                self.game.continue_journey();
                self.resume_driving()?;
            }
        }
        Ok(())
    }

    /// Start driving the road just chosen, unless the tank can't finish it:
    /// then the player gets the chance to think again.
    fn set_out(&mut self) -> Result<(), String> {
        let state = &self.game.state;
        let miles = state.en_route.as_ref().map_or(0.0, |e| e.remaining_miles);
        if self.game.range_miles() < miles {
            let fuel = &self.game.ride_def().fuel;
            let verb = &self.game.ride_def().verb;
            return Err(format!(
                "Not enough {fuel} for this road. Press {verb} to risk it, or pick another."
            ));
        }
        self.drive_a_day()?;
        self.cruising = true;
        Ok(())
    }

    fn drive_a_day(&mut self) -> Result<(), String> {
        let before = self.game.state.clone();
        self.game.drive_day()?;
        self.drive = Drive::between(&before, &self.game);
        Ok(())
    }

    /// Back on the road after an interruption, if all is well.
    fn resume_driving(&mut self) -> Result<(), String> {
        if !self.cruising || self.game.state.screen != Screen::Travel {
            return Ok(());
        }
        match self.game.reason_to_stop() {
            Some(why) => {
                self.cruising = false;
                Err(format!("Pulled over. {why}"))
            }
            None => self.drive_a_day(),
        }
    }

    fn start_run(&mut self) {
        self.game = Game::new(self.game.content.clone(), rand::random::<u64>());
        self.circle = finale::Circle::default();
        self.view = View::Game;
        self.history.clear();
    }

    /// Save the run in progress; forget it once it has ended. Storage can be
    /// unavailable (private browsing), in which case we just carry on.
    fn save(&mut self) {
        if !self.run_showing() {
            return;
        }
        // Listed before the saved run is thrown away, and kept for this
        // visit even where nothing can be stored.
        if self.game.state.screen.is_over() {
            self.keep_score();
        }
        let Some(Ok(Some(storage))) = web_sys::window().map(|w| w.local_storage()) else {
            return;
        };
        if self.game.state.screen.is_over() {
            let _ = storage.remove_item(SAVE_KEY);
            self.has_save = false;
        } else if let Ok(json) = serde_json::to_string(&self.game.state) {
            self.has_save = storage.set_item(SAVE_KEY, &json).is_ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The crash screen in `index.html` erases the save by its key.
    #[test]
    fn the_crash_screen_knows_the_save_key() {
        let page = include_str!("../../index.html");
        assert!(page.contains(&format!("localStorage.removeItem('{SAVE_KEY}')")));
    }
}

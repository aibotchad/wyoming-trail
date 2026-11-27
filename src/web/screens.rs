//! Turns app state into a [`Page`]. Words only — no drawing here.

use crate::core::config::WILD_FOOD_PRICE;
use crate::core::config::{MEALS, MOTEL_COST, PACES};
use crate::core::defs::{Crew, GatherMode, ItemKind, ProfessionDef};
use crate::core::game::GoalState;
use crate::core::map_graph::{Node, Road, Services};
use crate::core::state::Screen;

use super::page::{days, Action, Block, Page, Tone, View};
use super::stats::thousands;
use super::App;

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

const OPENING_LINE: &str =
    "If you think this is full of Redneck and meth jokes then you'd be correct.";

fn services_text(s: &Services) -> String {
    let have: Vec<&str> = [
        (s.fuel, "gas"),
        (s.food, "food"),
        (s.motel, "beds"),
        (s.mechanic, "mechanic"),
        (s.outfitter, "outfitter"),
    ]
    .iter()
    .filter_map(|&(yes, name)| yes.then_some(name))
    .collect();
    if have.is_empty() {
        "no services".into()
    } else {
        have.join(", ")
    }
}

/// "sagebrush basin, then mountain pass": the country a road crosses.
fn country_of(road: &Road) -> String {
    let mut seen: Vec<&str> = Vec::new();
    for &(terrain, _) in &road.country {
        if !seen.contains(&terrain.label()) {
            seen.push(terrain.label());
        }
    }
    if seen.is_empty() {
        return road.terrain.label().into();
    }
    seen.join(", then ")
}

/// The true story and the made-up one, clearly labelled.
fn stories(p: &mut Page, node: &Node) {
    p.text("TRUE STORY", Tone::Label);
    p.text(&node.fact, Tone::Body);
    p.text("LOCAL LEGEND (we made this up)", Tone::Label);
    p.text(&node.legend, Tone::Joke);
    p.body.push(Block::Gap);
}

impl App {
    pub(super) fn build_page(&self) -> Page {
        let mut p = match &self.view {
            View::Title => self.title_page(),
            View::Map => self.map_page(),
            View::Place(id) => self.place_page(id),
            View::Credits => self.credits_page(),
            View::Game if self.drive.is_some() => self.driving_page(),
            View::Game => self.game_page(),
            View::Gather => self.gather_page(),
            View::Stats => self.stats_page(),
            View::Scores => self.scores_page(),
            View::Pause => self.pause_page(),
            View::Help => self.help_page(),
        };
        // Setup questions can be revisited.
        let screen = self.game.state.screen;
        if self.view == View::Game && screen.is_setup() && screen != Screen::Profession {
            p.button("Go back", "Change your last answer", Action::SetupBack);
            p.back = Some(Action::SetupBack);
        }
        p
    }

    /// What running out of fuel costs, the way the party is travelling.
    fn dry_warning(&self) -> String {
        let ride = self.game.ride_def();
        let cost = format!("{:.0}", ride.rescue_cost);
        ride.dry_warning.replace("{cost}", &cost)
    }

    pub(super) fn camp_detail(&self) -> String {
        let grazes = self.game.ride_def().graze > 0.0
            && self
                .game
                .season(self.game.state.day)
                .is_none_or(|s| s.forage);
        let extra = if grazes { " Dolly grazes." } else { "" };
        format!("Free, but uses a day. Some rest; mood drops.{extra}")
    }

    /// "Tap" for fingers, "Click" for a mouse.
    fn press_word(&self) -> &'static str {
        if self.touch {
            "Tap"
        } else {
            "Click"
        }
    }

    fn title_page(&self) -> Page {
        let mut p = Page {
            id: "title".into(),
            ..Page::default()
        };
        p.head = vec![
            Block::Text(OPENING_LINE.into(), Tone::Banner),
            Block::Text("WYOMING TRAIL".into(), Tone::Heading),
        ];
        p.side.push(Block::Photo("grand_teton".into()));
        p.text(
            "Four questionable people, one worse vehicle, and a mission somewhere \
             across Wyoming. All roads start in Lander.",
            Tone::Body,
        );
        p.text(
            "Pick a job. Get your mission. Buy your junk. Try not to lose anyone.",
            Tone::Dim,
        );
        p.body.push(Block::Gap);
        if self.has_save {
            p.button(
                "Continue trip",
                "Pick up where you left off",
                Action::Continue,
            );
        }
        p.button("New trip", "Fresh poor decisions", Action::NewRun);
        p.button(
            "Look at the map",
            "Every stop, with photos",
            Action::ShowMap,
        );
        p.button("Top ten", "The best finished missions", Action::ShowScores);
        p.button("Credits", "Photos, maps and facts", Action::ShowCredits);
        p
    }

    fn map_page(&self) -> Page {
        let mut p = Page {
            id: "map".into(),
            back: Some(Action::Back),
            ..Page::default()
        };
        p.head.push(Block::Text("WYOMING".into(), Tone::Heading));
        p.text(
            format!(
                "{} a dot to see the town, its photo and its stories.",
                self.press_word()
            ),
            Tone::Dim,
        );
        p.button("Back", "", Action::Back);
        p.side.push(Block::Map);
        p
    }

    fn place_page(&self, id: &str) -> Page {
        let mut p = Page {
            id: format!("place:{id}"),
            back: Some(Action::Back),
            ..Page::default()
        };
        let Some(node) = self.game.content.map.node(id) else {
            p.button("Back", "", Action::Back);
            return p;
        };
        p.head.push(Block::Text(node.name.clone(), Tone::Heading));
        p.head.push(Block::Text(
            format!("{}  ·  {}", node.kind, services_text(&node.services)),
            Tone::Dim,
        ));
        p.side.push(Block::Photo(node.id.clone()));
        // Offer the drive if this card was opened mid-trip next door.
        let stopped = matches!(self.game.state.screen, Screen::Town | Screen::RoadChoice);
        let in_game = self.history.contains(&View::Game);
        if stopped && in_game {
            let from = &self.game.state.current_node;
            for (i, road) in self.game.content.map.roads.iter().enumerate() {
                if road.from == *from && road.to == id && !road.closed {
                    p.button(
                        format!("Drive there on {}", road.highway),
                        format!("{} miles", road.distance_miles),
                        Action::ChooseRoad(i),
                    );
                }
            }
        }
        let adjacent = p.button_count() > 0;
        if stopped && in_game && !adjacent {
            self.head_for(&mut p, id);
        }
        p.button("Back", "", Action::Back);
        p.body.push(Block::Gap);
        stories(&mut p, node);
        p
    }

    /// The first leg of the shortest way to a town further off.
    fn head_for(&self, p: &mut Page, id: &str) {
        let map = &self.game.content.map;
        let here = &self.game.state.current_node;
        let target = [id.to_string()];
        let Some(path) = map.route(here, &target) else {
            return;
        };
        let Some((&first, miles)) = path.first().zip(map.miles_to(here, &target)) else {
            return;
        };
        let Some(road) = map.road(first) else {
            return;
        };
        let next = map.node(&road.to).map_or("", |n| n.name.as_str());
        p.button(
            "Head this way",
            format!(
                "{miles:.0} miles over {} road(s). First: {} to {next}.",
                path.len(),
                road.highway
            ),
            Action::ChooseRoad(first),
        );
    }

    fn credits_page(&self) -> Page {
        let mut p = Page {
            id: "credits".into(),
            back: Some(Action::Back),
            ..Page::default()
        };
        p.head.push(Block::Text("CREDITS".into(), Tone::Heading));
        p.button("Back", "", Action::Back);
        p.text(
            "Map: USGS The National Map (public domain). Road routes: OSRM, from \
             map data © OpenStreetMap contributors (ODbL). Facts: summarised from \
             English Wikipedia (CC BY-SA 4.0). Jokes, legends, prices, services \
             and events are invented for the game.",
            Tone::Body,
        );
        p.text(
            "Art (all CC0): mountain and tree layers by ansimuz (Luis Zuno); \
             mountain and cloud layers by FabinhoSC; traffic, hitchhiker and \
             road signs by Kenney (kenney.nl); deer and rabbit animation by \
             ScratchIO (elk and pronghorn repainted from the deer). Font: \
             Jersey 15 (SIL Open Font License). Panels and buttons: Simple RPG \
             GUI by ermizidis.stan.",
            Tone::Body,
        );
        p.text(
            "Characters (CC-BY 3.0): RPG Sprite Set 1 by Antifarea, \
             opengameart.org/content/antifareas-rpg-sprite-set-1-enlarged-w-\
             transparent-background-fixed (repacked into one sheet). Cowboy with \
             rifle (CC-BY 3.0) by Krasi Wasilev, freegameassets.blogspot.com.",
            Tone::Body,
        );
        p.text(
            "Photos from Wikimedia Commons, resized. Source links are in \
             assets/data/credits.json.",
            Tone::Body,
        );
        for c in &self.game.content.credits {
            // A stop, a plant or a backdrop.
            let name = self
                .game
                .content
                .map
                .node(&c.id)
                .map(|n| n.name.as_str())
                .or_else(|| {
                    let kinds = &self.game.content.gatherables;
                    let plant = kinds.iter().find(|g| g.photo.as_ref() == Some(&c.id));
                    plant.map(|g| g.name.as_str())
                })
                .or_else(|| {
                    let field = self.game.content.fields.iter().find(|f| f.id == c.id);
                    field.map(|f| f.place.as_str())
                })
                .unwrap_or(&c.id);
            p.text(format!("{name}: {} · {}", c.author, c.license), Tone::Dim);
        }
        p
    }

    fn game_page(&self) -> Page {
        let s = &self.game.state;
        let mut p = Page {
            id: format!("{:?}:{}", s.screen, s.current_node),
            ..Page::default()
        };
        match s.screen {
            Screen::Profession => self.profession_page(&mut p),
            Screen::Party => self.party_page(&mut p),
            Screen::Season => self.season_page(&mut p),
            Screen::Trade => self.trade_page(&mut p),
            Screen::Work => self.work_page(&mut p),
            Screen::Hitchhiker => self.hitchhiker_page(&mut p),
            Screen::Outfit => self.truck_page(&mut p),
            Screen::Store => self.store_page(&mut p),
            Screen::Town => self.town_page(&mut p),
            Screen::RoadChoice => self.road_page(&mut p),
            Screen::Travel => self.travel_page(&mut p),
            Screen::Event => self.event_page(&mut p),
            Screen::Win | Screen::GameOver => self.end_page(&mut p),
        }
        if s.screen != Screen::Profession && !s.screen.is_over() {
            self.mission_banner(&mut p);
        }
        p
    }

    /// The mission's goals, met or not, at the top of every screen of a run.
    fn mission_banner(&self, p: &mut Page) {
        if self.game.current_step().is_none() {
            return;
        }
        // On a small screen the setup questions and the road keep only the goal
        // in hand, so that their buttons stay in view.
        let screen = self.game.state.screen;
        let busy = screen.is_setup() || screen == Screen::Travel;
        let brief = self.width < super::draw::WIDE && (self.drive.is_some() || busy);
        self.checklist(p, brief);
        let left = self.game.days_left();
        let tone = if left <= 2 { Tone::Warn } else { Tone::Label };
        let count = self.game.job().and_then(|j| j.goal.countdown.as_deref());
        let count = count.unwrap_or("DAYS LEFT TO FINISH");
        p.head.push(Block::Text(format!("{count}: {left}"), tone));
    }

    /// The goals into the page head: all of them, or the count and the one in
    /// hand.
    pub(super) fn checklist(&self, p: &mut Page, brief: bool) {
        let goals = self.game.goals();
        let met = goals.iter().filter(|g| g.1 == GoalState::Done).count();
        if goals.len() > 1 {
            let title = format!("GOALS: {met} of {} met", goals.len());
            p.head.push(Block::Text(title, Tone::Label));
        }
        let over = self.game.state.screen.is_over();
        for (task, state) in goals {
            if brief && state != GoalState::Current {
                continue;
            }
            let (mut line, tone) = match state {
                GoalState::Done => (format!("[X] MET: {task}"), Tone::GoalMet),
                _ if over => (format!("[ ] NOT MET: {task}"), Tone::Warn),
                GoalState::Current => (format!("[ ] NOW: {task}"), Tone::Mission),
                GoalState::Later => (format!("[ ] LATER: {task}"), Tone::GoalLater),
            };
            // While driving, the progress bar shows the live distance instead.
            let parked = self.game.state.en_route.is_none() && !over;
            if let (GoalState::Current, true, Some(step)) =
                (state, parked, self.game.current_step())
            {
                let from = &self.game.state.current_node;
                if let Some(miles) = self.game.content.map.miles_to(from, &step.at) {
                    line.push_str(&format!("  ({miles:.0} miles away)"));
                }
            }
            p.head.push(Block::Text(line, tone));
        }
    }

    fn heading(p: &mut Page, title: impl Into<String>, sub: impl Into<String>) {
        p.head.push(Block::Text(title.into(), Tone::Heading));
        p.head.push(Block::Text(sub.into(), Tone::Dim));
    }

    fn log(&self, p: &mut Page) {
        for line in &self.game.state.log {
            p.text(line, Tone::Body);
        }
        if !self.game.state.log.is_empty() {
            p.body.push(Block::Gap);
        }
    }

    fn profession_page(&self, p: &mut Page) {
        Self::heading(
            p,
            "WHAT DO YOU DO FOR A LIVING?",
            "Each job has its own mission. Less money, bigger score.",
        );
        // Side by side where there's room, so the two crowds can be compared;
        // otherwise one after the other.
        let wide = self.width >= super::draw::WIDE;
        if !wide {
            p.side
                .push(Block::Photo(self.game.content.map.start.clone()));
        }
        for (crew, title) in [
            (Crew::Wyoming, "REDNECK JOBS"),
            (Crew::Hippie, "HIPPIE JOBS"),
        ] {
            let column = if wide && crew == Crew::Hippie {
                &mut p.side
            } else {
                &mut p.body
            };
            column.push(Block::Text(title.into(), Tone::Section));
            let jobs = self.game.content.professions.iter().enumerate();
            for (i, job) in jobs.filter(|(_, j)| j.crew == crew) {
                column.push(self.job_card(i, job));
            }
        }
        // Last in whichever column is read last.
        let last = if wide { &mut p.side } else { &mut p.body };
        last.push(Block::Button {
            label: "Main menu".into(),
            detail: String::new(),
            action: Action::MainMenu,
        });
    }

    /// One job, set out so the choices can be compared at a glance.
    fn job_card(&self, i: usize, job: &ProfessionDef) -> Block {
        let miles = self.game.mission_miles(i).unwrap_or(0.0);
        let food = match job.gather {
            GatherMode::Poach => {
                "Poaching. Lots of meat, but game wardens hand out fines up to $950."
            }
            GatherMode::Forage | GatherMode::Scavenge => {
                "Foraging, and dumpster diving in winter. Smaller hauls, safer."
            }
        };
        let mut sections = vec![
            (
                "MISSION",
                format!(
                    "{}\n{miles:.0} miles  ·  {}",
                    job.goal.title,
                    days(job.goal.days)
                ),
            ),
            ("YOUR EDGE", job.perk_text.clone()),
            ("WILD FOOD", food.to_string()),
        ];
        let issued = job.mount.as_ref().and_then(|id| {
            let vehicles = &self.game.content.vehicles;
            vehicles.iter().find(|v| v.id == *id)
        });
        if let Some(mount) = issued {
            sections.push(("YOUR RIDE", format!("{}. {}", mount.name, mount.blurb)));
        }
        sections.push(("YOUR STORY", job.joke.clone()));
        Block::Card {
            title: job.name.clone(),
            summary: format!("${:.0} to start  ·  score ×{}", job.money, job.multiplier),
            sections,
            action: Action::ChooseProfession(i),
        }
    }

    fn party_page(&self, p: &mut Page) {
        let sub = format!("{} a name to change it.", self.press_word());
        Self::heading(p, "WHO'S RIDING ALONG?", sub);
        p.side
            .push(Block::Photo(self.game.content.map.start.clone()));
        let female = self.game.state.female;
        if let Some(job) = self.game.job() {
            let look = self.game.content.look(job.look(female));
            p.body.push(Block::Specimen {
                sprite: job.hero_sprite(female, "down"),
                frame: look.map_or(0, |l| l.stand),
                note: format!("This is you: {}", if female { "a woman" } else { "a man" }),
            });
        }
        p.button(
            if female {
                "Play as a man instead"
            } else {
                "Play as a woman instead"
            },
            "Changes your character when poaching or foraging",
            Action::SetFemale(!female),
        );
        for (i, m) in self.game.state.party.members.iter().enumerate() {
            let role = if i == 0 { "that's you" } else { "passenger" };
            p.button(&m.name, role, Action::Rename(i));
        }
        p.body.push(Block::Gap);
        p.default_selected = p.button_count();
        p.button(
            "That's everybody",
            "Next: when to leave",
            Action::ConfirmParty,
        );
    }

    fn season_page(&self, p: &mut Page) {
        Self::heading(
            p,
            "WHEN DO YOU LEAVE?",
            "The month decides the weather. Winter is slow and mean.",
        );
        p.side
            .push(Block::Photo(self.game.content.map.start.clone()));
        for (i, name) in MONTHS.iter().enumerate() {
            let month = i as u32 + 1;
            let mut hint = self.game.month_outlook(month).join(", ");
            let forager = self
                .game
                .job()
                .is_some_and(|j| j.gather == GatherMode::Forage);
            let bare = self
                .game
                .content
                .scenery
                .season(month)
                .is_some_and(|s| !s.forage);
            if forager && bare {
                hint.push_str(". No foraging: dumpster diving only");
            }
            if month == crate::core::config::DEFAULT_MONTH {
                hint.push_str(" (a good choice)");
            }
            p.button(*name, hint, Action::ChooseMonth(month));
        }
        p.default_selected = crate::core::config::DEFAULT_MONTH as usize - 1;
    }

    fn hitchhiker_page(&self, p: &mut Page) {
        p.id = format!("hitchhiker:{}", self.game.state.day);
        Self::heading(
            p,
            "A HOBO NEEDS A RIDE",
            format!("Day {}", self.game.state.day),
        );
        p.side.push(match &self.game.state.en_route {
            Some(_) => Block::Scene,
            None => Block::Photo(self.game.state.current_node.clone()),
        });
        p.body.push(Block::Specimen {
            sprite: self.game.content.scenery.hitchhiker.clone(),
            frame: 0,
            note: "A traveller with a bindle, a big smile and a slightly green complexion.".into(),
        });
        p.text(
            "Could go great. Could go all over your upholstery.",
            Tone::Joke,
        );
        p.button(
            "Pull over and pick them up",
            "Anything might happen",
            Action::PickUp(true),
        );
        p.default_selected = p.button_count();
        p.button("Keep driving", "Nothing happens", Action::PickUp(false));
        // The question first; the day's news underneath.
        p.body.push(Block::Gap);
        self.log(p);
    }

    /// Today's date and weather.
    fn forecast(&self, p: &mut Page) {
        let day = self.game.state.day;
        let (month, date) = self.game.date(day);
        let Some(w) = self.game.weather_on(day) else {
            return;
        };
        let tone = if w.speed < 0.9 { Tone::Warn } else { Tone::Dim };
        let name = MONTHS[(month as usize - 1) % 12];
        let feels = self.game.weather_text(w);
        p.text(format!("{name} {date}: {}. {feels}", w.name), tone);
    }

    fn truck_page(&self, p: &mut Page) {
        let money = self.game.state.resources.money;
        Self::heading(
            p,
            "HONEST HANK'S CAR LOT",
            format!("You have ${money:.0}. Save some for gas and food."),
        );
        p.side
            .push(Block::Photo(self.game.content.map.start.clone()));
        let lot = self.game.content.vehicles.iter().enumerate();
        for (i, t) in lot.filter(|(_, t)| t.for_sale) {
            let short = if money < t.price {
                "  (too rich for you)"
            } else {
                ""
            };
            p.button(
                format!("{} — ${:.0}{short}", t.name, t.price),
                format!(
                    "{} gal tank, {} mpg, {:.0} mile range. {}",
                    t.tank_gallons,
                    t.mpg,
                    t.tank_gallons * t.mpg,
                    t.blurb
                ),
                Action::BuyTruck(i),
            );
        }
    }

    fn fuel_button(&self, p: &mut Page) {
        if let Some(price) = self.game.fuel_price() {
            let t = &self.game.state.truck;
            let ride = self.game.ride_def();
            let need = t.tank_gallons - t.fuel_gallons;
            let label = if need < 0.05 {
                ride.full.clone()
            } else {
                format!("{} — ${:.0}", ride.fill, need * price)
            };
            p.button(
                label,
                format!(
                    "{:.1} of {} {unit} now, ${price:.2} per {unit}",
                    t.fuel_gallons,
                    t.tank_gallons,
                    unit = ride.unit
                ),
                Action::FillTank,
            );
        }
    }

    fn store_page(&self, p: &mut Page) {
        let s = &self.game.state;
        let place = self.game.node().map_or("", |n| n.name.as_str());
        Self::heading(
            p,
            format!("GENERAL STORE — {place}"),
            format!("Cash: ${:.2}", s.resources.money),
        );
        p.side.push(Block::Photo(s.current_node.clone()));
        self.log(p);
        let daily = self.game.daily_food();
        p.text(
            format!(
                "Your crew eats {daily} servings a day. You have {}. {}",
                s.resources.food,
                self.dry_warning()
            ),
            Tone::Dim,
        );
        // Before the first mile, say what the trip asks of the tank.
        let setting_off = s.day == 1 && s.current_node == self.game.content.map.start;
        if let Some(miles) = self
            .game
            .mission_miles(s.profession)
            .filter(|_| setting_off)
        {
            let days = self.game.days_left();
            p.text(
                format!(
                    "Your mission is about {miles:.0} miles, with {days} days to do it. \
                     Your {} reaches {:.0} miles right now.",
                    self.game.ride_def().fuel,
                    self.game.range_miles()
                ),
                Tone::Label,
            );
        }
        self.fuel_button(p);
        for (id, item) in self.game.stock() {
            let have = if item.kind == ItemKind::Food {
                s.resources.food
            } else {
                s.resources.count(id)
            };
            let price = self.game.item_price(item);
            let note = if s.resources.money < price {
                "  (can't afford)"
            } else {
                ""
            };
            p.button(
                format!("{} — ${price:.0}{note}", item.name),
                format!("{} {}\nYou have {have}.", item.helps, item.desc),
                Action::Buy(id.to_string()),
            );
        }
        let mut missing = Vec::new();
        if s.resources.food < daily {
            missing.push("food for today");
        }
        if setting_off && s.truck.fuel_gallons + 0.05 < s.truck.tank_gallons {
            missing.push(&self.game.ride_def().full_need);
        }
        let done = if missing.is_empty() {
            "Back to the road".to_string()
        } else {
            format!("Careful: you don't have {}", missing.join(" or "))
        };
        p.button("Done shopping", done, Action::LeaveStore);
        p.back = Some(Action::LeaveStore);
    }

    fn trade_page(&self, p: &mut Page) {
        let s = &self.game.state;
        let place = self.game.node().map_or("", |n| n.name.as_str());
        Self::heading(
            p,
            format!("BACK-DOOR TRADING — {place}"),
            "Cash only. No questions, a few opinions.",
        );
        p.side.push(Block::Photo(s.current_node.clone()));
        self.log(p);
        let wild = s.resources.wild;
        p.text(
            format!(
                "Wild food on hand: {wild} servings. Groceries from the store can't be resold."
            ),
            Tone::Dim,
        );
        let mut lots = vec![10, wild];
        lots.dedup();
        for n in lots.into_iter().filter(|&n| n > 0 && n <= wild) {
            p.button(
                format!(
                    "Sell {n} servings — get ${:.0}",
                    f64::from(n) * WILD_FOOD_PRICE
                ),
                "Meat, berries, whatever you dragged in",
                Action::SellFood(n),
            );
        }
        if wild == 0 && s.resources.inventory.values().all(|&n| n == 0) {
            p.text(
                "Nothing to sell yet. Come back with wild food or spare gear.",
                Tone::Body,
            );
        }
        for id in self.game.content.items.ids() {
            let Some((name, units, cash)) = self.game.pawn_offer(id) else {
                continue;
            };
            let have = s.resources.count(id);
            if have >= units {
                p.button(
                    format!("Pawn: {name} — get ${cash:.0}"),
                    format!("You have {have}."),
                    Action::SellItem(id.to_string()),
                );
            }
        }
        p.button("Done trading", "", Action::LeaveStore);
        p.back = Some(Action::LeaveStore);
    }

    fn work_page(&self, p: &mut Page) {
        let place = self.game.node().map_or("", |n| n.name.as_str());
        Self::heading(
            p,
            format!("HELP WANTED — {place}"),
            "Money now, but the clock keeps running.",
        );
        p.side
            .push(Block::Photo(self.game.state.current_node.clone()));
        self.log(p);
        for gig in self.game.offers() {
            let mut pitch = gig.pitch.clone();
            if gig.days >= self.game.days_left() {
                pitch.push_str("\nWARNING: your mission would run out of time.");
            }
            p.button(
                format!("{} — ${:.0}, {}", gig.name, gig.pay, days(gig.days)),
                pitch,
                Action::Work(gig.id.clone()),
            );
        }
        p.button("Never mind", "", Action::LeaveStore);
        p.back = Some(Action::LeaveStore);
    }

    fn party_status(&self, p: &mut Page) {
        for m in &self.game.state.party.members {
            if m.in_trip() {
                p.text(
                    format!(
                        "{}: health {:.0}, tired {:.0}, mood {:.0}",
                        m.name, m.health, m.fatigue, m.mood
                    ),
                    if m.health < 40.0 {
                        Tone::Warn
                    } else {
                        Tone::Dim
                    },
                );
            } else {
                p.text(format!("{}: left the trip", m.name), Tone::Dim);
            }
        }
        p.body.push(Block::Gap);
    }

    /// Shop, pump and beds — shared by the town and road screens.
    fn stop_buttons(&self, p: &mut Page) {
        self.fuel_button(p);
        if !self.game.stock().is_empty() {
            p.button("Shop", "Supplies for the road", Action::OpenStore);
        }
        if self.game.node().is_some_and(|n| n.services.motel) {
            let broke = self.game.state.resources.money < MOTEL_COST;
            let note = if broke { "  (can't afford)" } else { "" };
            p.button(
                format!("Motel night — ${MOTEL_COST:.0}{note}"),
                "Uses a day. Heals and rests the crew well.",
                Action::Rest(true),
            );
        }
        p.button(
            self.game.ride_def().camp.clone(),
            self.camp_detail(),
            Action::Rest(false),
        );
        if self.game.can_trade() {
            p.button(
                "Trade or pawn",
                format!(
                    "Sell wild food ({} servings) and spare gear",
                    self.game.state.resources.wild
                ),
                Action::OpenTrade,
            );
        }
        if !self.game.offers().is_empty() {
            p.button("Find work", "Earn money; uses up days", Action::OpenWork);
        }
        self.gather_button(p);
    }

    fn town_page(&self, p: &mut Page) {
        let Some(node) = self.game.node() else {
            return;
        };
        Self::heading(
            p,
            node.name.clone(),
            format!("You made it. Here: {}.", services_text(&node.services)),
        );
        p.side.push(Block::Photo(node.id.clone()));
        p.side.push(Block::Map);
        self.log(p);
        // The way onward comes before the reading.
        p.button(
            "Pick the next road",
            "Shop and beds are there too",
            Action::ContinueJourney,
        );
        stories(p, node);
        self.stop_buttons(p);
        p.button("Stats", "Bar graphs of the trip so far", Action::ShowStats);
        p.button("Main menu", "Your trip is saved", Action::MainMenu);
        self.party_status(p);
    }

    fn road_page(&self, p: &mut Page) {
        let Some(node) = self.game.node() else {
            return;
        };
        let s = &self.game.state;
        Self::heading(
            p,
            format!("WHICH WAY FROM {}?", node.name.to_uppercase()),
            format!("Pick a road. Here: {}.", services_text(&node.services)),
        );
        // On a small screen the picture waits below, so the roads come sooner.
        let narrow = self.width < super::draw::WIDE;
        if !narrow {
            p.side.push(Block::Photo(node.id.clone()));
        }
        p.side.push(Block::Map);
        self.log(p);
        self.forecast(p);
        let opening = s.day == 1 && node.id == self.game.content.map.start;
        if s.resources.food < self.game.daily_food() {
            p.text("You're about out of food.", Tone::Warn);
        }
        let range = self.game.range_miles();
        let advised = self
            .game
            .content
            .map
            .route(&node.id, self.game.targets())
            .and_then(|path| path.first().copied());
        let mut roads: Vec<usize> = (0..self.game.content.map.roads.len())
            .filter(|&i| {
                let road = &self.game.content.map.roads[i];
                road.from == node.id && !road.closed
            })
            .collect();
        // The recommended road leads, so it needs no scrolling to find.
        roads.sort_by_key(|&i| advised != Some(i));
        // The first road button is the default selection.
        p.default_selected = 0;
        for i in roads {
            let road = &self.game.content.map.roads[i];
            let Some(dest) = self.game.content.map.node(&road.to) else {
                continue;
            };
            let mut detail = format!(
                "{} miles, {}. At the other end: {}.",
                road.distance_miles,
                country_of(road),
                services_text(&dest.services)
            );
            if road.seasonal_risk >= 3 {
                detail.push_str(" Risky road.");
            }
            if road.distance_miles > range {
                let fuel = self.game.ride_def().fuel.to_uppercase();
                detail.push_str(&format!(" NOT ENOUGH {fuel}."));
            }
            if !s.visited.contains(&dest.id) {
                detail.push_str(" New to you.");
            }
            let mut label = format!("{} to {}", road.highway, dest.name);
            if advised == Some(i) {
                label.push_str("  ★");
                detail.push_str("\n★ Shortest way to your mission");
            }
            p.button(label, detail, Action::ChooseRoad(i));
        }
        p.body.push(Block::Gap);
        self.stop_buttons(p);
        p.button("Stats", "Bar graphs of the trip so far", Action::ShowStats);
        p.button("Map", self.map_hint(), Action::ShowMap);
        p.button("Main menu", "Your trip is saved", Action::MainMenu);
        self.party_status(p);
        if narrow {
            p.body.push(Block::Photo(node.id.clone()));
        }
        // The send-off story reads fine after the choices.
        if opening {
            stories(p, node);
        }
    }

    fn travel_page(&self, p: &mut Page) {
        let s = &self.game.state;
        let Some(en) = &s.en_route else {
            return;
        };
        let dest = self.game.content.map.node(&en.destination);
        let name = dest.map_or(en.destination.as_str(), |n| n.name.as_str());
        // One page per leg, so selection and scroll hold between days.
        p.id = format!("travel:{}", en.road);
        Self::heading(
            p,
            format!("PULLED OVER ON THE WAY TO {}", name.to_uppercase()),
            format!(
                "{:.0} of {:.0} miles to go",
                en.remaining_miles, en.total_miles
            ),
        );
        let short = self.game.range_miles() < en.remaining_miles;
        // Stuck for fuel on a small screen, the way out comes above the view.
        let view_below = short && self.width < super::draw::WIDE;
        if !view_below {
            p.side.push(Block::Scene);
        }
        p.side.push(Block::Map);
        // First, so it is in the same place every day. A tank that won't last
        // says so right here, with the way back beside it.
        let ride = self.game.ride_def();
        if short {
            p.text(
                format!(
                    "NOT MOVING: not enough {} to finish this road. {}",
                    ride.fuel,
                    self.dry_warning()
                ),
                Tone::Warn,
            );
        }
        let (label, why) = if short {
            (format!("{} anyway", ride.verb), "Risk running dry")
        } else {
            (
                ride.verb.clone(),
                "Back on the road until you arrive, stop or something happens",
            )
        };
        p.button(label, why, Action::DriveDay);
        let can_turn_back = self.game.can_turn_back();
        if can_turn_back && short {
            p.button(
                format!("Go back and get more {}", ride.fuel),
                "Costs nothing: you haven't left yet",
                Action::TurnBack,
            );
        }
        if view_below {
            p.body.push(Block::Scene);
        }
        self.log(p);
        self.forecast(p);
        if s.resources.food < self.game.daily_food() {
            p.text("Not enough food for today.", Tone::Warn);
        }
        let pace = PACES[s.pace_index % PACES.len()];
        let meal = MEALS[s.meal_index % MEALS.len()];
        self.gather_button(p);
        p.button(
            format!("Pace: {}", pace.name),
            format!(
                "{} hours a day. {} {} to change.",
                pace.hours,
                pace.note,
                self.press_word()
            ),
            Action::CyclePace,
        );
        p.button(
            format!("Rations: {}", meal.name),
            format!(
                "{} per person a day. {} {} to change.",
                meal.servings,
                meal.note,
                self.press_word()
            ),
            Action::CycleMeal,
        );
        if can_turn_back && !short {
            p.button(
                "Pick a different road",
                "Free until you start driving",
                Action::TurnBack,
            );
        }
        p.button("Stats", "Bar graphs of the trip so far", Action::ShowStats);
        p.button("Map", self.map_hint(), Action::ShowMap);
        p.button("Main menu", "Your trip is saved", Action::MainMenu);
        p.body.push(Block::Gap);
        self.party_status(p);
        p.body.push(Block::Photo(en.destination.clone()));
    }

    fn map_hint(&self) -> &'static str {
        if self.touch {
            "Look around"
        } else {
            "Look around (M)"
        }
    }

    fn gather_button(&self, p: &mut Page) {
        let Some(mode) = self.game.gather_mode() else {
            return;
        };
        match mode {
            GatherMode::Poach => {
                let ammo = self.game.ammo();
                let detail = if ammo == 0 {
                    "No ammo. Buy a box at a pump or outfitter.".to_string()
                } else {
                    format!("{ammo} rounds left. Riskier in towns and parks.")
                };
                p.button("Go poaching (one day)", detail, Action::StartGather);
            }
            GatherMode::Forage => p.button(
                "Go foraging (one day)",
                "Wild food, free. Mind the death camas.",
                Action::StartGather,
            ),
            GatherMode::Scavenge => match self.game.gather_blocked() {
                Some(why) => p.button("Go dumpster diving (not here)", why, Action::StartGather),
                None => p.button(
                    "Go dumpster diving (one day)",
                    "Winter groceries, free. Skip the burrito.",
                    Action::StartGather,
                ),
            },
        }
    }

    fn gather_page(&self) -> Page {
        let mut p = Page {
            id: "gather".into(),
            // Esc pauses a running round; it doesn't throw it away.
            back: Some(if self.round_started {
                Action::Pause
            } else {
                Action::CancelGather
            }),
            ..Page::default()
        };
        let Some(round) = &self.round else {
            p.button("Back", "", Action::FinishGather);
            return p;
        };
        let poach = round.mode == GatherMode::Poach;
        p.head.push(Block::Text(
            match round.mode {
                GatherMode::Poach => "POACHING",
                GatherMode::Forage => "FORAGING",
                GatherMode::Scavenge => "DUMPSTER DIVING",
            }
            .into(),
            Tone::Heading,
        ));
        self.checklist(&mut p, true);
        p.head.push(Block::Field);
        if round.over() {
            p.id = "gather-done".into();
            p.text(
                if poach {
                    format!(
                        "Time to go. Shot {} lb of meat, carrying {} lb ({} servings).",
                        round.meat_shot,
                        round.carried(),
                        round.food
                    )
                } else {
                    format!("Done. {} servings in the bag.", round.food)
                },
                Tone::Mission,
            );
            p.button(self.game.ride_def().back.clone(), "", Action::FinishGather);
            return p;
        }
        if !self.round_started {
            p.id = "gather-ready".into();
            let start = if round.mode == GatherMode::Scavenge {
                "Jump in"
            } else {
                "Start"
            };
            p.button(start, "The clock starts now", Action::BeginGather);
            p.button("Not now", "No day or ammo used", Action::CancelGather);
        } else {
            p.text(
                match (poach, self.touch) {
                    (true, true) => "Hold to walk. Tap to shoot.",
                    (true, false) => "Arrows or WASD to walk. Click or Space to shoot.",
                    (false, true) if round.mode == GatherMode::Scavenge => {
                        "Tap a pile to dig. Tap the find to take it."
                    }
                    (false, false) if round.mode == GatherMode::Scavenge => {
                        "Click a pile to dig, then again to take. Space works too."
                    }
                    (false, true) => "Tap something to walk over and take it.",
                    (false, false) => "Click something, or walk to it and press Space.",
                },
                Tone::Dim,
            );
            p.button("Pause", "Stops the clock", Action::Pause);
            p.button(
                "Head back now",
                "Ends the round and uses the day",
                Action::FinishGather,
            );
        }
        p.text(
            match round.mode {
                GatherMode::Poach => "IN SEASON (ACCORDING TO YOU)",
                GatherMode::Forage => {
                    "FIELD GUIDE  (a game, not advice: never eat wild plants on its say-so)"
                }
                GatherMode::Scavenge => "KNOW YOUR TRASH",
            },
            Tone::Label,
        );
        if let Some(field) = &round.backdrop {
            if let Some(c) = self.game.content.credits.iter().find(|c| c.id == field.id) {
                // Our own drawings need no photographer.
                if !c.source.is_empty() {
                    p.text(
                        format!(
                            "Backdrop: {}. Photo: {} · {} · Wikimedia Commons",
                            field.place, c.author, c.license
                        ),
                        Tone::Dim,
                    );
                }
            }
        }
        for kind in &round.kinds {
            let note = if kind.poison {
                format!("{}: POISON, leave it", kind.name)
            } else if !poach && kind.food == 0 {
                format!("{}: junk", kind.name)
            } else {
                if poach {
                    format!("{}: {} lb of meat", kind.name, kind.pounds)
                } else {
                    format!("{}: {} servings", kind.name, kind.food)
                }
            };
            p.body.push(match &kind.photo {
                Some(photo) => Block::Species {
                    photo: photo.clone(),
                    sprite: kind.sprite.clone(),
                    note: format!("{note}\n{}", kind.fact),
                },
                None => Block::Specimen {
                    sprite: kind.sprite.clone(),
                    frame: 0,
                    note: if poach {
                        note
                    } else {
                        format!("{note}\n{}", kind.fact)
                    },
                },
            });
        }
        p
    }

    /// Shown while a day's drive plays out.
    fn driving_page(&self) -> Page {
        let mut p = Page {
            id: "driving".into(),
            back: Some(Action::SkipDrive),
            ..Page::default()
        };
        if let Some(drive) = &self.drive {
            let name = self.game.content.map.node(&drive.to);
            let name = name.map_or(drive.to.as_str(), |n| n.name.as_str());
            Self::heading(
                &mut p,
                format!("{} TO {}", self.game.ride_def().going, name.to_uppercase()),
                format!("{} · day {}", drive.highway, drive.day),
            );
        }
        self.mission_banner(&mut p);
        p.side.push(Block::Scene);
        // Beside the road view where there's room, so the marker can be
        // watched moving along its highway.
        if self.width >= super::draw::WIDE {
            p.body.push(Block::Map);
        } else {
            p.side.push(Block::Map);
        }
        let going = if self.cruising {
            "Pull over when this day is done"
        } else {
            "Get on with it"
        };
        p.button("Stop here", going, Action::StopDriving);
        p.button("Pause", "Menu, rest and map", Action::Pause);
        p
    }

    fn event_page(&self, p: &mut Page) {
        let Some(event) = &self.game.state.event else {
            p.button("Continue", "", Action::ContinueJourney);
            return;
        };
        p.id = format!("event:{}", self.game.state.day);
        Self::heading(
            p,
            event.name.to_uppercase(),
            format!("Day {}", self.game.state.day),
        );
        p.side.push(Block::Map);
        p.text(&event.text, Tone::Joke);
        if !event.consequences.is_empty() {
            p.text(event.consequences.join("  ·  "), Tone::Warn);
        }
        p.body.push(Block::Gap);
        self.log(p);
        p.button("Continue", "", Action::ContinueJourney);
    }

    fn end_page(&self, p: &mut Page) {
        let s = &self.game.state;
        let won = s.screen == Screen::Win;
        Self::heading(
            p,
            if won {
                "MISSION COMPLETE!"
            } else {
                "MISSION FAILED"
            },
            s.ending_reason.clone(),
        );
        // A mission that ends in a drum circle starts it here.
        let finale = self.game.job().and_then(|j| j.goal.finale.as_ref());
        if let Some(finale) = finale.filter(|_| won) {
            p.body.push(Block::Finale(finale.clone()));
            if self.circle.complete() {
                p.text(&finale.won, Tone::Mission);
                p.button("Keep drumming", "Nobody wants it to stop", Action::Drum);
            } else {
                let how = "Tap the scene, or press this, as the ring closes";
                p.button(finale.invite.clone(), how, Action::Drum);
            }
        }
        // The result and what to do next, before the reading.
        let score = &s.score;
        p.text(format!("SCORE: {}", thousands(score.total)), Tone::Label);
        if won {
            match self.scores.rank(s.seed, score.total) {
                Some(rank) => p.text(
                    format!("You made the top ten: number {rank}!"),
                    Tone::Mission,
                ),
                None => p.text("Not quite a top ten score this time.", Tone::Dim),
            }
        }
        p.button("Play again", "New trip", Action::PlayAgain);
        p.button("Top ten", "The best finished missions", Action::ShowScores);
        p.button("Main menu", "", Action::MainMenu);
        p.body.push(Block::Gap);
        // On a small screen nothing may push a drum circle off the top.
        let circle_first = won && finale.is_some() && self.width < super::draw::WIDE;
        if circle_first {
            p.body.push(Block::Photo(s.current_node.clone()));
        } else {
            p.side.push(Block::Photo(s.current_node.clone()));
        }
        p.side.push(Block::Map);
        if let Some(event) = &s.event {
            p.text(&event.text, Tone::Joke);
        }
        self.log(p);
        if let Some(job) = self.game.job() {
            p.text(format!("{}: {}", job.name, job.goal.title), Tone::Mission);
        }
        p.text(
            format!("Days on the road: {}", score.days_traveled),
            Tone::Body,
        );
        p.text(
            format!(
                "Survived the company: {} of {}",
                score.party_survivors,
                s.party.members.len()
            ),
            Tone::Body,
        );
        p.text(format!("Cash left: ${:.2}", score.money_left), Tone::Body);
        p.text(
            format!("Places visited: {}", score.places_visited),
            Tone::Body,
        );
        if won {
            if let Some(node) = self.game.node() {
                stories(p, node);
            }
        }
    }
}

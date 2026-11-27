//! Balance: a sensible but fallible player runs every job many times,
//! gathering rounds and all. `cargo test balance_report -- --ignored
//! --nocapture` prints the table; the other test keeps the jobs in step.

use super::*;
use crate::core::defs::Crew;
use crate::core::gather::{Session, LANDSCAPE};

/// How the player handles a gathering round.
const AIM_ERROR: f64 = 11.0;
/// Chance a forager mistakes a poisonous find for a good one.
const MISTAKE: f64 = 0.07;

#[derive(Debug, Default, Clone)]
pub struct Tally {
    pub runs: u32,
    pub wins: u32,
    pub late: u32,
    pub broke_down: u32,
    pub crew_left: u32,
    pub stuck: u32,
    pub score: u64,
    pub days_used: f64,
    pub money_left: f64,
    pub outings: u32,
    pub servings: u32,
    pub fines: u32,
}

impl Tally {
    pub fn win_rate(&self) -> f64 {
        f64::from(self.wins) / f64::from(self.runs.max(1)) * 100.0
    }

    /// Average score over every run, losses counting nothing.
    pub fn expected_score(&self) -> f64 {
        self.score as f64 / f64::from(self.runs.max(1))
    }
}

/// Play a round the way a person might: decently, not perfectly.
fn play_round(mut round: Session, rng: &mut Rng) -> Session {
    const DT: f64 = 0.05;
    for _ in 0..2_000 {
        if round.over() {
            break;
        }
        let me = (round.x, round.y);
        let far = |t: &crate::core::gather::Thing| (t.x - me.0).hypot(t.y - me.1);
        if round.mode == GatherMode::Poach {
            let target = round
                .things
                .iter()
                .filter(|t| t.x > 0.0 && t.x < round.width)
                .min_by(|a, b| far(a).total_cmp(&far(b)))
                .cloned();
            let Some(t) = target else {
                round.update(DT, 0.0, 0.0);
                continue;
            };
            let distance = far(&t);
            if distance > 80.0 {
                round.update(DT, t.x - me.0, t.y - me.1);
                continue;
            }
            // A human takes a moment between shots, and misses some.
            if !round.aiming() && rng.chance(0.12) {
                let lead = distance / BULLET_SPEED;
                let miss = |rng: &mut Rng| (rng.unit() - 0.5) * 2.0 * AIM_ERROR;
                let at = (t.x + t.vx * lead + miss(rng), t.y + t.vy * lead + miss(rng));
                round.fire(Some(at));
            }
            round.update(DT, 0.0, 0.0);
        } else {
            if !round.moving {
                let careless = rng.chance(MISTAKE);
                let kinds = &round.kinds;
                let target = round
                    .things
                    .iter()
                    .filter(|t| {
                        let kind = &kinds[t.kind];
                        t.hidden || careless || (!kind.poison && kind.food > 0)
                    })
                    .min_by(|a, b| far(a).total_cmp(&far(b)))
                    .map(|t| (t.x, t.y));
                if let Some((x, y)) = target {
                    // A moment to look before walking over.
                    for _ in 0..14 {
                        round.update(DT, 0.0, 0.0);
                    }
                    round.go_to(x, y);
                }
            }
            round.update(DT, 0.0, 0.0);
        }
    }
    round
}

struct Player {
    g: Game,
    /// Skips the spares, buys little, dawdles and takes chances.
    careless: bool,
    rng: Rng,
    outings: u32,
    servings: u32,
    fines: u32,
}

impl Player {
    fn note_fines(&mut self) {
        let fined = self
            .g
            .state
            .log
            .iter()
            .any(|l| l.contains("ine:") || l.contains("fines you"));
        self.fines += u32::from(fined);
    }

    fn buy_up_to(&mut self, food: u32, keep: f64) {
        while self.g.state.resources.food < food && self.g.state.resources.money > keep {
            if self.g.buy("groceries").is_err() {
                break;
            }
        }
    }

    fn setup(&mut self, job: usize, month: u32) {
        let g = &mut self.g;
        g.choose_profession(job).unwrap();
        g.confirm_party().unwrap();
        g.choose_month(month).unwrap();
        if g.state.screen == Screen::Outfit {
            let pick = usize::from(g.state.resources.money >= 2_800.0);
            g.buy_truck(pick).unwrap();
        }
        if self.careless {
            let want = g.daily_food() * 2;
            self.buy_up_to(want, 0.0);
            self.g.leave_store().unwrap();
            return;
        }
        let _ = g.fill_tank();
        for id in [
            "spare_tire",
            "duct_tape",
            "jumper_cables",
            "road_atlas",
            "gas_can",
            "horseshoes",
            "hay_sack",
            "first_aid",
        ] {
            if self.g.state.resources.money > 400.0 {
                let _ = self.g.buy(id);
            }
        }
        let days = self.g.days_left().min(8);
        let want = self.g.daily_food() * days / 2 + 8;
        self.buy_up_to(want, 300.0);
        self.g.leave_store().unwrap();
    }

    fn tired(&self) -> bool {
        let crew = self.g.state.party.in_trip();
        if self.careless {
            // Stops for every yawn.
            return self.g.state.day.is_multiple_of(3) && crew.iter().any(|m| m.fatigue > 35.0);
        }
        crew.iter().any(|m| m.fatigue > 75.0 || m.health < 45.0)
    }

    /// Days still needed if nothing goes wrong, roughly.
    fn needed(&self) -> u32 {
        let from = match &self.g.state.en_route {
            Some(e) => e.destination.clone(),
            None => self.g.state.current_node.clone(),
        };
        let path = self.g.content.map.route(&from, self.g.targets());
        let roads = path.map_or(0, |p| p.len() as u32);
        let later = self
            .g
            .job()
            .map_or(0, |j| (j.goal.steps.len() - self.g.state.goal_step) as u32);
        roads + later
    }

    fn gather(&mut self) -> bool {
        if self.g.gather_blocked().is_some() {
            return false;
        }
        let Ok(round) = self.g.start_gather(LANDSCAPE) else {
            return false;
        };
        let round = play_round(round, &mut self.rng);
        self.outings += 1;
        self.servings += round.food;
        self.g.finish_gather(&round).unwrap();
        self.note_fines();
        true
    }

    fn at_a_stop(&mut self) {
        // The careless only think about fuel when the needle is low.
        let truck = &self.g.state.truck;
        let low = truck.fuel_gallons < truck.tank_gallons * 0.3;
        if !self.careless || low {
            let _ = self.g.fill_tank();
        }
        let daily = self.g.daily_food();
        if self.g.state.resources.food < daily * 2 && self.g.open_store().is_ok() {
            self.buy_up_to(daily * 4, 150.0);
            if self.g.ammo() < 4 && self.g.state.resources.money > 100.0 {
                let _ = self.g.buy("ammo");
            }
            if self.g.range_miles() < 120.0 {
                let _ = self.g.buy("hay_sack");
                let _ = self.g.buy("gas_can");
            }
            self.g.leave_store().unwrap();
        }
        let spare = self.g.days_left() > self.needed() + 1;
        if self.tired() && spare {
            if self.g.rest(true).is_err() {
                self.g.rest(false).unwrap();
            }
            return;
        }
        // Wild food when short, or to sell when the wallet is thin.
        let short = self.g.state.resources.food < daily;
        let poor = self.g.state.resources.money < 150.0;
        let poaching = self.g.gather_mode() == Some(GatherMode::Poach);
        let watched = self.g.node().is_some_and(|n| !n.hunting_allowed);
        let wary = !self.careless && poaching && watched;
        // The careless head out on a whim, deadline or not.
        let whim = self.careless && self.rng.chance(0.25);
        let wise = (short || (poor && spare)) && !wary && spare;
        if (wise || whim) && self.gather() {
            return;
        }
        if self.g.state.resources.money < 120.0 && spare && self.g.can_trade() {
            let wild = self.g.state.resources.wild;
            if wild > daily * 2 && self.g.open_trade().is_ok() {
                let _ = self.g.sell_food(wild - daily * 2);
                self.note_fines();
                if self.g.state.screen == Screen::Trade {
                    self.g.leave_store().unwrap();
                }
                return;
            }
            let best = self
                .g
                .offers()
                .into_iter()
                .filter(|o| o.days + self.needed() + 1 < self.g.days_left())
                .max_by(|a, b| (a.pay / f64::from(a.days)).total_cmp(&(b.pay / f64::from(b.days))))
                .map(|o| o.id.clone());
            if let Some(id) = best {
                self.g.open_work().unwrap();
                self.g.work(&id).unwrap();
                return;
            }
        }
        let node = self.g.state.current_node.clone();
        let path = self.g.content.map.route(&node, self.g.targets());
        match path.and_then(|p| p.first().copied()) {
            Some(road) => self.g.set_off(road).unwrap(),
            None => self.g.rest(false).unwrap(),
        }
    }

    fn step(&mut self) {
        match self.g.state.screen {
            Screen::Town | Screen::Event => self.g.continue_journey(),
            Screen::Hitchhiker => {
                let yes = self.rng.chance(if self.careless { 0.9 } else { 0.3 });
                self.g.pick_up(yes).unwrap();
            }
            Screen::RoadChoice => self.at_a_stop(),
            Screen::Travel => {
                let spare = self.g.days_left() > self.needed() + 1;
                let hungry = self.g.state.resources.food < self.g.daily_food();
                if self.tired() && spare {
                    self.g.rest(false).unwrap();
                } else if !(hungry && spare && self.gather()) {
                    self.g.drive_day().unwrap();
                }
            }
            _ => {}
        }
    }
}

fn play_as(
    content: &Rc<Content>,
    (job, month, seed): (usize, u32, u64),
    careless: bool,
    tally: &mut Tally,
) {
    let mut p = Player {
        g: Game::new(content.clone(), seed),
        careless,
        rng: Rng::for_day(seed ^ 0xBA1A, 1),
        outings: 0,
        servings: 0,
        fines: 0,
    };
    p.setup(job, month);
    for _ in 0..200 {
        if p.g.state.screen.is_over() {
            break;
        }
        p.step();
    }
    let g = &p.g;
    tally.runs += 1;
    tally.outings += p.outings;
    tally.servings += p.servings;
    tally.fines += p.fines;
    tally.money_left += g.state.resources.money;
    let allowed = g.job().map_or(1, |j| j.goal.days);
    match g.state.screen {
        Screen::Win => {
            tally.wins += 1;
            tally.score += g.state.score.total;
            tally.days_used += f64::from(g.state.day - 1) / f64::from(allowed);
        }
        Screen::GameOver => {
            let why = &g.state.ending_reason;
            if g.state.party.all_left() {
                tally.crew_left += 1;
            } else if why.contains("miles short") {
                tally.broke_down += 1;
            } else {
                tally.late += 1;
            }
        }
        _ => tally.stuck += 1,
    }
}

/// `runs` games of each job, leaving between `months`.
fn survey(content: &Rc<Content>, months: &[u32], runs: u64, careless: bool) -> Vec<Tally> {
    (0..content.professions.len())
        .map(|job| {
            let mut tally = Tally::default();
            for seed in 0..runs {
                let month = months[seed as usize % months.len()];
                let run = (job, month, seed * 31 + job as u64);
                play_as(content, run, careless, &mut tally);
            }
            tally
        })
        .collect()
}

fn print(content: &Content, title: &str, tallies: &[Tally]) {
    println!("\n== {title}");
    println!(
        "{:32} {:>5} {:>5} {:>6} {:>6} {:>6} {:>7} {:>6} {:>7} {:>6} {:>6}",
        "job",
        "win%",
        "late",
        "stuck",
        "crew",
        "dry",
        "E[score]",
        "days",
        "outings",
        "fined",
        "cash"
    );
    for (job, t) in content.professions.iter().zip(tallies) {
        println!(
            "{:32} {:5.0} {:5} {:6} {:6} {:6} {:7.0} {:6.2} {:7.1} {:6.2} {:6.0}",
            job.name,
            t.win_rate(),
            t.late,
            t.stuck,
            t.crew_left,
            t.broke_down,
            t.expected_score(),
            t.days_used / f64::from(t.wins.max(1)),
            f64::from(t.outings) / f64::from(t.runs),
            f64::from(t.fines) / f64::from(t.runs),
            t.money_left / f64::from(t.runs),
        );
    }
    for crew in [Crew::Wyoming, Crew::Hippie] {
        let of: Vec<&Tally> = content
            .professions
            .iter()
            .zip(tallies)
            .filter(|(j, _)| j.crew == crew)
            .map(|(_, t)| t)
            .collect();
        let n = of.len().max(1) as f64;
        println!(
            "{crew:?}: win {:.1}%  score {:.0}  servings/outing {:.1}  fined {:.2}",
            of.iter().map(|t| t.win_rate()).sum::<f64>() / n,
            of.iter().map(|t| t.expected_score()).sum::<f64>() / n,
            of.iter().map(|t| f64::from(t.servings)).sum::<f64>()
                / of.iter()
                    .map(|t| f64::from(t.outings))
                    .sum::<f64>()
                    .max(1.0),
            of.iter()
                .map(|t| f64::from(t.fines) / f64::from(t.runs))
                .sum::<f64>()
                / n,
        );
    }
}

/// Trial numbers without a rebuild: `BALANCE="job:money:days,..."` and
/// `FOOD=40` (price of a grub box).
fn trial_content() -> Content {
    let mut content = Content::load();
    for part in std::env::var("BALANCE").unwrap_or_default().split(',') {
        let n: Vec<f64> = part.split(':').filter_map(|v| v.parse().ok()).collect();
        if let [job, money, days] = n[..] {
            let job = &mut content.professions[job as usize];
            job.money = money;
            job.goal.days = days as u32;
        }
    }
    content
}

/// For each job, the day limit whose win rate comes closest to its target:
/// the bigger a job's score multiplier, the harder it is meant to be.
#[test]
#[ignore = "searches for day limits; run with --ignored --nocapture"]
fn tune_days() {
    let base = trial_content();
    let only: Option<usize> = std::env::var("JOB").ok().and_then(|v| v.parse().ok());
    for job in (0..base.professions.len()).filter(|&j| only.is_none_or(|o| o == j)) {
        let target = target_win_rate(base.professions[job].multiplier);
        let mut row = Vec::new();
        let mut best = (f64::MAX, 0);
        for days in 4..=40 {
            let mut content = base.clone();
            content.professions[job].goal.days = days;
            let content = Rc::new(content);
            let mut t = Tally::default();
            for seed in 0..500 {
                let month = [5, 6, 7, 8, 9][seed as usize % 5];
                let run = (job, month, seed * 31 + job as u64);
                play_as(&content, run, std::env::var("CARELESS").is_ok(), &mut t);
            }
            let off = (t.win_rate() - target).abs();
            if off < best.0 {
                best = (off, days);
            }
            row.push(format!("{days}:{:.0}", t.win_rate()));
            if t.win_rate() > 97.0 {
                break;
            }
        }
        println!(
            "{} (target {target}) -> {} days | {}",
            base.professions[job].name,
            best.1,
            row.join(" ")
        );
    }
}

fn target_win_rate(multiplier: u32) -> f64 {
    match multiplier {
        1 => 85.0,
        2 => 78.0,
        3 => 72.0,
        _ => 65.0,
    }
}

#[test]
#[ignore = "prints a table; run with --ignored --nocapture"]
fn balance_report() {
    let content = Rc::new(trial_content());
    let warm = [5, 6, 7, 8, 9];
    let cold = [11, 12, 1, 2, 3];
    for (title, months, careless) in [
        ("sensible, May to September", warm, false),
        ("sensible, November to March", cold, false),
        ("careless, May to September", warm, true),
    ] {
        print(&content, title, &survey(&content, &months, 600, careless));
    }
}

/// What one outing is worth, by job: the measure of poaching against
/// foraging.
#[test]
#[ignore = "prints a table; run with --ignored --nocapture"]
fn outing_report() {
    let content = Rc::new(trial_content());
    println!(
        "{:32} {:>5} {:>9} {:>7} {:>8} {:>7} {:>7}",
        "job / where", "n", "servings", "fine $", "worst $", "health", "ammo"
    );
    for (job, def) in content.professions.iter().enumerate() {
        for (place, node, month) in [
            ("country", "riverton", 6),
            ("town/park", "lander", 6),
            ("winter town", "riverton", 1),
        ] {
            let (mut food, mut fines, mut worst, mut health, mut ammo) =
                (0.0, 0.0, 0.0f64, 0.0, 0.0);
            let n = 600;
            for seed in 0..n {
                let mut g = Game::new(content.clone(), seed * 7 + job as u64);
                g.choose_profession(job).unwrap();
                g.confirm_party().unwrap();
                g.choose_month(month).unwrap();
                if g.state.screen == Screen::Outfit {
                    g.buy_truck(0).unwrap();
                }
                g.leave_store().unwrap();
                g.state.current_node = node.into();
                g.state.resources.money = 2_000.0;
                g.state.resources.food = 100;
                g.set_ammo(10);
                let mut rng = Rng::for_day(seed ^ 0xF00D, 1);
                let round = g.start_gather(LANDSCAPE).unwrap();
                let round = play_round(round, &mut rng);
                let before: f64 = g.state.party.members.iter().map(|m| m.health).sum();
                g.finish_gather(&round).unwrap();
                let eaten = 100.0 - g.state.resources.food.min(100) as f64;
                food +=
                    f64::from(g.state.resources.food) - 100.0 + g.daily_food() as f64 + eaten * 0.0;
                let paid = 2_000.0 - g.state.resources.money;
                fines += paid;
                worst = worst.max(paid);
                health += before - g.state.party.members.iter().map(|m| m.health).sum::<f64>();
                ammo += 10.0 - f64::from(g.ammo());
            }
            let n = n as f64;
            println!(
                "{:32} {:5} {:9.1} {:7.0} {:8.0} {:7.1} {:7.1}",
                format!("{} / {place}", &def.name[..def.name.len().min(18)]),
                n,
                food / n,
                fines / n,
                worst,
                health / n,
                ammo / n,
            );
        }
    }
}

/// The numbers in the data keep the jobs in step. If this fails after a
/// change, run `balance_report` to see the whole picture.
#[test]
fn jobs_stay_balanced() {
    let content = Rc::new(Content::load());
    let warm = [5, 6, 7, 8, 9];
    let sensible = survey(&content, &warm, 150, false);
    let careless = survey(&content, &warm, 150, true);
    let jobs = &content.professions;
    for (job, (good, poor)) in jobs.iter().zip(sensible.iter().zip(&careless)) {
        let name = &job.name;
        assert_eq!(good.stuck + poor.stuck, 0, "{name}");
        assert!(good.win_rate() >= 90.0, "{name}: {:.0}%", good.win_rate());
        // Mistakes cost something, but not everything.
        let rate = poor.win_rate();
        assert!((60.0..=97.0).contains(&rate), "{name}: careless {rate:.0}%");
    }
    // Neither crowd has the easier or the richer time of it.
    let crew = |tallies: &[Tally], crew: Crew, of: fn(&Tally) -> f64| {
        let these: Vec<f64> = jobs
            .iter()
            .zip(tallies)
            .filter(|(j, _)| j.crew == crew)
            .map(|(_, t)| of(t))
            .collect();
        these.iter().sum::<f64>() / these.len().max(1) as f64
    };
    let wins = |c| crew(&careless, c, Tally::win_rate);
    let gap = (wins(Crew::Wyoming) - wins(Crew::Hippie)).abs();
    assert!(gap <= 8.0, "win rates differ by {gap:.1} points");
    let score = |c| crew(&sensible, c, Tally::expected_score);
    let ratio = score(Crew::Wyoming) / score(Crew::Hippie);
    assert!((0.8..=1.25).contains(&ratio), "scores differ: {ratio:.2}");
}

/// Poaching pays in meat and costs in fines; foraging is smaller and safer.
#[test]
fn both_ways_of_living_off_the_land_are_worth_it() {
    let content = Rc::new(Content::load());
    let groceries = content.items.get("groceries").expect("sold");
    let per_serving = groceries.price / f64::from(groceries.count.unwrap_or(1));
    let ammo = content.items.get("ammo").expect("sold").price;
    let worth = |job: usize| {
        let (mut value, mut worst, n) = (0.0, 0.0_f64, 150);
        for seed in 0..n {
            let mut g = Game::new(content.clone(), seed * 13 + 5);
            g.choose_profession(job).unwrap();
            g.confirm_party().unwrap();
            g.choose_month(6).unwrap();
            g.buy_truck(0).unwrap();
            g.leave_store().unwrap();
            g.state.current_node = "riverton".into();
            g.state.resources.money = 2_000.0;
            g.state.resources.food = 100;
            let eats = g.daily_food();
            let mut rng = Rng::for_day(seed ^ 0xF00D, 1);
            let round = play_round(g.start_gather(LANDSCAPE).unwrap(), &mut rng);
            let poach = round.mode == GatherMode::Poach;
            let rounds = g.ammo();
            g.finish_gather(&round).unwrap();
            let food = f64::from(g.state.resources.food + eats) - 100.0;
            let fine = 2_000.0 - g.state.resources.money;
            let shells = f64::from(rounds - g.ammo().min(rounds)) / 10.0;
            let spent = if poach { shells * ammo } else { 0.0 };
            value += food * per_serving - fine - spent;
            worst = worst.max(fine);
        }
        (value / n as f64, worst)
    };
    // The rodeo clown poaches; the teacher forages.
    let (poaching, worst_poaching) = worth(3);
    let (foraging, worst_foraging) = worth(4);
    assert!(
        poaching > 0.0 && foraging > 0.0,
        "{poaching:.0} {foraging:.0}"
    );
    let ratio = poaching / foraging;
    assert!(
        (0.5..=1.6).contains(&ratio),
        "poaching {poaching:.0}, foraging {foraging:.0}"
    );
    assert!(worst_poaching >= 900.0, "a bad day should hurt");
    assert!(worst_foraging <= 100.0);
}

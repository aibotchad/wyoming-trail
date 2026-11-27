//! Robustness: random play, damaged saves and random gathering rounds. Set
//! `STRESS_RUNS` to play more games than the usual quick pass.

use super::*;
use crate::core::gather::{LANDSCAPE, PORTRAIT};

fn pick<T: Clone>(rng: &mut Rng, from: &[T]) -> T {
    from[((rng.unit() * from.len() as f64) as usize).min(from.len() - 1)].clone()
}

fn below(rng: &mut Rng, n: usize) -> usize {
    ((rng.unit() * n as f64) as usize).min(n.saturating_sub(1))
}

/// Try one thing a player could try, legal or not.
fn poke(g: &mut Game, rng: &mut Rng) {
    let items: Vec<String> = g.content.items.ids().map(str::to_string).collect();
    let item = pick(rng, &items);
    let gigs: Vec<String> = g.content.gigs.iter().map(|g| g.id.clone()).collect();
    let gig = pick(rng, &gigs);
    let roads: Vec<usize> = (0..g.content.map.roads.len())
        .filter(|&i| g.content.map.roads[i].from == g.state.current_node)
        .collect();
    let road = if rng.chance(0.9) && !roads.is_empty() {
        pick(rng, &roads)
    } else {
        below(rng, 200)
    };
    // Weight the moves that carry a run forward.
    let _ = match below(rng, 34) {
        0 => g.choose_profession(below(rng, 11)),
        1 => g.set_female(rng.chance(0.5)),
        2 => {
            let name = pick(
                rng,
                &["", "  ", "Dwayne", "🦌🦌🦌", "a very long name indeed"],
            );
            g.set_name(below(rng, 6), name);
            Ok(())
        }
        3 => g.confirm_party(),
        4 => g.setup_back(),
        5 => g.choose_month(below(rng, 15) as u32),
        6 => g.buy_truck(below(rng, 6)),
        7..=8 => g.buy(&item),
        9 => g.fill_tank(),
        10 => g.open_store(),
        11 => g.leave_store(),
        12 => {
            if rng.chance(0.3) {
                g.camp(below(rng, 6) as u32)
            } else {
                g.rest(rng.chance(0.5))
            }
        }
        13..=15 => g.set_off(road),
        16 => g.turn_back(),
        17..=22 => g.drive_day(),
        23 => {
            g.cycle_pace();
            g.cycle_meal();
            Ok(())
        }
        24 => g.pick_up(rng.chance(0.5)),
        25..=27 => {
            g.continue_journey();
            Ok(())
        }
        28 => g.open_trade(),
        29 => g.open_work(),
        30 => g.sell_food(below(rng, 60) as u32),
        31 => g.sell_item(&item),
        32 => g.work(&gig),
        _ => {
            let shape = if rng.chance(0.5) { LANDSCAPE } else { PORTRAIT };
            g.start_gather(shape).and_then(|mut round| {
                // Flail about for part or all of a round.
                for _ in 0..below(rng, 700) {
                    let at = (
                        rng.unit() * round.width * 1.4 - 20.0,
                        rng.unit() * round.height * 1.4 - 20.0,
                    );
                    match below(rng, 8) {
                        0 => round.fire(Some(at)),
                        1 => round.fire(None),
                        2 => round.go_to(at.0, at.1),
                        3 => round.pick(),
                        _ => {}
                    }
                    let dt = rng.unit() * 0.5 - 0.05;
                    round.update(dt, rng.unit() * 2.0 - 1.0, rng.unit() * 2.0 - 1.0);
                    assert!(round.x >= 0.0 && round.x <= round.width, "walked off");
                    assert!(round.y >= 0.0 && round.y <= round.height, "walked off");
                    assert!(round.things.len() <= 64, "things pile up");
                }
                g.finish_gather(&round)
            })
        }
    };
    g.record();
}

/// What must hold after every move.
fn check(g: &Game, step: usize, seed: u64) {
    let at = format!("seed {seed} step {step} on {:?}", g.state.screen);
    let r = &g.state.resources;
    assert!(
        r.money.is_finite() && r.money >= -1e-6,
        "{at}: money {}",
        r.money
    );
    assert!(r.wild <= r.food, "{at}: more wild food than food");
    let t = &g.state.truck;
    assert!(
        t.fuel_gallons >= -1e-6 && t.fuel_gallons <= t.tank_gallons + 1e-6,
        "{at}: fuel {}",
        t.fuel_gallons
    );
    for m in &g.state.party.members {
        for v in [m.health, m.fatigue, m.mood] {
            assert!((0.0..=100.0).contains(&v), "{at}: {} {v}", m.name);
        }
        assert!(!m.name.trim().is_empty(), "{at}: nameless");
        assert!(m.name.chars().count() <= NAME_MAX_CHARS, "{at}: {}", m.name);
    }
    if let Some(e) = &g.state.en_route {
        assert!(
            e.remaining_miles >= -1e-6 && e.remaining_miles <= e.total_miles + 1e-6,
            "{at}: {} of {}",
            e.remaining_miles,
            e.total_miles
        );
    }
    // Whatever play can reach, a save must be able to bring back.
    let text = serde_json::to_string(&g.state).expect("state serialises");
    let back: GameState = serde_json::from_str(&text).expect("state reads back");
    assert!(
        Game::from_state(g.content.clone(), back).is_some(),
        "{at}: this save would be refused: {text}"
    );
    stuck(g, &at);
}

/// Every screen but the last offers a way forward.
fn stuck(g: &Game, at: &str) {
    let try_one = |act: &dyn Fn(&mut Game) -> bool| act(&mut g.clone());
    let free = match g.state.screen {
        Screen::Profession => try_one(&|g| g.choose_profession(0).is_ok()),
        Screen::Party => try_one(&|g| g.confirm_party().is_ok()),
        Screen::Season => try_one(&|g| g.choose_month(6).is_ok()),
        Screen::Outfit => (0..g.content.vehicles.len()).any(|i| g.clone().buy_truck(i).is_ok()),
        Screen::Store | Screen::Trade | Screen::Work => try_one(&|g| g.leave_store().is_ok()),
        Screen::Town => try_one(&|g| {
            g.continue_journey();
            g.state.screen == Screen::RoadChoice
        }),
        Screen::RoadChoice => try_one(&|g| g.rest(false).is_ok()),
        Screen::Travel => try_one(&|g| g.drive_day().is_ok()),
        Screen::Event => try_one(&|g| {
            g.continue_journey();
            g.state.screen != Screen::Event
        }),
        Screen::Hitchhiker => try_one(&|g| g.pick_up(false).is_ok()),
        Screen::Win | Screen::GameOver => true,
    };
    assert!(free, "{at}: nowhere to go: {:?}", g.state);
}

fn runs(default: u64) -> u64 {
    std::env::var("STRESS_RUNS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[test]
fn random_play_never_breaks_or_traps_the_player() {
    let content = Rc::new(Content::load());
    let mut ended = [0u32; 2];
    for seed in 0..runs(150) {
        let mut g = Game::new(content.clone(), seed);
        let mut rng = Rng::for_day(seed ^ 0x57E5, 1);
        for step in 0..600 {
            poke(&mut g, &mut rng);
            check(&g, step, seed);
            match g.state.screen {
                Screen::Win => ended[0] += 1,
                Screen::GameOver => ended[1] += 1,
                _ => continue,
            }
            break;
        }
    }
    // The random player reaches both endings, so both are exercised.
    assert!(ended[1] > 0, "no run ever failed: {ended:?}");
}

#[test]
fn damaged_saves_are_refused_or_playable() {
    use serde_json::{json, Value};
    let content = Rc::new(Content::load());
    // A mid-trip save to damage.
    let mut g = Game::new(content.clone(), 7);
    g.choose_profession(0).unwrap();
    g.confirm_party().unwrap();
    g.choose_month(6).unwrap();
    g.buy_truck(0).unwrap();
    g.leave_store().unwrap();
    g.set_off(
        g.content
            .map
            .roads
            .iter()
            .position(|r| r.from == "lander")
            .unwrap(),
    )
    .unwrap();
    g.record();
    let good = serde_json::to_value(&g.state).unwrap();

    /// Every path to a value inside `v`.
    fn paths(v: &Value, here: String, out: &mut Vec<String>) {
        let children: Vec<(String, &Value)> = match v {
            Value::Object(m) => m.iter().map(|(k, v)| (k.clone(), v)).collect(),
            Value::Array(a) => a
                .iter()
                .enumerate()
                .map(|(i, v)| (i.to_string(), v))
                .collect(),
            _ => Vec::new(),
        };
        for (key, child) in children {
            let path = format!("{here}/{key}");
            paths(child, path.clone(), out);
            out.push(path);
        }
    }
    let mut all = Vec::new();
    paths(&good, String::new(), &mut all);
    let junk = [
        json!(null),
        json!(0),
        json!(-1),
        json!(1),
        json!(13),
        json!(999),
        json!(4294967295u32),
        json!(1e300),
        json!(-1e300),
        json!(""),
        json!("atlantis"),
        json!("Travel"),
        json!("Event"),
        json!("Win"),
        json!("Store"),
        json!([]),
        json!({}),
        json!(true),
    ];
    let mut rng = Rng::for_day(99, 1);
    let mut loaded = 0;
    for path in &all {
        for bad in &junk {
            let mut save = good.clone();
            *save.pointer_mut(path).expect("path exists") = bad.clone();
            let Ok(state) = serde_json::from_value::<GameState>(save) else {
                continue;
            };
            let Some(mut g) = Game::from_state(content.clone(), state) else {
                continue;
            };
            loaded += 1;
            stuck(&g, &format!("save with {path} = {bad}"));
            for _ in 0..80 {
                poke(&mut g, &mut rng);
            }
        }
    }
    assert!(loaded > 0, "the damage never got past the loader");
}

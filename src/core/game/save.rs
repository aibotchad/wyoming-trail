//! Is a saved game one that play could have produced? A save lives in the
//! browser, where it can be half-written, out of date or edited, and a bad one
//! must be refused, not crash the game or strand the player.

use super::*;

/// `0.0..=max`, and a real number.
fn within(value: f64, max: f64) -> bool {
    value.is_finite() && (0.0..=max).contains(&value)
}

/// Larger than any purse or tank play could fill, small enough to add safely.
const HUGE: f64 = 1e9;

pub(super) fn is_sound(content: &Content, s: &GameState) -> bool {
    run(content, s) && party(s) && supplies(s) && screens(s)
}

fn run(content: &Content, s: &GameState) -> bool {
    let Some(job) = content.professions.get(s.profession) else {
        return false;
    };
    content.map.node(&s.current_node).is_some()
        && s.goal_step <= job.goal.steps.len()
        && (1..=MAX_DAY).contains(&s.day)
        && (1..=12).contains(&s.start_month)
        && s.pace_index < PACES.len()
        && s.meal_index < MEALS.len()
        && s.history.len() <= MAX_DAY as usize
        && s.history
            .iter()
            .all(|d| (1..=s.day).contains(&d.day) && within(d.miles, HUGE) && within(d.money, HUGE))
        && s.history.windows(2).all(|w| w[0].day < w[1].day)
}

fn party(s: &GameState) -> bool {
    let members = &s.party.members;
    (1..=MAX_PARTY).contains(&members.len())
        && members.iter().all(|m| {
            !m.name.trim().is_empty()
                && m.name.chars().count() <= NAME_MAX_CHARS
                && [m.health, m.fatigue, m.mood]
                    .iter()
                    .all(|&v| within(v, 100.0))
        })
}

fn supplies(s: &GameState) -> bool {
    let (r, t) = (&s.resources, &s.truck);
    within(r.money, HUGE)
        && r.wild <= r.food
        && r.food <= HUGE as u32
        && r.inventory.values().all(|&n| n <= HUGE as u32)
        && within(t.tank_gallons, HUGE)
        && t.tank_gallons > 0.0
        && within(t.mpg, HUGE)
        && t.mpg > 0.0
        && within(t.fuel_gallons, t.tank_gallons + 1e-6)
}

/// The screen, and the one it returns to, must have what they need to be left.
fn screens(s: &GameState) -> bool {
    let driving = s.en_route.as_ref();
    let road_ok = driving.is_none_or(|e| {
        within(e.total_miles, HUGE) && within(e.remaining_miles, e.total_miles + 1e-6)
    });
    let fits = |screen: Screen| match screen {
        Screen::Travel => driving.is_some(),
        Screen::Town | Screen::RoadChoice => driving.is_none(),
        _ => true,
    };
    // Where a shop, an event or a hitchhiker leads on to.
    let onward = matches!(
        s.next_screen,
        Screen::Town | Screen::RoadChoice | Screen::Travel | Screen::Win | Screen::GameOver
    );
    let returns = match s.screen {
        Screen::Event | Screen::Hitchhiker => onward && fits(s.next_screen),
        // Counters open from a stop and close back onto it.
        Screen::Store | Screen::Trade | Screen::Work => {
            matches!(s.next_screen, Screen::Town | Screen::RoadChoice) && driving.is_none()
        }
        _ => true,
    };
    road_ok && fits(s.screen) && returns
}

//! Rule tests.

use super::*;
use crate::core::data::load_vehicles;
use crate::core::defs::Ride;
use crate::core::gather::LANDSCAPE;

// Places in `professions.json`; `the_named_jobs_are_where_the_tests_think`
// keeps them honest.
const SKI_BUM: usize = 1;
const TEACHER: usize = 4;
const DROPOUT: usize = 5;
const RANCH_HAND: usize = 6;

#[test]
fn the_named_jobs_are_where_the_tests_think() {
    let jobs = Game::new_run(1).content.professions.clone();
    assert!(jobs[SKI_BUM].name.contains("Ski Bum"));
    assert!(jobs[TEACHER].name.contains("Teacher"));
    assert!(jobs[DROPOUT].name.contains("NOLS"));
    assert!(jobs[RANCH_HAND].name.contains("Ranch Hand"));
}

/// A run that has finished setup and sits at the first road choice.
fn on_the_road(seed: u64, profession: usize, truck: usize) -> Game {
    let mut g = Game::new_run(seed);
    g.choose_profession(profession).unwrap();
    // Enough that the job's own purse never gets in a rule's way.
    g.state.resources.money = g.state.resources.money.max(3_600.0);
    g.confirm_party().unwrap();
    g.choose_month(6).unwrap();
    if g.state.screen == Screen::Outfit {
        g.buy_truck(truck).unwrap();
    }
    g.leave_store().unwrap();
    g
}

fn road_index(g: &Game, to: &str) -> usize {
    g.content
        .map
        .roads
        .iter()
        .position(|r| r.from == g.state.current_node && r.to == to)
        .unwrap_or_else(|| panic!("no road to {to}"))
}

#[test]
fn setup_spends_money_on_a_truck() {
    let mut g = Game::new_run(1);
    assert!(g.buy_truck(0).is_err(), "must pick a profession first");
    g.choose_profession(0).unwrap();
    g.set_name(0, "  Earl  ");
    let second = g.state.party.members[1].name.clone();
    g.set_name(1, "   ");
    assert_eq!(g.state.party.members[0].name, "Earl");
    assert_eq!(g.state.party.members[1].name, second);
    g.confirm_party().unwrap();
    g.choose_month(6).unwrap();
    g.buy_truck(1).unwrap();
    assert_eq!(
        g.state.resources.money,
        g.content.professions[0].money - g.content.vehicles[1].price
    );
    assert_eq!(g.state.screen, Screen::Store);
}

#[test]
fn every_profession_can_afford_the_cheapest_truck() {
    for p in &Game::new_run(1).content.professions {
        // Whoever is issued a mount buys nothing.
        assert!(
            p.mount.is_some() || p.money > load_vehicles()[0].price,
            "{}",
            p.name
        );
    }
}

#[test]
fn every_profession_can_buy_the_prius() {
    let prius = load_vehicles()
        .iter()
        .position(|t| t.id == "prius")
        .expect("the Prius is for sale");
    let jobs = Game::new_run(1).content.professions.clone();
    for (job, def) in jobs.iter().enumerate() {
        if def.mount.is_some() {
            continue;
        }
        let g = on_the_road(1, job, prius);
        assert!(g.state.truck.name.contains("Hillary sticker"));
    }
}

#[test]
fn store_sells_food_and_refuses_the_broke() {
    let mut g = on_the_road(1, 0, 0);
    g.open_store().unwrap();
    g.buy("groceries").unwrap();
    assert_eq!(g.state.resources.food, 10);
    g.buy("banjo").unwrap();
    assert!(g.buy("banjo").is_err(), "reusable items are bought once");
    g.state.resources.money = 1.0;
    assert!(g.buy("groceries").is_err());
    g.leave_store().unwrap();
    assert_eq!(g.state.screen, Screen::RoadChoice);
}

#[test]
fn filling_the_tank_costs_money_and_clerks_pay_less() {
    let mut full = on_the_road(1, 0, 1);
    let mut clerk = on_the_road(1, 2, 1);
    let before = full.state.resources.money;
    full.fill_tank().unwrap();
    assert_eq!(full.state.truck.fuel_gallons, full.state.truck.tank_gallons);
    assert!(full.state.resources.money < before);
    assert!(full.fill_tank().is_err(), "already full");
    assert!(clerk.fuel_price().unwrap() < full.fuel_price().unwrap());
    clerk.state.resources.money = 3.0;
    clerk.fill_tank().unwrap();
    assert!(clerk.state.resources.money < 0.01);
}

#[test]
fn driving_uses_fuel_and_food() {
    let mut g = on_the_road(7, 0, 1);
    g.state.resources.food = 100;
    g.fill_tank().unwrap();
    g.set_off(road_index(&g, "riverton")).unwrap();
    let fuel = g.state.truck.fuel_gallons;
    g.drive_day().unwrap();
    assert!(g.state.truck.fuel_gallons < fuel);
    assert!(g.state.resources.food < 100);
    assert!(g.state.day >= 2);
}

#[test]
fn a_gas_can_prevents_running_dry() {
    let mut g = on_the_road(7, 0, 1);
    g.state.resources.add_item("gas_can", 2);
    g.state.truck.fuel_gallons = 0.0;
    g.state.resources.money = 0.0;
    g.set_off(road_index(&g, "riverton")).unwrap();
    g.drive_day().unwrap();
    assert_ne!(g.state.screen, Screen::GameOver);
}

#[test]
fn running_dry_costs_a_tow_or_ends_the_trip() {
    let mut g = on_the_road(7, 0, 1);
    g.state.truck.fuel_gallons = 0.0;
    g.set_off(road_index(&g, "riverton")).unwrap();
    let mut broke = g.clone();
    let money = g.state.resources.money;
    g.drive_day().unwrap();
    assert_ne!(g.state.screen, Screen::GameOver);
    assert!(g.state.resources.money <= money - g.ride_def().rescue_cost);

    broke.state.resources.money = 0.0;
    broke.drive_day().unwrap();
    assert_eq!(broke.state.screen, Screen::GameOver);
    assert_eq!(broke.state.score.total, 0);
}

fn event_named(g: &Game, name: &str) -> EventDef {
    let any = g.content.events.events("any").unwrap();
    any.iter().find(|e| e.name == name).unwrap().clone()
}

#[test]
fn needed_items_soften_events() {
    let mut g = on_the_road(3, 0, 1);
    let flat = event_named(&g, "Flat tire");
    assert_eq!(flat.needs.as_deref(), Some("spare_tire"));
    g.state.resources.add_item("spare_tire", 1);
    let day = g.state.day;
    let money = g.state.resources.money;
    g.apply_event(&flat, &mut Rng::for_day(1, 1));
    assert_eq!((g.state.day, g.state.resources.money), (day, money));
    assert_eq!(g.state.resources.count("spare_tire"), 0);
    g.apply_event(&flat, &mut Rng::for_day(1, 1));
    assert_eq!(g.state.day, day + 1);
}

#[test]
fn handy_players_fix_overheating_for_free() {
    let mut g = on_the_road(3, 0, 0);
    let hot = event_named(&g, "Overheating");
    assert_eq!(hot.needs.as_deref(), Some("duct_tape"));
    let day = g.state.day;
    g.apply_event(&hot, &mut Rng::for_day(1, 1));
    assert_eq!(g.state.day, day);
}

/// Take the next road toward the mission and drive it to its end.
fn drive_toward_goal(g: &mut Game) {
    let path = g
        .content
        .map
        .route(&g.state.current_node, g.targets())
        .unwrap();
    g.set_off(path[0]).unwrap();
    for _ in 0..20 {
        match g.state.screen {
            Screen::Travel => g.drive_day().unwrap(),
            Screen::Event => g.continue_journey(),
            Screen::Hitchhiker => g.pick_up(false).unwrap(),
            _ => return,
        }
    }
}

#[test]
fn every_mission_can_be_completed() {
    for job in 0..Game::new_run(1).content.professions.len() {
        let mut g = on_the_road(job as u64, job, 0);
        // This checks the roads, so the clock is no object.
        Rc::make_mut(&mut g.content).professions[job].goal.days = 999;
        for _ in 0..40 {
            if !g.stopped() {
                break;
            }
            // Keep supplies topped up: this tests the map, not the economy.
            g.state.resources.money = 5_000.0;
            g.state.resources.food = 200;
            let _ = g.fill_tank();
            g.rest(false).unwrap();
            drive_toward_goal(&mut g);
        }
        let name = &g.content.professions[job].name;
        assert_eq!(
            g.state.screen,
            Screen::Win,
            "{name}: {}",
            g.state.ending_reason
        );
        assert!(g.state.score.total > 0);
    }
}

#[test]
fn a_two_step_mission_goes_there_and_back() {
    let mut g = on_the_road(5, SKI_BUM, 0);
    assert_eq!(g.targets(), ["dubois"]);
    g.arrive("dubois");
    assert_eq!(g.state.screen, Screen::Town);
    assert!(g.state.log.iter().any(|l| l.contains("skis")));
    assert_eq!(g.targets(), ["jackson"]);
    g.arrive("jackson");
    assert_eq!(g.state.screen, Screen::Win);
}

#[test]
fn only_your_own_destination_wins() {
    let mut g = on_the_road(5, TEACHER, 0);
    g.arrive("yellowstone_south");
    assert_eq!(g.state.screen, Screen::Town);
    g.arrive("casper");
    assert_eq!(g.state.screen, Screen::Win);
}

#[test]
fn poachers_start_armed_and_foragers_need_no_ammo() {
    let poacher = on_the_road(1, 0, 0);
    assert_eq!(poacher.state.resources.count("ammo"), STARTING_AMMO);
    assert!(poacher.start_gather(LANDSCAPE).is_ok());
    assert!(poacher.stock().iter().any(|(id, _)| *id == "ammo"));

    let mut teacher = on_the_road(1, TEACHER, 0);
    assert_eq!(
        teacher.start_gather(LANDSCAPE).unwrap().mode,
        GatherMode::Forage
    );
    assert!(!teacher.stock().iter().any(|(id, _)| *id == "ammo"));
    teacher.state.resources.add_item("ammo", 0);

    let mut dry = on_the_road(1, 0, 0);
    dry.state.resources.inventory.clear();
    assert!(dry.start_gather(LANDSCAPE).is_err());
}

#[test]
fn a_round_costs_a_day_and_feeds_the_party() {
    let mut g = on_the_road(1, TEACHER, 0);
    g.state.resources.food = 0;
    let mut round = g.start_gather(LANDSCAPE).unwrap();
    round.food = 30;
    let day = g.state.day;
    g.finish_gather(&round).unwrap();
    assert_eq!(g.state.day, day + 1);
    assert_eq!(g.state.resources.food, 30 - 8, "four eat two each");
}

#[test]
fn poison_hurts_someone() {
    let mut g = on_the_road(1, TEACHER, 0);
    g.state.resources.food = 50;
    let mut round = g.start_gather(LANDSCAPE).unwrap();
    round.poison = 1;
    g.finish_gather(&round).unwrap();
    assert!(g.state.party.members.iter().any(|m| m.health < 100.0));
}

#[test]
fn the_warden_sometimes_takes_the_meat() {
    let outcomes: Vec<(u32, f64)> = (0..200)
        .map(|seed| {
            let mut g = on_the_road(seed, 0, 0);
            g.state.resources.food = 100;
            let money = g.state.resources.money;
            let mut round = g.start_gather(LANDSCAPE).unwrap();
            (round.food, round.fired, round.ammo) = (28, 1, 9);
            g.finish_gather(&round).unwrap();
            assert!(matches!(g.state.resources.count("ammo"), 0 | 9));
            (g.state.resources.food, money - g.state.resources.money)
        })
        .collect();
    assert!(outcomes.contains(&(92 + 28, 0.0)), "got away with it");
    // Each tier of fine turns up, always costing the meat.
    for w in &crate::core::data::load_wardens() {
        let food = 92 - 8 * w.days;
        assert!(
            outcomes.contains(&(food, w.fine)),
            "{}: {outcomes:?}",
            w.name
        );
    }
}

#[test]
fn default_names_are_unique_and_match_the_driver() {
    for seed in 0..50 {
        let mut g = Game::new_run(seed);
        g.choose_profession(0).unwrap();
        let names = |g: &Game| -> Vec<String> {
            g.state
                .party
                .members
                .iter()
                .map(|m| m.name.clone())
                .collect()
        };
        let first = names(&g);
        let unique: std::collections::HashSet<_> = first.iter().collect();
        assert_eq!(unique.len(), 4, "{first:?}");
        assert!(g.content.names.wyoming.men.contains(&first[0]));
        g.set_female(true).unwrap();
        assert!(
            g.content.names.wyoming.women.contains(&names(&g)[0]),
            "{:?}",
            names(&g)
        );
        let unique: std::collections::HashSet<_> = names(&g).into_iter().collect();
        assert_eq!(unique.len(), 4);
    }
}

#[test]
fn a_chosen_name_survives_switching_sex() {
    let mut g = Game::new_run(1);
    g.choose_profession(0).unwrap();
    g.set_name(0, "Pat");
    g.set_female(true).unwrap();
    assert_eq!(g.state.party.members[0].name, "Pat");
    assert!(g.state.female);
}

#[test]
fn a_quote_is_exactly_what_gets_charged() {
    let mut g = Game::new_run(1);
    g.choose_profession(0).unwrap();
    // Enough to buy one of everything quoted.
    g.state.resources.money = 5_000.0;
    g.confirm_party().unwrap();
    g.choose_month(6).unwrap();
    let charged = |g: &mut Game, p: Purchase, buy: &dyn Fn(&mut Game)| {
        let before = g.state.resources.money;
        let quote = g.quote(&p).unwrap();
        assert_eq!(g.state.resources.money, before, "quoting is free");
        buy(g);
        assert!((before - g.state.resources.money - quote.cost).abs() < 1e-9);
    };
    charged(&mut g, Purchase::Truck(1), &|g| g.buy_truck(1).unwrap());
    charged(&mut g, Purchase::Item("groceries".into()), &|g| {
        g.buy("groceries").unwrap()
    });
    charged(&mut g, Purchase::Fuel, &|g| g.fill_tank().unwrap());
    assert!(g.quote(&Purchase::Fuel).is_err(), "already full");
    g.leave_store().unwrap();
    charged(&mut g, Purchase::Motel, &|g| g.rest(true).unwrap());
    g.state.resources.money = 1.0;
    assert!(g.quote(&Purchase::Motel).is_err());
}

#[test]
fn the_calendar_rolls_over_the_new_year() {
    let mut g = on_the_road(1, 0, 0);
    g.state.start_month = 12;
    assert_eq!(g.date(1), (12, 1));
    assert_eq!(g.date(30), (12, 30));
    assert_eq!(g.date(31), (1, 1));
}

#[test]
fn winter_brings_snow_and_summer_does_not() {
    let mut g = on_the_road(1, 0, 0);
    let snowy = |g: &Game| {
        (1..=30)
            .filter(|&d| matches!(g.weather_on(d).unwrap().id.as_str(), "snow" | "blizzard"))
            .count()
    };
    g.state.start_month = 1;
    assert!(snowy(&g) > 5);
    g.state.start_month = 7;
    assert_eq!(snowy(&g), 0);
    assert_eq!(g.weather_on(3), g.weather_on(3));
}

#[test]
fn bad_weather_slows_the_drive() {
    let miles = |weather: &str| {
        let mut g = on_the_road(1, 0, 1);
        Rc::make_mut(&mut g.content)
            .weather
            .retain(|w| w.id == weather);
        for w in &mut Rc::make_mut(&mut g.content).weather {
            w.months = [1; 12];
        }
        g.fill_tank().unwrap();
        g.set_off(road_index(&g, "dubois")).unwrap();
        Rc::make_mut(&mut g.content).events = Default::default();
        g.state.pace_index = 0;
        g.drive_day().unwrap();
        g.state.en_route.map_or(75.0, |e| 75.0 - e.remaining_miles)
    };
    assert!(miles("blizzard") < miles("clear"));
}

#[test]
fn a_wrong_turn_adds_miles_unless_you_carry_an_atlas() {
    let mut g = on_the_road(3, 0, 1);
    let any = g.content.events.events("any").unwrap();
    let gps = any
        .iter()
        .find(|e| e.name == "GPS shortcut")
        .unwrap()
        .clone();
    assert_eq!(gps.needs.as_deref(), Some("road_atlas"));
    g.set_off(road_index(&g, "dubois")).unwrap();
    g.apply_event(&gps, &mut Rng::for_day(1, 1));
    assert_eq!(g.state.en_route.as_ref().unwrap().remaining_miles, 93.0);
    g.state.resources.add_item("road_atlas", 1);
    g.apply_event(&gps, &mut Rng::for_day(1, 1));
    assert_eq!(g.state.en_route.as_ref().unwrap().remaining_miles, 93.0);
}

#[test]
fn picking_up_a_hitchhiker_is_your_choice() {
    let mut g = on_the_road(3, 0, 1);
    g.state.screen = Screen::Hitchhiker;
    g.state.next_screen = Screen::Town;
    let mut declined = g.clone();
    declined.pick_up(false).unwrap();
    assert_eq!(declined.state.screen, Screen::Town);
    assert_eq!(declined.state.resources, g.state.resources);

    g.pick_up(true).unwrap();
    assert_eq!(g.state.screen, Screen::Event);
    assert!(g.state.event.is_some());
    g.continue_journey();
    assert_eq!(g.state.screen, Screen::Town);
}

#[test]
fn hitchhikers_turn_up_on_the_road_with_good_and_bad_results() {
    let met = (0..300)
        .filter(|&seed| {
            let mut g = on_the_road(seed, 0, 1);
            g.set_off(road_index(&g, "dubois")).unwrap();
            g.drive_day().unwrap();
            g.state.screen == Screen::Hitchhiker
        })
        .count();
    assert!(met > 0);
    let g = on_the_road(1, 0, 0);
    assert!(g.content.hitchhikers.iter().any(|h| h.effect.mood > 0.0));
    assert!(g
        .content
        .hitchhikers
        .iter()
        .any(|h| h.text.contains("pukes")));
}

#[test]
fn running_out_of_days_fails_the_mission() {
    let mut g = on_the_road(1, 0, 0);
    g.state.resources.food = 500;
    let days = g.job().unwrap().goal.days;
    assert_eq!(g.days_left(), days);
    for _ in 0..days - 1 {
        g.rest(false).unwrap();
    }
    assert_eq!(g.days_left(), 1);
    assert_ne!(g.state.screen, Screen::GameOver);
    g.rest(false).unwrap();
    assert_eq!(g.state.screen, Screen::GameOver);
    assert_eq!(g.state.ending_reason, g.job().unwrap().goal.late);
}

#[test]
fn arriving_on_the_last_day_still_wins() {
    let mut g = on_the_road(1, TEACHER, 1);
    g.fill_tank().unwrap();
    g.state.resources.food = 100;
    Rc::make_mut(&mut g.content).events = Default::default();
    g.state.current_node = "shoshoni".into();
    g.state.day = g.job().unwrap().goal.days;
    let road = road_index(&g, "casper");
    g.set_off(road).unwrap();
    g.state.en_route.as_mut().unwrap().remaining_miles = 5.0;
    g.drive_day().unwrap();
    assert_eq!(g.state.screen, Screen::Win, "{}", g.state.ending_reason);
}

#[test]
fn a_job_pays_costs_days_and_is_only_done_once_per_town() {
    let mut g = on_the_road(1, 0, 0);
    g.state.resources.food = 200;
    let (money, day) = (g.state.resources.money, g.state.day);
    g.open_work().unwrap();
    let quote = g.quote(&Purchase::Work("roofer".into())).unwrap();
    assert_eq!((quote.cost, quote.days), (-420.0, 3));
    g.work("roofer").unwrap();
    assert_eq!(g.state.resources.money, money + 420.0);
    assert_eq!(g.state.day, day + 3);
    assert!(g.state.log.iter().any(|l| l.contains("vanish")));
    assert!(g.offers().iter().all(|j| j.id != "roofer"));
    assert!(g.offers().iter().any(|j| j.id == "flagger"));
}

#[test]
fn wild_food_and_gear_can_be_sold_but_groceries_cannot() {
    let mut g = on_the_road(1, TEACHER, 0);
    g.state.resources.food = 30;
    g.state.resources.wild = 10;
    g.state.resources.add_item("spare_tire", 1);
    g.open_trade().unwrap();
    assert!(g.sell_food(11).is_err());
    let money = g.state.resources.money;
    g.sell_food(10).unwrap();
    assert_eq!(g.state.resources.money, money + 10.0 * WILD_FOOD_PRICE);
    assert_eq!((g.state.resources.food, g.state.resources.wild), (20, 0));
    assert!(g.sell_item("groceries").is_err());
    g.sell_item("spare_tire").unwrap();
    assert_eq!(g.state.resources.count("spare_tire"), 0);
    assert!(g.sell_item("spare_tire").is_err());
    g.leave_store().unwrap();
    assert_eq!(g.state.screen, Screen::RoadChoice);
}

#[test]
fn selling_never_pays_more_than_the_shop_charges() {
    let g = on_the_road(1, 0, 0);
    for id in g.content.items.ids() {
        if let Some((_, _, cash)) = g.pawn_offer(id) {
            assert!(
                cash < g.item_price(g.content.items.get(id).unwrap()),
                "{id}"
            );
        }
    }
}

#[test]
fn stolen_food_cannot_then_be_sold() {
    let mut g = on_the_road(1, 0, 0);
    (g.state.resources.food, g.state.resources.wild) = (16, 16);
    let heist = EventDef {
        effect: Effect {
            food: -10,
            ..Effect::default()
        },
        ..g.content.hitchhikers[0].clone()
    };
    g.apply_event(&heist, &mut Rng::for_day(1, 1));
    assert_eq!((g.state.resources.food, g.state.resources.wild), (6, 6));
}

#[test]
fn a_gift_of_gas_reaches_the_tank() {
    let mut g = on_the_road(1, 0, 1);
    let before = g.state.truck.fuel_gallons;
    let wizard = g
        .content
        .hitchhikers
        .iter()
        .find(|h| h.effect.fuel > 0.0)
        .unwrap()
        .clone();
    g.apply_event(&wizard, &mut Rng::for_day(1, 1));
    assert_eq!(g.state.truck.fuel_gallons, before + wizard.effect.fuel);
}

#[test]
fn a_wrong_turn_costs_gas_only_when_driven_and_keeps_progress() {
    let mut g = on_the_road(3, 0, 1);
    let gps = event_named(&g, "Recalculating");
    g.set_off(road_index(&g, "dubois")).unwrap();
    g.state.en_route.as_mut().unwrap().remaining_miles = 60.0;
    let fuel = g.state.truck.fuel_gallons;
    g.apply_event(&gps, &mut Rng::for_day(1, 1));
    let e = g.state.en_route.as_ref().unwrap();
    assert_eq!(g.state.truck.fuel_gallons, fuel);
    assert_eq!(e.total_miles - e.remaining_miles, 15.0);
}

#[test]
fn a_lost_day_on_the_last_leg_misses_the_deadline() {
    let lost = (0..50)
        .map(|seed| {
            let mut g = on_the_road(seed, TEACHER, 1);
            g.fill_tank().unwrap();
            g.state.resources.food = 100;
            let stall = EventDef {
                effect: Effect {
                    days: 1,
                    ..Effect::default()
                },
                needs: None,
                weight: 1,
                ..g.content.hitchhikers[0].clone()
            };
            Rc::make_mut(&mut g.content).events =
                serde_json::from_value(serde_json::json!({ "any": [stall] })).unwrap();
            g.state.current_node = "shoshoni".into();
            g.state.day = g.job().unwrap().goal.days;
            let road = road_index(&g, "casper");
            g.set_off(road).unwrap();
            g.state.en_route.as_mut().unwrap().remaining_miles = 5.0;
            g.drive_day().unwrap();
            (g.state.event.is_some(), g.state.screen)
        })
        .filter(|(stalled, _)| *stalled)
        .map(|(_, screen)| screen)
        .collect::<Vec<_>>();
    assert!(!lost.is_empty());
    assert!(lost.iter().all(|&s| s == Screen::GameOver), "{lost:?}");
}

#[test]
fn teachers_get_a_discount() {
    let teacher = on_the_road(1, TEACHER, 0);
    let other = on_the_road(1, 0, 0);
    let tire = teacher.content.items.get("spare_tire").unwrap();
    assert!(teacher.item_price(tire) < other.item_price(tire));
}

#[test]
fn an_event_that_ends_the_trip_is_still_shown() {
    // Find a seed whose first drive day has an event.
    let mut g = (0..200)
        .find_map(|seed| {
            let mut g = on_the_road(seed, 0, 1);
            g.set_off(road_index(&g, "dubois")).unwrap();
            let mut probe = g.clone();
            probe.drive_day().unwrap();
            probe.state.event.is_some().then_some(g)
        })
        .expect("some seed rolls an event");
    // Everyone is on their last legs and there is nothing to eat.
    for m in &mut g.state.party.members {
        m.health = 1.0;
    }
    g.drive_day().unwrap();
    assert_eq!(g.state.screen, Screen::GameOver);
    assert!(g.state.event.is_some());
}

#[test]
fn saved_state_round_trips() {
    let g = on_the_road(9, RANCH_HAND, 0);
    let json = serde_json::to_string(&g.state).unwrap();
    let back = Game::from_state(g.content.clone(), serde_json::from_str(&json).unwrap()).unwrap();
    assert_eq!(back.perk(), g.perk());
    assert!(back.perk().fatigue < 1.0);
    let mut stale = g.state.clone();
    stale.current_node = "atlantis".into();
    assert!(Game::from_state(g.content.clone(), stale).is_none());
}

#[test]
fn backing_up_in_setup_can_change_jobs() {
    let mut g = Game::new_run(1);
    assert!(g.setup_back().is_err(), "nothing before the first question");
    g.choose_profession(0).unwrap();
    g.confirm_party().unwrap();
    g.choose_month(6).unwrap();
    for expected in [Screen::Season, Screen::Party, Screen::Profession] {
        g.setup_back().unwrap();
        assert_eq!(g.state.screen, expected);
    }
    assert!(g.ammo() > 0);
    // The teacher forages: no leftover ammunition, their own money.
    g.choose_profession(TEACHER).unwrap();
    assert_eq!(g.ammo(), 0);
    assert_eq!(
        g.state.resources.money,
        g.content.professions[TEACHER].money
    );
}

#[test]
fn a_road_can_be_unchosen_until_miles_are_driven() {
    let mut g = Game::new_run(2);
    g.choose_profession(0).unwrap();
    g.confirm_party().unwrap();
    g.choose_month(6).unwrap();
    g.buy_truck(0).unwrap();
    g.leave_store().unwrap();
    let road = g
        .content
        .map
        .roads
        .iter()
        .position(|r| r.from == g.state.current_node);
    let road = road.unwrap();
    g.set_off(road).unwrap();
    assert!(g.can_turn_back());
    g.turn_back().unwrap();
    assert_eq!(g.state.screen, Screen::RoadChoice);
    assert!(g.state.en_route.is_none());
    assert_eq!(g.state.day, 1);
    g.set_off(road).unwrap();
    g.state.en_route.as_mut().unwrap().remaining_miles -= 1.0;
    assert!(g.turn_back().is_err());
}

#[test]
fn sleeping_in_the_truck_is_quoted_as_a_free_day() {
    let mut g = Game::new_run(3);
    assert!(g.quote(&Purchase::Camp(1)).is_err(), "not on the road yet");
    g.choose_profession(0).unwrap();
    g.confirm_party().unwrap();
    g.choose_month(6).unwrap();
    g.buy_truck(0).unwrap();
    g.leave_store().unwrap();
    let quote = g.quote(&Purchase::Camp(1)).unwrap();
    assert_eq!((quote.cost, quote.days), (0.0, 1));
    g.rest(false).unwrap();
    assert_eq!(g.state.day, 2);
}

#[test]
fn the_dropout_gathers_friends_then_holds_the_circle_at_sinks_canyon() {
    let mut g = on_the_road(4, DROPOUT, 0);
    let stops = [
        "dubois",
        "jackson",
        "pinedale",
        "south_pass_city",
        "sinks_canyon",
    ];
    for stop in stops {
        assert_eq!(g.targets().first().map(String::as_str), Some(stop));
        assert_ne!(g.state.screen, Screen::Win);
        g.arrive(stop);
    }
    assert_eq!(g.state.screen, Screen::Win);
}

#[test]
fn a_save_survives_roads_being_renumbered() {
    let mut g = on_the_road(5, 0, 0);
    let road = g.content.map.roads.len() - 1;
    let from = g.content.map.roads[road].from.clone();
    g.state.current_node = from;
    g.set_off(road).unwrap();
    let mut saved = g.state.clone();
    saved.en_route.as_mut().unwrap().road = road - 1;
    let loaded = Game::from_state(g.content.clone(), saved.clone()).unwrap();
    assert_eq!(loaded.state.en_route.unwrap().road, road);
    saved.en_route.as_mut().unwrap().destination = "atlantis".into();
    assert!(Game::from_state(g.content, saved).is_none());
}

#[test]
fn foraging_jobs_travel_with_hippie_names() {
    let mut g = Game::new_run(11);
    let redneck = |g: &Game, n: &str| {
        let pool = &g.content.names.wyoming;
        pool.men.iter().chain(&pool.women).any(|x| x == n)
    };
    let names = |g: &Game| -> Vec<String> {
        g.state
            .party
            .members
            .iter()
            .map(|m| m.name.clone())
            .collect()
    };
    // The dropout's crew.
    g.choose_profession(DROPOUT).unwrap();
    let hippie = &g.content.names.hippie;
    assert!(names(&g)
        .iter()
        .all(|n| hippie.men.contains(n) || hippie.women.contains(n)));
    g.set_female(true).unwrap();
    assert!(g.content.names.hippie.women.contains(&names(&g)[0]));
    // A typed name survives a change of job; the rest are swapped.
    g.set_name(1, "Dwayne");
    g.setup_back().unwrap();
    g.choose_profession(0).unwrap();
    assert_eq!(names(&g)[1], "Dwayne");
    assert!(redneck(&g, &names(&g)[0]) && redneck(&g, &names(&g)[2]));
}

#[test]
fn the_goal_list_ticks_off_each_step() {
    use GoalState::{Current, Done, Later};
    let mut g = on_the_road(12, DROPOUT, 0);
    let states = |g: &Game| -> Vec<GoalState> { g.goals().iter().map(|g| g.1).collect() };
    assert_eq!(states(&g), [Current, Later, Later, Later, Later]);
    g.arrive("dubois");
    assert_eq!(states(&g), [Done, Current, Later, Later, Later]);
    assert!(g.state.log.iter().any(|l| l.contains("GOAL MET")));
    for stop in ["jackson", "pinedale", "south_pass_city", "sinks_canyon"] {
        g.arrive(stop);
    }
    assert_eq!(states(&g), [Done; 5]);
}

#[test]
fn foragers_dive_dumpsters_in_winter_and_only_by_a_store() {
    // The teacher, leaving Lander (which has shops) in January.
    let mut g = on_the_road(13, TEACHER, 0);
    assert_eq!(g.gather_mode(), Some(GatherMode::Forage));
    g.state.start_month = 1;
    assert_eq!(g.gather_mode(), Some(GatherMode::Scavenge));
    assert!(g.gather_blocked().is_none());
    let round = g.start_gather(LANDSCAPE).unwrap();
    assert_eq!(round.mode, GatherMode::Scavenge);
    assert!(round.kinds.iter().all(|k| k.mode == GatherMode::Scavenge));
    assert!(round.backdrop.as_ref().is_some_and(|f| f.scavenge));
    // Nothing behind a ghost town, and nothing out on the road.
    g.state.current_node = "south_pass_city".into();
    assert!(g.start_gather(LANDSCAPE).is_err());
    g.state.current_node = "lander".into();
    let road = road_index(&g, "riverton");
    g.set_off(road).unwrap();
    assert!(g.start_gather(LANDSCAPE).is_err());
    // Come spring, the plants are back.
    g.state.start_month = 5;
    assert_eq!(g.gather_mode(), Some(GatherMode::Forage));
    assert!(g.start_gather(LANDSCAPE).is_ok());
}

#[test]
fn poachers_poach_all_year_and_dumpsters_never_see_country_backdrops() {
    let mut g = on_the_road(14, 0, 0);
    g.state.start_month = 1;
    assert_eq!(g.gather_mode(), Some(GatherMode::Poach));
    let round = g.start_gather(LANDSCAPE).unwrap();
    assert!(round.backdrop.as_ref().is_some_and(|f| !f.scavenge));
}

#[test]
fn the_record_keeps_one_line_a_day_with_the_miles_driven() {
    let mut g = on_the_road(15, TEACHER, 0);
    g.record();
    let road = road_index(&g, "riverton");
    g.set_off(road).unwrap();
    g.drive_day().unwrap();
    g.record();
    let days: Vec<u32> = g.state.history.iter().map(|s| s.day).collect();
    assert_eq!(days.first(), Some(&1));
    assert!(days.windows(2).all(|w| w[1] == w[0] + 1), "{days:?}");
    assert_eq!(days.last(), Some(&g.state.day));
    assert!(g.state.history[0].miles > 0.0);
    // Several days at once leave no holes.
    g.state.day += 3;
    g.state.resources.money = 42.0;
    g.record();
    assert_eq!(g.state.history.len() as u32, g.state.day);
    assert_eq!(g.state.history.last().unwrap().money, 42.0);
}

#[test]
fn a_leftover_outing_cannot_be_banked_again() {
    let mut g = on_the_road(16, 0, 0);
    let mut round = g.start_gather(LANDSCAPE).unwrap();
    (round.fired, round.ammo) = (3, 7);
    g.finish_gather(&round).unwrap();
    // Seven left, unless a warden took the lot.
    assert!(matches!(g.ammo(), 0 | 7));
    let before = g.state.clone();
    assert!(g.finish_gather(&round).is_err());
    assert_eq!(g.state.resources, before.resources);
    assert_eq!(g.state.day, before.day);
}

#[test]
fn only_a_completed_mission_earns_a_high_score() {
    let mut g = on_the_road(17, TEACHER, 0);
    assert!(g.high_score().is_none(), "still driving");
    g.arrive("casper");
    let entry = g.high_score().expect("mission done");
    assert_eq!(entry.run, g.state.seed);
    assert_eq!(entry.score, g.state.score.total);
    assert_eq!(entry.name, g.state.party.members[0].name);
    let mut late = on_the_road(18, TEACHER, 0);
    late.state.day = 99;
    late.check_deadline();
    assert_eq!(late.state.screen, Screen::GameOver);
    assert!(late.high_score().is_none());
}

#[test]
fn the_ranch_hand_is_issued_a_horse_and_never_sees_the_car_lot() {
    let mut g = Game::new_run(19);
    let hand = mounted_on_a_horse(&g);
    g.choose_profession(hand).unwrap();
    g.confirm_party().unwrap();
    g.choose_month(6).unwrap();
    assert_eq!(g.state.screen, Screen::Store);
    assert_eq!(g.ride(), Ride::Horse);
    assert_eq!(g.state.truck.fuel_gallons, g.state.truck.tank_gallons);
    let stock: Vec<&str> = g.stock().into_iter().map(|(id, _)| id).collect();
    assert!(stock.contains(&"hay_sack") && stock.contains(&"horseshoes"));
    assert!(!stock.contains(&"spare_tire") && !stock.contains(&"gas_can"));
    // Nobody else is offered a horse, in the lot or the store.
    let mut other = Game::new_run(19);
    other.choose_profession(0).unwrap();
    other.confirm_party().unwrap();
    other.choose_month(6).unwrap();
    let horse = other
        .content
        .vehicles
        .iter()
        .position(|v| v.id == "horse")
        .unwrap();
    assert!(other.buy_truck(horse).is_err());
    assert!(other.quote(&Purchase::Truck(horse)).is_err());
    other.buy_truck(0).unwrap();
    assert!(!other.stock().iter().any(|(id, _)| *id == "horseshoes"));
}

#[test]
fn a_horse_covers_less_ground_grazes_when_it_can_and_meets_no_motor_trouble() {
    let motor_only = ["Flat tire", "Speed trap", "Dead battery", "Overheating"];
    for seed in 0..60 {
        let mut g = on_a_horse(seed);
        let road = road_index(&g, "dubois");
        g.set_off(road).unwrap();
        g.drive_day().unwrap();
        let left = g.state.en_route.as_ref().map_or(0.0, |e| e.remaining_miles);
        assert!(
            left >= 75.0 - 35.0 - 12.0 - 1e-6,
            "seed {seed}: {left} left"
        );
        assert_ne!(g.state.screen, Screen::Hitchhiker);
        if let Some(event) = &g.state.event {
            assert!(!motor_only.contains(&event.name.as_str()), "{}", event.name);
        }
    }
    let mut g = on_a_horse(1);
    g.state.truck.fuel_gallons = 0.0;
    g.rest(false).unwrap();
    assert!(g.state.truck.fuel_gallons > 0.0, "grass in June");
    g.state.start_month = 1;
    g.state.truck.fuel_gallons = 0.0;
    g.rest(false).unwrap();
    assert_eq!(g.state.truck.fuel_gallons, 0.0, "none in winter");
}

#[test]
fn the_crew_can_camp_on_the_road_and_rest_several_days() {
    let mut g = on_the_road(20, 0, 0);
    let road = road_index(&g, "riverton");
    g.set_off(road).unwrap();
    assert!(g.rest(true).is_err(), "no motels out here");
    for m in &mut g.state.party.members {
        m.fatigue = 90.0;
    }
    g.state.resources.food = 100;
    g.camp(3).unwrap();
    assert_eq!(g.state.day, 4);
    assert!(g.state.party.members.iter().all(|m| m.fatigue <= 30.0));
    assert_eq!(g.state.screen, Screen::Travel, "still on the way");
    assert!(g.camp(0).is_err());
    // Resting past the deadline ends it, and stops counting days.
    g.camp(60).unwrap();
    assert_eq!(g.state.screen, Screen::GameOver);
    assert!(g.state.day <= g.job().unwrap().goal.days + 1);
}

/// A ranch hand who has left the store, in June.
fn on_a_horse(seed: u64) -> Game {
    let mut g = Game::new_run(seed);
    let hand = mounted_on_a_horse(&g);
    g.choose_profession(hand).unwrap();
    g.confirm_party().unwrap();
    g.choose_month(6).unwrap();
    g.leave_store().unwrap();
    g
}

#[test]
fn the_trustifarian_gets_the_van_and_alone_may_collect() {
    let mut g = Game::new_run(21);
    let job = g
        .content
        .professions
        .iter()
        .position(|p| p.name == "Trustifarian")
        .unwrap();
    g.choose_profession(job).unwrap();
    g.confirm_party().unwrap();
    g.choose_month(6).unwrap();
    assert_eq!(g.state.screen, Screen::Store);
    assert_eq!(g.vehicle().map(|v| v.id.as_str()), Some("sprinter"));
    g.buy("rotary").unwrap();
    assert!(g.buy("rotary").is_err(), "one is plenty");
    assert_eq!(g.collection().len(), 1);
    g.leave_store().unwrap();
    // Camping in the van beats camping in a truck.
    for m in &mut g.state.party.members {
        m.fatigue = 80.0;
    }
    g.state.resources.food = 100;
    g.rest(false).unwrap();
    assert!(g.state.party.members.iter().all(|m| m.fatigue <= 35.0));
    // Nobody else is offered the collection.
    let other = on_the_road(21, DROPOUT, 0);
    assert!(!other.stock().iter().any(|(id, _)| *id == "rotary"));
}

#[test]
fn a_collection_raises_the_winning_score() {
    let job = Game::new_run(1).content.professions.len() - 1;
    let score = |collect: bool| {
        let mut g = on_the_road(22, job, 0);
        assert_eq!(g.job().unwrap().name, "Trustifarian");
        if collect {
            g.state.resources.add_item("jukebox", 1);
        }
        for stop in ["thermopolis", "cody", "jackson"] {
            g.arrive(stop);
        }
        assert_eq!(g.state.screen, Screen::Win);
        g.state.score.total
    };
    assert_eq!(score(true) - score(false), 1300);
}

/// The job that comes with the horse.
fn mounted_on_a_horse(g: &Game) -> usize {
    let jobs = &g.content.professions;
    jobs.iter()
        .position(|p| p.mount.as_deref() == Some("horse"))
        .unwrap()
}

#[test]
fn every_road_on_the_map_can_be_driven_to_its_end() {
    let map = crate::core::map_graph::MapGraph::load();
    for (i, road) in map.roads.iter().enumerate() {
        for job in [0, mounted_on_a_horse(&Game::new_run(1))] {
            let mut g = on_the_road(i as u64, job, 0);
            Rc::make_mut(&mut g.content).professions[job].goal.days = 999;
            g.state.current_node = road.from.clone();
            g.set_off(i).unwrap();
            for _ in 0..60 {
                g.state.resources.food = 200;
                g.state.resources.money = 5_000.0;
                for m in &mut g.state.party.members {
                    m.health = 100.0;
                }
                g.state.truck.fuel_gallons = g.state.truck.tank_gallons;
                match g.state.screen {
                    Screen::Travel => g.drive_day().unwrap(),
                    Screen::Event => g.continue_journey(),
                    Screen::Hitchhiker => g.pick_up(false).unwrap(),
                    _ => break,
                }
            }
            assert_eq!(
                g.state.current_node, road.to,
                "{} to {}",
                road.from, road.to
            );
            assert!(g.state.en_route.is_none());
        }
    }
}

#[test]
fn driving_on_by_itself_stops_for_trouble() {
    let mut g = on_the_road(23, 0, 0);
    g.set_off(road_index(&g, "dubois")).unwrap();
    g.state.resources.food = 100;
    g.state.truck.fuel_gallons = g.state.truck.tank_gallons;
    assert_eq!(g.reason_to_stop(), None);
    let mut dry = g.clone();
    dry.state.truck.fuel_gallons = 0.5;
    assert!(dry.reason_to_stop().unwrap().contains("gas"));
    let mut hungry = g.clone();
    hungry.state.resources.food = 0;
    assert!(hungry.reason_to_stop().unwrap().contains("food"));
    g.state.party.members[2].health = 10.0;
    let name = g.state.party.members[2].name.clone();
    assert!(g.reason_to_stop().unwrap().contains(&name));
}

#[test]
fn a_save_finds_its_job_by_name_after_the_list_changes() {
    let g = on_the_road(24, TEACHER, 0);
    let mut shuffled = g.state.clone();
    shuffled.profession = 0;
    let loaded = Game::from_state(g.content.clone(), shuffled).unwrap();
    assert_eq!(loaded.state.profession, TEACHER);
    // A job that has been dropped takes its save with it.
    for gone in ["Reformed Backyard Chemist", ""] {
        let mut old = g.state.clone();
        old.job = gone.into();
        assert!(
            Game::from_state(g.content.clone(), old).is_none(),
            "{gone:?}"
        );
    }
}

#[test]
fn the_store_only_offers_what_the_party_can_use() {
    let stocked =
        |g: &Game| -> Vec<String> { g.stock().iter().map(|(id, _)| id.to_string()).collect() };
    // A rig hand fixes hoses and batteries for nothing.
    let hand = on_the_road(25, 0, 0);
    let shelf = stocked(&hand);
    assert!(!shelf.contains(&"duct_tape".to_string()), "{shelf:?}");
    assert!(!shelf.contains(&"jumper_cables".to_string()), "{shelf:?}");
    assert!(shelf.contains(&"spare_tire".to_string()));
    // Everyone else still needs them, and a teacher needs no ammunition.
    let mut teacher = on_the_road(25, TEACHER, 0);
    let shelf = stocked(&teacher);
    assert!(shelf.contains(&"duct_tape".to_string()));
    assert!(!shelf.contains(&"ammo".to_string()));
    // A thing that lasts leaves the shelf once you own one.
    assert!(shelf.contains(&"banjo".to_string()));
    teacher.state.resources.add_item("banjo", 1);
    assert!(!stocked(&teacher).contains(&"banjo".to_string()));
    // Every stop, every job: nothing on any shelf is dead weight.
    let jobs = teacher.content.professions.len();
    for job in 0..jobs {
        let mut g = on_the_road(26, job, 0);
        for node in g.content.map.nodes.clone() {
            g.state.current_node = node.id.clone();
            let mounted = g.ride() == Ride::Horse;
            for (id, item) in g.stock() {
                assert!(
                    item.ride.is_none_or(|r| (r == Ride::Horse) == mounted),
                    "{id} at {}",
                    node.id
                );
            }
        }
    }
}

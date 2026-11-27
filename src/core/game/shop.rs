//! Buying: quotes, the store, fuel and a night's rest.

use super::*;

impl Game {
    /// Items on sale at the current stop: food first, then by price.
    pub fn stock(&self) -> Vec<(&str, &ItemDef)> {
        let Some(node) = self.node() else {
            return Vec::new();
        };
        let mut list: Vec<(&str, &ItemDef)> = self
            .content
            .items
            .ids()
            .filter_map(|id| self.content.items.get(id).map(|d| (id, d)))
            .filter(|(_, d)| {
                if d.at.is_empty() {
                    d.sold_at.iter().any(|&s| node.services.has(s))
                } else {
                    d.at.contains(&node.id)
                }
            })
            .filter(|(_, d)| {
                let job = self.job().map(|j| j.name.as_str());
                d.for_job.as_deref().is_none_or(|only| job == Some(only))
            })
            .filter(|(_, d)| d.gather.is_none() || d.gather == self.gather_mode())
            .filter(|(_, d)| d.ride.is_none_or(|r| r == self.ride()))
            .filter(|(id, d)| self.is_useful(id, d))
            .collect();
        list.sort_by(|a, b| {
            (a.1.kind != ItemKind::Food, a.1.price)
                .partial_cmp(&(b.1.kind != ItemKind::Food, b.1.price))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        list
    }

    /// Would buying one more of this do this party any good? The store only
    /// offers what would.
    fn is_useful(&self, id: &str, item: &ItemDef) -> bool {
        // One of anything that lasts is all there's any use for.
        if item.reusable && self.state.resources.count(id) > 0 {
            return false;
        }
        match item.kind {
            ItemKind::Food | ItemKind::Ammo | ItemKind::FuelCan | ItemKind::Collectible => true,
            _ => {
                // A fix the job already does for free is no use, and gear is
                // only gear if some trouble this party can meet calls for it.
                let free = self.perk().free_repairs && item.repair;
                let ride = self.ride();
                !free
                    && self
                        .content
                        .events
                        .all()
                        .any(|e| e.needs.as_deref() == Some(id) && e.ride.is_none_or(|r| r == ride))
            }
        }
    }

    pub fn open_store(&mut self) -> Result<(), String> {
        if !self.stopped() || self.stock().is_empty() {
            return Err("nothing for sale here".into());
        }
        self.state.next_screen = self.state.screen;
        self.state.screen = Screen::Store;
        self.state.log.clear();
        Ok(())
    }

    pub fn leave_store(&mut self) -> Result<(), String> {
        if !matches!(
            self.state.screen,
            Screen::Store | Screen::Trade | Screen::Work
        ) {
            return Err("you can't do that right now".into());
        }
        self.state.screen = self.state.next_screen;
        self.state.log.clear();
        Ok(())
    }

    pub fn buy(&mut self, id: &str) -> Result<(), String> {
        self.expect_screen(Screen::Store)?;
        let item = self
            .stock()
            .into_iter()
            .find(|(i, _)| *i == id)
            .map(|(_, d)| d.clone())
            .ok_or("not sold here")?;
        if item.reusable && self.state.resources.count(id) > 0 {
            return Err(format!("One {} is plenty.", item.name));
        }
        if !self.state.resources.pay(self.item_price(&item)) {
            return Err("You can't afford that.".into());
        }
        let units = item.count.unwrap_or(1);
        if item.kind == ItemKind::Food {
            self.state.resources.food += units;
        } else {
            self.state.resources.add_item(id, units);
        }
        self.state.log = vec![format!("Bought: {}.", item.name)];
        Ok(())
    }

    /// What an item costs this player.
    pub fn item_price(&self, item: &ItemDef) -> f64 {
        item.price * self.perk().store_price
    }

    /// Price per gallon here, if there is a pump.
    pub fn fuel_price(&self) -> Option<f64> {
        let node = self.node().filter(|n| n.services.fuel)?;
        Some(node.fuel_price * self.perk().fuel_price * self.ride_def().price)
    }

    /// Gallons and dollars for filling up as far as money allows.
    pub(super) fn fuel_order(&self) -> Result<(f64, f64), String> {
        let price = self.fuel_price().ok_or("There's no pump here.")?;
        let truck = &self.state.truck;
        let wanted = truck.tank_gallons - truck.fuel_gallons;
        let gallons = wanted.min(self.state.resources.money / price);
        if gallons < 0.05 {
            return Err(if wanted < 0.05 {
                self.ride_def().full_why.clone()
            } else {
                "You're broke.".into()
            });
        }
        Ok((gallons, (gallons * price).min(self.state.resources.money)))
    }

    /// What a purchase would cost right now, or why it can't be made.
    /// Changes nothing.
    pub fn quote(&self, purchase: &Purchase) -> Result<Quote, String> {
        let money = self.state.resources.money;
        let quote = match purchase {
            Purchase::Truck(i) => {
                self.expect_screen(Screen::Outfit)?;
                let def = self.content.vehicles.get(*i).filter(|v| v.for_sale);
                let def = def.ok_or("no such truck")?;
                if money < def.price {
                    return Err(format!("You can't afford the {}.", def.name));
                }
                Quote {
                    what: def.name.clone(),
                    cost: def.price,
                    days: 0,
                }
            }
            Purchase::Item(id) => {
                self.expect_screen(Screen::Store)?;
                let item = self
                    .stock()
                    .into_iter()
                    .find(|(i, _)| i == id)
                    .map(|(_, d)| d)
                    .ok_or("not sold here")?;
                if item.reusable && self.state.resources.count(id) > 0 {
                    return Err(format!("One {} is plenty.", item.name));
                }
                let cost = self.item_price(item);
                if money < cost {
                    return Err("You can't afford that.".into());
                }
                Quote {
                    what: item.name.clone(),
                    cost,
                    days: 0,
                }
            }
            Purchase::Fuel => {
                if !(self.stopped() || self.state.screen == Screen::Store) {
                    return Err("you can't do that right now".into());
                }
                let (gallons, cost) = self.fuel_order()?;
                Quote {
                    what: {
                        let ride = self.ride_def();
                        format!("{gallons:.1} {} of {}", ride.unit, ride.fuel)
                    },
                    cost,
                    days: 0,
                }
            }
            Purchase::Motel => {
                if !self.stopped() || !self.node().is_some_and(|n| n.services.motel) {
                    return Err("There's no motel here.".into());
                }
                if money < MOTEL_COST {
                    return Err("You can't afford a room.".into());
                }
                Quote {
                    what: "a night at the motel".into(),
                    cost: MOTEL_COST,
                    days: 1,
                }
            }
            Purchase::Camp(nights) => {
                if !self.can_camp() || *nights == 0 {
                    return Err("you can't do that right now".into());
                }
                Quote {
                    what: self.ride_def().camp_night.clone(),
                    cost: 0.0,
                    days: *nights,
                }
            }
            Purchase::SellFood(n) => {
                self.expect_screen(Screen::Trade)?;
                if *n == 0 || self.state.resources.wild < *n {
                    return Err("You don't have that much wild food.".into());
                }
                Quote {
                    what: format!("{n} servings of wild food"),
                    cost: -f64::from(*n) * WILD_FOOD_PRICE,
                    days: 0,
                }
            }
            Purchase::SellItem(id) => {
                self.expect_screen(Screen::Trade)?;
                let (item, units, value) = self.pawn_offer(id).ok_or("Nobody wants that.")?;
                if self.state.resources.count(id) < units {
                    return Err("You don't have one to sell.".into());
                }
                Quote {
                    what: item,
                    cost: -value,
                    days: 0,
                }
            }
            Purchase::Work(id) => {
                self.expect_screen(Screen::Work)?;
                let gig = self
                    .offers()
                    .into_iter()
                    .find(|g| g.id == *id)
                    .ok_or("That job's gone.")?;
                Quote {
                    what: gig.name.clone(),
                    cost: -gig.pay,
                    days: gig.days,
                }
            }
        };
        Ok(quote)
    }

    /// Buy as much of a full tank as money allows.
    pub fn fill_tank(&mut self) -> Result<(), String> {
        if !(self.stopped() || self.state.screen == Screen::Store) {
            return Err("you can't do that right now".into());
        }
        let (gallons, cost) = self.fuel_order()?;
        self.state.resources.lose_money(cost);
        self.state.truck.add_fuel(gallons);
        let line = self.ride_def().bought.clone();
        let line = line
            .replace("{amount}", &format!("{gallons:.1}"))
            .replace("{cost}", &format!("{cost:.2}"));
        self.state.log = vec![line];
        Ok(())
    }

    /// Camping is free wherever the party is: at a stop or beside the road.
    pub fn can_camp(&self) -> bool {
        self.stopped_or_driving()
    }

    /// Spend a night: at a motel for money, or free in camp.
    pub fn rest(&mut self, motel: bool) -> Result<(), String> {
        if motel {
            if !self.stopped() || !self.node().is_some_and(|n| n.services.motel) {
                return Err("There's no motel here.".into());
            }
            if !self.state.resources.pay(MOTEL_COST) {
                return Err("You can't afford a room.".into());
            }
        } else if !self.can_camp() {
            return Err("you can't do that right now".into());
        }
        self.state.log.clear();
        self.night(motel);
        Ok(())
    }

    /// Camp several nights running. Stops early if the trip ends.
    pub fn camp(&mut self, nights: u32) -> Result<(), String> {
        if !self.can_camp() || nights == 0 {
            return Err("you can't do that right now".into());
        }
        self.state.log.clear();
        for _ in 0..nights {
            if self.state.screen.is_over() {
                break;
            }
            self.night(false);
        }
        Ok(())
    }

    /// One night's rest, already paid for.
    fn night(&mut self, motel: bool) {
        let comfy = self.vehicle().is_some_and(|v| v.comfy);
        let ride = self.ride_def();
        let (health, fatigue, mood, line, graze) = if comfy && !motel {
            (
                MOTEL_HEALTH_GAIN,
                MOTEL_FATIGUE_RECOVERY,
                MOTEL_MOOD_GAIN,
                "A night in the van's real bed, under organic linen.".to_string(),
                0.0,
            )
        } else if motel {
            (
                MOTEL_HEALTH_GAIN,
                MOTEL_FATIGUE_RECOVERY,
                MOTEL_MOOD_GAIN,
                "A night at the motel. The ice machine screamed, but everyone slept.".to_string(),
                0.0,
            )
        } else {
            (
                CAMP_HEALTH_GAIN,
                CAMP_FATIGUE_RECOVERY,
                CAMP_MOOD_GAIN,
                ride.camp_log.clone(),
                ride.graze,
            )
        };
        let growing = self.season(self.state.day).is_none_or(|s| s.forage);
        if graze > 0.0 && growing {
            self.state.truck.add_fuel(graze);
        }
        self.state.log.push(line);
        self.pass_day();
        for m in self.state.party.members.iter_mut().filter(|m| m.in_trip()) {
            m.apply_health(health);
            m.apply_fatigue(-fatigue);
            m.apply_mood(mood);
        }
        if !self.check_party() {
            self.check_deadline();
        }
    }
}

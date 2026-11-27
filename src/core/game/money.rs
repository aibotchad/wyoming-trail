//! Earning: selling wild food, pawning gear and odd jobs.

use super::*;

impl Game {
    /// Is there anyone here to sell to?
    pub fn can_trade(&self) -> bool {
        self.node()
            .is_some_and(|n| n.services.food || n.services.fuel || n.services.outfitter)
    }

    /// Odd jobs going here that you haven't already walked out on.
    pub fn offers(&self) -> Vec<&GigDef> {
        let Some(node) = self.node() else {
            return Vec::new();
        };
        self.content
            .gigs
            .iter()
            .filter(|g| {
                if g.at.is_empty() {
                    node.services.has(g.needs)
                } else {
                    g.at.contains(&node.id)
                }
            })
            .filter(|g| {
                let key = format!("{}@{}", g.id, node.id);
                !self.state.worked.contains(&key)
            })
            .collect()
    }

    pub(super) fn open_counter(&mut self, counter: Screen) {
        self.state.next_screen = self.state.screen;
        self.state.screen = counter;
        self.state.log.clear();
    }

    pub fn open_trade(&mut self) -> Result<(), String> {
        if !self.stopped() || !self.can_trade() {
            return Err("nobody to trade with here".into());
        }
        self.open_counter(Screen::Trade);
        Ok(())
    }

    pub fn open_work(&mut self) -> Result<(), String> {
        if !self.stopped() || self.offers().is_empty() {
            return Err("no work going here".into());
        }
        self.open_counter(Screen::Work);
        Ok(())
    }

    /// What the pawn counter pays for an item: name, units taken and cash.
    pub fn pawn_offer(&self, id: &str) -> Option<(String, u32, f64)> {
        let item = self
            .content
            .items
            .get(id)
            .filter(|i| i.kind != ItemKind::Food)?;
        Some((
            item.name.clone(),
            item.count.unwrap_or(1),
            item.price * PAWN_SHARE,
        ))
    }

    /// Sell wild food. A poacher now and then gets reported for it.
    pub fn sell_food(&mut self, servings: u32) -> Result<(), String> {
        let quote = self.quote(&Purchase::SellFood(servings))?;
        let r = &mut self.state.resources;
        r.wild -= servings;
        r.food = r.food.saturating_sub(servings);
        r.money -= quote.cost;
        self.state.log = vec![format!("Sold {} for ${:.0}.", quote.what, -quote.cost)];
        let mut rng = Rng::for_day(
            self.state.seed ^ 0x5E11,
            self.state.day.wrapping_mul(97) ^ self.state.resources.wild,
        );
        if self.gather_mode() == Some(GatherMode::Poach) && rng.chance(SELLING_RISK) {
            self.state
                .log
                .push("Your buyer's cousin is a game warden.".into());
            self.warden(&mut rng, false);
            if !self.check_party() {
                self.check_deadline();
            }
        }
        Ok(())
    }

    pub fn sell_item(&mut self, id: &str) -> Result<(), String> {
        let quote = self.quote(&Purchase::SellItem(id.to_string()))?;
        let (_, units, _) = self.pawn_offer(id).ok_or("Nobody wants that.")?;
        for _ in 0..units {
            self.state.resources.use_item(id);
        }
        self.state.resources.money -= quote.cost;
        self.state.log = vec![format!("Pawned {} for ${:.0}.", quote.what, -quote.cost)];
        Ok(())
    }

    /// Spend the days an odd job takes, then collect.
    pub fn work(&mut self, id: &str) -> Result<(), String> {
        self.expect_screen(Screen::Work)?;
        let gig = self
            .offers()
            .into_iter()
            .find(|g| g.id == id)
            .cloned()
            .ok_or("That job's gone.")?;
        self.state.screen = self.state.next_screen;
        self.state.log.clear();
        for _ in 0..gig.days {
            self.pass_day();
        }
        let scale = self.mood_scale(gig.mood);
        let mut gone = Vec::new();
        for m in self.state.party.members.iter_mut().filter(|m| m.in_trip()) {
            m.apply_fatigue(gig.fatigue);
            m.apply_mood(gig.mood * scale);
            if gig.health < 0.0 && m.apply_health(gig.health) {
                gone.push(m.name.clone());
            }
        }
        self.report_departures(gone);
        self.state.resources.money += gig.pay;
        self.state.resources.food += gig.food;
        self.state
            .worked
            .push(format!("{id}@{}", self.state.current_node));
        self.state.log.push(gig.done.clone());
        let s = if gig.days == 1 { "" } else { "s" };
        self.state
            .log
            .push(format!("+${:.0} after {} day{s}.", gig.pay, gig.days));
        if !self.check_party() {
            self.check_deadline();
        }
        Ok(())
    }
}

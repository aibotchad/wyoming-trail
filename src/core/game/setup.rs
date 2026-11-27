//! Starting a run: profession, party, vehicle and departure month.

use super::*;
use crate::core::defs::{Crew, NamePool};

impl Game {
    pub fn choose_profession(&mut self, index: usize) -> Result<(), String> {
        self.expect_screen(Screen::Profession)?;
        let p = self
            .content
            .professions
            .get(index)
            .ok_or("no such profession")?;
        let poacher = p.gather == GatherMode::Poach;
        self.state.resources.money = p.money;
        self.state.profession = index;
        self.state.job = p.name.clone();
        // Set either way, in case the player backed up and changed jobs.
        self.set_ammo(if poacher { STARTING_AMMO } else { 0 });
        self.state.screen = Screen::Party;
        self.name_crew();
        Ok(())
    }

    /// Step back to the previous question while setting up.
    pub fn setup_back(&mut self) -> Result<(), String> {
        self.state.screen = match self.state.screen {
            Screen::Party => Screen::Profession,
            Screen::Season => Screen::Party,
            Screen::Outfit => Screen::Season,
            _ => return Err("there's no going back from here".into()),
        };
        Ok(())
    }

    /// Choose whether the player character is a man or a woman. A driver
    /// still going by a default name gets one to match.
    pub fn set_female(&mut self, female: bool) -> Result<(), String> {
        self.expect_screen(Screen::Party)?;
        self.state.female = female;
        let was_default = self
            .state
            .party
            .members
            .first()
            .is_some_and(|m| self.content.names.is_default(&m.name));
        if !was_default {
            return Ok(());
        }
        let pool = self.name_pool();
        let taken: Vec<&String> = self.state.party.members.iter().map(|m| &m.name).collect();
        let list = if female { &pool.women } else { &pool.men };
        let pick = party_names(pool, self.state.seed, female)
            .into_iter()
            .next()
            .filter(|n| !taken.contains(&n))
            .or_else(|| list.iter().find(|n| !taken.contains(n)).cloned());
        if let Some(name) = pick {
            self.set_name(0, &name);
        }
        Ok(())
    }

    /// The names that suit the chosen profession's crowd.
    fn name_pool(&self) -> &NamePool {
        let crew = self.job().map_or_else(Crew::default, |j| j.crew);
        self.content.names.pool(crew)
    }

    /// Give everyone still going by a game-given name one that fits the job.
    /// Names the player typed are left alone.
    fn name_crew(&mut self) {
        let fresh = party_names(self.name_pool(), self.state.seed, self.state.female);
        let book = &self.content.names;
        for (member, name) in self.state.party.members.iter_mut().zip(fresh) {
            if book.is_default(&member.name) {
                member.name = name;
            }
        }
    }

    /// Rename a traveller. Blank names are ignored.
    pub fn set_name(&mut self, index: usize, name: &str) {
        let name: String = name.trim().chars().take(NAME_MAX_CHARS).collect();
        if name.is_empty() {
            return;
        }
        if let Some(m) = self.state.party.members.get_mut(index) {
            m.name = name;
        }
    }

    pub fn confirm_party(&mut self) -> Result<(), String> {
        self.expect_screen(Screen::Party)?;
        self.state.screen = Screen::Season;
        Ok(())
    }

    /// Pick the month to leave (1..=12).
    pub fn choose_month(&mut self, month: u32) -> Result<(), String> {
        self.expect_screen(Screen::Season)?;
        if !(1..=12).contains(&month) {
            return Err("no such month".into());
        }
        self.state.start_month = month;
        let mount = self.job().and_then(|j| j.mount.as_ref());
        let issued = mount.and_then(|id| self.content.vehicles.iter().find(|v| v.id == *id));
        if let Some(def) = issued {
            // Nothing to buy: straight to the store, with a full load.
            self.state.truck = Truck::from_def(def);
            self.state.truck.add_fuel(def.tank_gallons);
            self.state.screen = Screen::Store;
            self.state.next_screen = Screen::RoadChoice;
        } else {
            self.state.screen = Screen::Outfit;
        }
        Ok(())
    }
}

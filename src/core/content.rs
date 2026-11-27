//! Everything that is authored rather than played: the reference tables from
//! `assets/data`. Loaded once, shared by every run, never changed during one.
//! Rules read from here; only `GameState` changes.

use std::collections::BTreeMap;

use crate::core::data;
use crate::core::defs::{
    CreditDef, EventDef, EventTables, FieldDef, GatherDef, GigDef, ItemIndex, LookDef, NameBook,
    ProfessionDef, RideDef, SceneryBook, SpriteDef, VehicleDef, WardenDef, WeatherDef,
};
use crate::core::map_graph::MapGraph;

#[derive(Debug, Clone)]
pub struct Content {
    // --- what the rules use ------------------------------------------------
    pub map: MapGraph,
    pub professions: Vec<ProfessionDef>,
    pub names: NameBook,
    /// Vehicles for sale, in menu order.
    pub vehicles: Vec<VehicleDef>,
    pub items: ItemIndex,
    /// Road events, by table.
    pub events: EventTables,
    pub hitchhikers: Vec<EventDef>,
    pub weather: Vec<WeatherDef>,
    pub gigs: Vec<GigDef>,
    /// Ways of travelling, and what game wardens do.
    pub rides: Vec<RideDef>,
    pub wardens: Vec<WardenDef>,
    pub gatherables: Vec<GatherDef>,
    /// Photo backdrops for gathering.
    pub fields: Vec<FieldDef>,
    // --- how it is shown ----------------------------------------------------
    /// Character animation, by look id.
    pub looks: Vec<LookDef>,
    pub scenery: SceneryBook,
    /// Sprite sheet atlas.
    pub sprites: BTreeMap<String, SpriteDef>,
    /// Road lines for the map: `"from>to"` to `[lon, lat]` points.
    pub road_paths: BTreeMap<String, Vec<[f64; 2]>>,
    pub credits: Vec<CreditDef>,
}

impl Content {
    /// Parse the bundled data. A bad file is a build-time bug, so this panics
    /// with the file's name rather than returning an error.
    pub fn load() -> Self {
        Content {
            map: MapGraph::load(),
            professions: data::load_professions(),
            names: data::load_names(),
            vehicles: data::load_vehicles(),
            items: data::load_items(),
            events: data::load_events(),
            hitchhikers: data::load_hitchhikers(),
            weather: data::load_weather(),
            gigs: data::load_gigs(),
            rides: data::load_rides(),
            wardens: data::load_wardens(),
            gatherables: data::load_gather(),
            fields: data::load_fields(),
            looks: data::load_looks(),
            scenery: data::load_scenery(),
            sprites: data::load_sprites(),
            road_paths: data::load_road_paths(),
            credits: data::load_credits(),
        }
    }

    pub fn look(&self, id: &str) -> Option<&LookDef> {
        self.looks.iter().find(|l| l.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::map_graph::Terrain::*;

    fn picture(name: &str) -> bool {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/img")
            .join(name)
            .is_file()
    }

    /// Data files refer to each other by name; every name must resolve.
    #[test]
    fn every_job_has_a_playable_character() {
        let c = Content::load();
        for job in &c.professions {
            for female in [false, true] {
                let id = job.look(female);
                let look = c.look(id).unwrap_or_else(|| panic!("no look {id}"));
                assert!(!look.walk.is_empty(), "{id}");
                for way in ["up", "right", "down", "left"] {
                    let strip = &c.sprites[&job.hero_sprite(female, way)];
                    let frames = look.walk.iter().chain(look.aim.iter()).chain([&look.stand]);
                    for &f in frames {
                        assert!(f < strip.frames, "{id}: frame {f}");
                    }
                }
            }
        }
    }

    #[test]
    fn the_seasons_cover_the_year_with_art_that_exists() {
        let c = Content::load();
        for month in 1..=12 {
            let covering = c
                .scenery
                .seasons
                .iter()
                .filter(|s| s.months.contains(&month));
            assert_eq!(covering.count(), 1, "month {month}");
        }
        for season in &c.scenery.seasons {
            for image in season.layers.values() {
                assert!(picture(&format!("bg_{image}.png")), "{image}");
            }
            for name in season.sprites.values() {
                assert!(c.sprites.contains_key(name), "{name}");
            }
        }
    }

    /// Saves and lookups go by these names, so each must be its own.
    #[test]
    fn ids_are_unique_and_there_are_names_to_go_round() {
        fn unique<'a>(what: &str, ids: impl Iterator<Item = &'a str>) {
            let mut seen = std::collections::BTreeSet::new();
            for id in ids {
                assert!(seen.insert(id), "{what}: {id} twice");
            }
            assert!(!seen.is_empty(), "no {what}");
        }
        let c = Content::load();
        unique("vehicle", c.vehicles.iter().map(|v| v.id.as_str()));
        unique("job", c.professions.iter().map(|p| p.name.as_str()));
        unique("gig", c.gigs.iter().map(|g| g.id.as_str()));
        unique("look", c.looks.iter().map(|l| l.id.as_str()));
        unique("weather", c.weather.iter().map(|w| w.id.as_str()));
        unique("backdrop", c.fields.iter().map(|f| f.id.as_str()));
        unique("find", c.gatherables.iter().map(|g| g.name.as_str()));
        for pool in [&c.names.wyoming, &c.names.hippie] {
            for names in [&pool.men, &pool.women] {
                unique("name", names.iter().map(String::as_str));
                assert!(names.len() >= 4, "{names:?}");
                let max = crate::core::config::NAME_MAX_CHARS;
                assert!(names.iter().all(|n| n.chars().count() <= max), "{names:?}");
            }
        }
    }

    #[test]
    fn every_way_of_travelling_is_described_and_every_mount_exists() {
        use crate::core::defs::Ride;
        let c = Content::load();
        for ride in [Ride::Motor, Ride::Horse] {
            assert_eq!(c.rides.iter().filter(|r| r.ride == ride).count(), 1);
        }
        for job in &c.professions {
            let Some(mount) = &job.mount else { continue };
            let vehicle = c.vehicles.iter().find(|v| v.id == *mount);
            assert!(vehicle.is_some_and(|v| !v.for_sale), "{mount}");
        }
        assert!(c.vehicles.first().is_some_and(|v| v.for_sale));
        assert!(!c.wardens.is_empty() && c.wardens.iter().all(|w| w.weight > 0));
    }

    #[test]
    fn every_finale_has_its_cast() {
        let c = Content::load();
        let finales: Vec<_> = c
            .professions
            .iter()
            .filter_map(|p| p.goal.finale.as_ref())
            .collect();
        assert!(finales.len() >= 2);
        for f in finales {
            assert!(!f.drummers.is_empty() && !f.guests.is_empty());
            for name in f.drummers.iter().chain(&f.guests) {
                assert!(c.sprites.contains_key(name), "{name}");
            }
        }
    }

    #[test]
    fn collectibles_have_art_a_buyer_and_somewhere_to_be_bought() {
        use crate::core::defs::ItemKind;
        let c = Content::load();
        let ids: Vec<&str> = c.items.ids().collect();
        let mut seen = 0;
        for item in ids.iter().filter_map(|id| c.items.get(id)) {
            if item.kind != ItemKind::Collectible {
                continue;
            }
            seen += 1;
            let sprite = item.sprite.as_ref().expect("a picture");
            assert!(c.sprites.contains_key(sprite), "{sprite}");
            assert!(item.score > 0 && !item.at.is_empty(), "{}", item.name);
            assert!(item.at.iter().all(|id| c.map.node(id).is_some()));
            let buyer = item.for_job.as_ref().expect("a buyer");
            assert!(c.professions.iter().any(|p| p.name == *buyer), "{buyer}");
        }
        assert!(seen > 0);
        assert!(c.sprites.contains_key("goddess"));
    }

    #[test]
    fn every_dumpster_has_piles_to_dig_through() {
        let c = Content::load();
        let dumpsters: Vec<_> = c.fields.iter().filter(|f| f.scavenge).collect();
        assert!(!dumpsters.is_empty());
        for field in dumpsters {
            assert!(!field.covers.is_empty(), "{}", field.id);
            for name in &field.covers {
                assert!(c.sprites.contains_key(name), "{name}");
            }
        }
    }

    #[test]
    fn every_terrain_has_scenery_that_exists() {
        let c = Content::load();
        for t in [
            HighPlains,
            SagebrushBasin,
            RiverValley,
            Badlands,
            MountainPass,
            GeyserBasin,
        ] {
            let look = c.scenery.look(t).unwrap_or_else(|| panic!("{t:?}"));
            let layers = look.layers.iter().map(|l| &l.image);
            for image in layers.chain(look.sky_image.iter()) {
                assert!(picture(&format!("bg_{image}.png")), "{image}");
            }
            for name in look.roadside.iter().filter(|n| !n.is_empty()) {
                assert!(c.sprites.contains_key(name), "{name}");
            }
        }
        let traffic = c.scenery.traffic.iter().filter(|n| !n.is_empty());
        let s = &c.scenery;
        for name in traffic.chain([&s.brush, &s.exhaust, &s.hitchhiker]) {
            assert!(c.sprites.contains_key(name), "{name}");
        }
    }
}

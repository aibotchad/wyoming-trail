# Wyoming Trail — Progress

## Vision

A short, replayable Oregon Trail-style road trip across modern Wyoming, full of redneck and meth jokes. Pick a job with its own mission and deadline, buy a vehicle and supplies in Lander, then choose your own roads across real towns. Manage fuel, money, food and a four-person party; poach or forage, trade, take odd jobs, and survive weather, GPS mistakes and hitchhikers. Each stop pairs true Wyoming history with an invented "local legend".

## Tech & Structure

- Rust (stable) to WebAssembly (wasm32-unknown-unknown), built and served with Trunk
- Crates: wasm-bindgen, web-sys 0.3, js-sys, serde, serde_json (via `include_str!`), rand (seedable)
- Canvas 2D; mouse, touch and keyboard; no runtime calls to outside services (images and font are served with the game); localStorage save
- Run: `./start.sh` (this machine and the LAN, port 8080) or `trunk serve --open`
- Check: `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check`
- Art: `python3 tools/make_sprites.py` rebuilds the sprite sheet, atlas, parallax layers and UI sheet from `assets/vendor`

```text
index.html, start.sh, Trunk.toml, Cargo.toml
assets/data/   all authored content (see "Data files" below)
assets/img/    photos, map, sprite sheet, backgrounds, UI sheet
assets/vendor/ downloaded source art with licences
src/core/      rules, no browser code:
  defs, data, content   typed content, loaders, the read-only Content bundle
  state, party, truck, resources   the run's plain state
  game/                 rules by subject: setup, supplies, mission, shop,
                        money, wild, road, events (tests in tests.rs)
                        history (graph data), save (save validation),
                        balance (simulated players)
                        (tests in tests.rs, random play in stress.rs)
  gather, map_graph, rng, scores (top ten), config
src/web/       mod (input, actions, save), page (screen description),
               screens (words), stats (graph page), scores (top ten page and
               storage), draw (layout), mapview (state map),
               charts (bar graphs), cards (job cards), pause (menu, help),
               scene (drive view), field (gathering),
               shop (confirm box, cash badge), toast (pop-up messages),
               ui (9-slice)
tools/         make_sprites.py
```

## Data-oriented design

- **Content** (`core::content::Content`): every table from `assets/data`, loaded once and shared read-only.
- **State** (`core::state::GameState`): plain serializable data, the only thing a run changes.
- **Rules** (`core::game`, `core::gather`): transitions over state that read content; seeded and unit tested.
- **Presentation** (`src/web`): describes and draws state; changes it only through rule methods.
- `config.rs` holds global balance numbers; numbers that belong to one thing (a price, a perk, a gig) sit with it in data.
- Names that link data files are checked by tests.

### Data files

| File | Content |
|------|---------|
| `map.json`, `road_paths.json` | Stops, roads, facts, legends; road lines |
| `professions.json`, `names.json`, `looks.json` | Jobs, perks, missions, day limits; Wyoming and hippie name pools; character animation |
| `vehicles.json`, `rides.json`, `items.json` | Vehicles and the horse; words and costs of driving or riding; store goods |
| `events.json`, `hitchhikers.json`, `weather.json`, `gigs.json`, `wardens.json` | Road events, hitchhikers, weather, odd jobs, warden penalties |
| `gather.json`, `fields.json` | Animals, plants and dumpster finds; backdrops |
| `scenery.json`, `sprites.json`, `credits.json` | Drive view per terrain and season; sprite atlas (generated); attributions |
| `npcs.json`, `dialogue.json` | Parsed by tests only until Milestone 4 |

To add a town, vehicle, job, item, event, gig, weather type, plant or animal, edit data only. A new perk effect, service, terrain or item kind needs code.

## Core Design

- Player goal: complete every step of your profession's mission (one to five stops) with at least one party member. The goal list, met or not, heads each screen. All trips start in **Lander**.
- The NOLS Dropout picks up friends in Dubois, Jackson and Pinedale, digs up a drum at South Pass City and holds a full-moon circle at Sinks Canyon (16 days).
- Foraging jobs travel with hippie names. Seasons (`scenery.json`): winter Nov-Mar, spring Apr-May, summer Jun-Aug, fall Sep-Oct. In winter nothing grows, so foragers dumpster dive at stops with food or gas; a night manager may chase them off.
- Setup: profession (money, perk, score multiplier, mission, poacher or forager) -> party names -> vehicle -> general store.
- Daily loop: drive, burn fuel, eat, tire, random event. Stops offer fuel, store, motel, a night in the truck, or a poaching / foraging round.
- Gathering: a 45 second field game (`core::gather`). Poachers need ammo, count pounds of meat (200 lb carry limit) and risk a warden; foragers must avoid death camas.
- Controls: mouse/touch, arrows/Enter, number keys, M for map, S for stats, Esc back; gathering starts on a tap or Space, then WASD/arrows + Space or tap.
- Anything that costs money or a day asks first; a road can be unchosen until miles are driven; setup questions can be revisited.
- Calendar: pick a departure month; daily weather (`weather.json`) slows driving and wears the party. GPS events add miles unless you carry an atlas; optional hitchhikers help or harm.
- Money: sell wild food, pawn gear, or take odd jobs (`gigs.json`) that cost days.
- Win: arrive at your mission's last stop before its day limit. Score from survivors, health, money, places and days, times the profession multiplier.
- Top ten: completed missions only (a failed run scores 0), best first, ties to the earlier run, one line per run; stored as `wyoming-trail.scores.v1` in the browser; bad stored lines are dropped.
- Lose: the days run out, everyone leaves, or the tank runs dry with no gas can and less than the tow fee.
- Party members at 0 health leave for the hospital (no deaths).
- Scope limits: no multiplayer, no free-roam driving, no 3D, no graphic content.

## Balance

Measured with `core::game::balance` (600 runs a job, leaving May to September) on 2026-11-26. The "sensible" player stocks up, refuels, rests when worn out and avoids poaching in towns; the "careless" one skips spares, dawdles and takes chances.

| Job | Cash | Days | Score | Sensible wins | Careless wins |
|-----|------|------|-------|---------------|---------------|
| Oil Rig Roughneck | 1950 | 7 | x2 | 100% | 84% |
| Gas Station Night Clerk | 2400 | 5 | x2 | 100% | 84% |
| Rodeo Clown | 2200 | 6 | x3 | 100% | 83% |
| Drunk Ranch Hand (horse) | 900 | 9 | x4 | 99% (89% in winter) | 88% |
| Trustifarian (van) | 9000 | 19 | x1 | 100% | 89% |
| Jackson Hole Ski Bum | 2000 | 6 | x2 | 100% | 88% |
| Newly Hired Teacher | 2200 | 6 | x3 | 100% | 93% |
| NOLS Dropout | 2100 | 14 | x4 | 100% | 75% |

- Redneck against hippie (careless): 84.4% against 86.6% wins.
- One round in the country: poaching about 44 servings, $46 average fine, $20 of ammo; foraging 30 servings and a little poison. In towns and parks poaching drops to about 29 servings with a $164 average fine.
- Warden: 11% of country rounds, 40% in towns and parks; $150, $450 (takes ammo) or $950 (takes ammo and a day). Unpaid fines are served as an extra day.
- A sensible player rarely runs short of cash (about $550-1,070 left over). Raise grocery prices or cut starting money for a harder game.

## Map Graph (data-driven; edit `assets/data/map.json`)

34 stops and 43 roads (each listed once; the loader adds the return direction). Coordinates and facts come from Wikipedia, mileages and road geometry from OSRM / OpenStreetMap (fetched 2026-11-26). Services, fuel prices, road risk, scenery and "legends" are game inventions. Map image: USGS tiles; photos: Wikimedia Commons (see CREDITS.md).

Missions are data in `assets/data/professions.json`; animals and plants in `assets/data/gather.json`.

## Current Status

- State: 128 tests pass (3 report tests are run by hand); clippy (zero warnings, no `allow` attributes) and fmt clean. Played in headless Chrome on desktop and emulated phone sizes with touch events; about 15,000 random browser actions raised no errors.
- Last session (2026-11-26): made the browser build work, then added everything below at the owner's request. It is on the `playable-game` branch, offered upstream as a pull request.

### Done this session

| Area | What |
|------|------|
| Fixes | Input listeners kept alive; Trunk entry point; animation timer overflow that crashed debug builds and froze sprite frames |
| Content | Map rebuilt (33 stops, 42 roads) with Wikipedia facts, OSRM mileages, a photo per stop, USGS state map; start moved to Lander |
| Tone | Opening warning line; labelled true story and invented legend per stop |
| Setup | 7 jobs (an eighth added later) with perks, missions and day limits; male or female character; Wyoming baby names; departure month; 4 vehicles including a 2012 Prius; general store |
| Road | Daily weather, 26 random events including GPS mix-ups, optional hitchhikers, parallax side view with traffic |
| Food and money | Poaching (pounds of meat, 200 lb carry, 45 s clock, warden) and foraging (photo backdrops, plant photos, death camas); sell wild food, pawn gear, 7 odd jobs |
| Interface | Wooden 9-slice menus, pixel font, clickable map with truck and mission marker, yes/no on every transaction, cash badge, phone layouts |
| Hosting | `start.sh` serves the LAN and builds into `target/dist-lan` |
| Playability | Phone taps aligned with the screen; flick scrolling; two-finger poaching; pop-up feedback; start gate for gathering; confirm before sleeping; back and turn-back buttons; plainer wording |
| Art | Sprite alignment fixed; seasonal scenery; dumpster finds |
| Goals and stats | Goal checklist and counter; stats page with bar graphs and daily history |
| Robustness | Saved games validated; random-play stress tests; crash screen; server no longer reloads open games |
| Top ten | High-score list kept between runs, shown at the finish and from the title screen |
| Ranch hand | Drunk Ranch Hand rides Dolly: hay and grazing, 35 miles a day, horse events and goods, three-step mission |
| Pause | MENU button or P: rest one night, three days or at a motel (on the road too), pace, rations, help |
| Dumpster scene | Jump in, dig through piles, choose what to take |
| Job screen | Cards with bold titles and sections, redneck and hippie columns |
| Trustifarian | Rich kid with dreadlocks: Sprinter van with a real bed, vintage collectibles that add score, three steps ending at a Jackson condo |
| Drum circle | Playable finale for the NOLS Dropout (Sinks Canyon, full moon, Groove Goddess) and the Trustifarian (condo, parents); the dropout's countdown reads "days until the full moon" |
| Live driving | One press drives on until arrival, a stop, an event, or low fuel, food or health; map marker follows the highway; any town can be headed for from its card |
| Road country | Each road lists the terrain along it (`country` in `map.json`), fading from one to the next; mountain stretches are now drawn by day |

## Code review (2026-11-26)

A high-effort review of the uncommitted work found 10 defects, all fixed:

| Finding | Fix | Test |
|---------|-----|------|
| Taps on the status bar or cash badge pressed buttons hidden under them | Covered points ignore page hotspots and the field | browser only |
| Food taken by events could still be sold | Wild food is capped to food held | yes |
| Events that add gas did nothing | Gas is added to the tank | yes |
| A wrong turn charged gas twice | Fuel is burned only by driving | yes |
| Days lost on the last leg could still win after the deadline | No arrival once overdue | yes |
| Tapping a plant near an edge left the forager walking in place | Tap target kept on walkable ground | yes |
| Drive animation could show negative miles after a wrong turn | Extra miles lengthen the leg | yes |
| Map tooltip lines overlapped | Line spacing matches the font | browser only |
| Purchase buttons hidden under the status bar in landscape | Box drawn last and kept on screen | browser only |
| Hitchhiker screen had an empty picture column on arrival | Shows the town photo | browser only |

The design pass then moved vehicles, perks, item behaviour, services, weather looks, scenery, character animation and map labels from code into data; separated `Content` from `GameState`; moved the last rules out of the web layer; split `game.rs` into modules; and removed dead code and lint suppressions.

## Milestones

### Milestone 1: Playable core with a branching map (done)

- [x] Cargo project, Trunk config, core/web module split
- [x] Canvas renders in the browser; unit tests
- [x] Screen state machine and game loop
- [x] Party, vehicle and resource data model in core
- [x] Data-driven map with tested connectivity and routing
- [x] Road choice screen (road, distance, terrain, services, advice toward the mission)
- [x] Travel day loop with pace and rations
- [x] Daily fuel, food, fatigue and health (unit tested)
- [x] Arrival screen, win and game over
- [x] Play again without reloading

### Milestone 2: Events and survival (in progress)

- [x] Central config module for balance numbers
- [x] Data-driven weighted event system with seeded RNG (unit tested)
- [x] 15+ events (26)
- [x] Weather and season (weather is statewide), terrain per road
- [ ] Road closures remove roads until reopened
- [ ] Sickness and injury with recovery (first-aid kit softens one event only)
- [ ] Vehicle condition (tires, battery, engine) affects breakdown odds

### Milestone 3: Stops, fuel and hunting (in progress)

- [x] Outfitting at the start (job, vehicle, store)
- [x] Fuel with per-town prices and range warnings
- [x] Ammo for sale; poaching and foraging minigame
- [x] Game warden fines (unit tested); carry limits
- [x] History text and photo for every stop
- [ ] Distinct diner, mechanic and outfitter services (store, motel, trading and jobs exist)
- [ ] Hunting seasons; meat spoilage without ice

### Milestone 4: NPCs and expanded world

- [x] Full road network
- [ ] NPC and dialogue data model, dialogue screen, town NPCs
- [ ] Roadside encounter system and recurring NPCs (hitchhikers are single outcomes)
- [ ] Temporary party members and side quests

### Milestone 5: Interface and feel (mostly done)

- [x] Visual style, sprites, drive animation, state map
- [x] Status bar and party read-outs
- [x] Phone and desktop layouts
- [x] Save and load with localStorage
- [ ] Sound
- [x] Help screen and pause menu

### Milestone 6: Polish and release

- [x] Final score screen
- [x] Balance pass against simulated players (see Balance); owner playtest still needed
- [ ] Fact-check against primary sources (checked against Wikipedia and OSRM only)
- [x] Local high score (top ten, per browser)
- [ ] Static hosting instructions

## Next Steps

1. Owner playtest; review jokes, facts, missions and day limits.
2. Balance pass: a simple bot wins every mission 100/100 on its starting budget.
3. Commit the work.
4. Road closures and vehicle condition (rest of Milestone 2), then talking NPCs.

## Known Issues

| # | Description | Severity | Status |
|---|-------------|----------|--------|
| 1 | Browser rendering unverified; input listeners were dropped after registration | High | Fixed 2026-11-26 |
| 2 | Inverted assertion in a party health test | Low | Fixed 2026-11-25 |
| 3 | No refuelling, so several routes could not be won | High | Fixed 2026-11-26 |
| 4 | Balance too generous | Medium | Tuned 2026-11-26; sensible play still rarely loses |
| 5 | A 17-mile road takes a whole drive day | Low | Open |
| 6 | Reloading during a gathering round discards it at no cost | Low | Open |
| 7 | Animation timer overflowed `u32` | High | Fixed 2026-11-26 |
| 8 | Canvas sized to 100vh, so phone taps landed off target | High | Fixed 2026-11-26 |
| 9 | Dev server reloaded open games, returning players to the title screen | High | Fixed 2026-11-26 |
| 10 | A damaged save with a huge day number exhausted memory | Medium | Fixed 2026-11-26 |
| 11 | The M key paused a gathering round | Low | Fixed 2026-11-26 |
| 12 | A leftover gathering round could return spent ammunition | Low | Fixed 2026-11-26 (not reachable from the screen) |
| 13 | Long functions remain: `road_view`, `attach_listeners`, `confirm_box`, `block`, `apply`, `drive_day` (about 115-170 lines) | Low | Open |

### Unverified

- [ ] A real phone or second machine on the network; Firefox and Safari
- [ ] Facts are summarised from Wikipedia only
- [ ] The terrain along each road is from general knowledge, not checked against a source
- [ ] Prius tank size (11.9 gal) not found in the source checked
- [ ] Prices, services, yields, weather odds, day limits and warden odds are invented

## Decisions

| Date/Session | Decision | Reason |
|--------------|----------|--------|
| 0 | Rust compiled to WASM, built with Trunk | Requested; simple dev/build loop |
| 0 | Pure-logic core separated from web layer | Makes rules unit-testable with cargo test |
| 0 | Canvas 2D via web-sys, no game engine | Small, easy to reason about |
| 0 | Seedable RNG | Reproducible events, encounters, and balance simulations |
| 0 | Data-driven map, events, items, NPCs, dialogue (JSON) | Expand content without touching logic |
| 0 | Modern-day setting, player drives a pickup truck | Requested; fuel, MPG, and truck condition replace oxen and wagon parts |
| 0 | Graph-based map with player-chosen roads and backtracking | Requested; makes the game non-linear and replayable |
| 0 | Multiple Yellowstone entrances (South, East) | Supports distinct routes |
| 0 | Hunting kept, with license, ammo, season, and location rules | Requested; modern and light on legal flavor |
| 0 | Hunting presented as non-graphic timing game | Family-friendly tone |
| 0 | NPCs: fixed town NPCs plus roadside encounters, branching dialogue, flags, side quests | Requested; adds variety and memory |
| 0 | Road closures replace river crossings | Fits a modern road trip |
| 0 | Party members leave the trip at 0 health instead of dying | Family-friendly tone |
| 0 | Placeholder art drawn in code first | Keep focus on gameplay |
| 2026-11-25 | 20 dead-code warnings suppressed with `#[allow(dead_code)]` + "first consumer: M#" comments on the M2–M6 placeholder APIs (constants, methods, struct) rather than deleting them | Keeps the intended API surface in place; annotations document which milestone consumes each item |
| 2026-11-25 | web-sys 0.3.107 API set: `get_context("2d")` + `dyn_into` (not deprecated `context_2d()`); `set_fill_style`/`set_font` take `&str`; `measure_text` returns `Result<TextMetrics, JsValue>` (needs `TextMetrics` cargo feature) | Compiler-verified against the pinned web-sys version |
| 2026-11-25 | `PartyMember::apply_health` contract: returns `true` exactly when this call makes them leave the trip (0 health); test corrected accordingly | Matches the documented contract and family-friendly "hospital, leave trip" rule |
| 2026-11-25 | Data files loaded via `include_str!` at compile time (no runtime fetches) | Satisfies "no external network calls at runtime"; JSON validated by parse tests |
| 2026-11-26 | Data-oriented programming adopted (PROMPT rule): content is plain typed data in `defs.rs`, parsed into `BTreeMap`-keyed tables by `data.rs`; enums carry `#[serde(other)]` fallback so newer data files don't break older builds; game logic will consume typed tables, never raw JSON | Clean separation of concerns, extensible without schema migration, deterministic iteration order; no over-engineering (no ORM-like indirection, no trait objects) |
| 2026-11-26 | `ItemKind` uses derived `Default` with `#[default]` on the `Other` variant (serde `other` catches unknown kinds at parse time) | `#[serde(other, default)]` on a variant is not valid attribute syntax (compile error); derived Default + `#[serde(other)]` is the correct combination |
| 2026-11-26 | New foundation modules (`defs.rs`, `data.rs`) get module-scoped `#![allow(dead_code)]` with rationale, not a global allow | M2 (event selection) and M3 (shop/hunter) are the first consumers; keeps clippy globally clean while the layer is in place |
| 2026-11-26 | Adult comedy tone (redneck and meth jokes) replaces "family-friendly" | Owner request |
| 2026-11-26 | Trip starts in Lander | Owner request |
| 2026-11-26 | Facts and invented "local legends" are separate, labelled fields | Jokes must never read as facts |
| 2026-11-26 | Photos and map tiles bundled in `assets/img` with credits | Owner request; keeps the no-runtime-network rule |
| 2026-11-26 | Canvas UI rebuilt as a flowing page (`screens` -> `page` -> `draw`) | Text wrapping, pictures, map, scrolling on phones |
| 2026-11-26 | Save key bumped to `wyoming-trail.v2` | Old saves do not fit the new state |
| 2026-11-26 | Each profession has its own mission instead of a shared Yellowstone goal | Owner request |
| 2026-11-26 | Ski bum replaces the mechanic; rehab run moved to the chemist | Owner request |
| 2026-11-26 | Gathering split into poaching and foraging; no hunting licence | Owner request |
| 2026-11-26 | Third-party art is CC0 where possible; elk and pronghorn derived from a CC0 deer | No free elk or pronghorn found |
| 2026-11-26 | Jersey 15 pixel font | Larger, solid, readable digits |
| 2026-11-26 | `start.sh` binds 0.0.0.0 | Owner wants LAN access |
| 2026-11-26 | Player characters are CC-BY 3.0 (Antifarea; Wasilev cowboy for male poachers); credit and links are required | Owner picked the packs |
| 2026-11-26 | Every purchase asks yes / no (No highlighted); cash badge top right where money can be spent | Owner request |
| 2026-11-26 | Poaching counts pounds of meat, 200 lb carry limit, 45 s clock | Owner request |
| 2026-11-26 | Default party names are very-Wyoming baby names (`names.json`) | Owner request |
| 2026-11-26 | Every mission has a day limit | Owner request |
| 2026-11-26 | Only wild food can be sold; pawned gear pays half the shop price | No buy-and-resell money loop |
| 2026-11-26 | Weather is derived from seed and date, not stored | The forecast always matches the drive |
| 2026-11-26 | Gathering backdrops and plant pictures are Wikimedia photos with credits | Owner request |
| 2026-11-26 | `Game` = `Rc<Content>` + `GameState`; content is read-only during play | Data, rules, state and presentation stay separate |
| 2026-11-26 | Perks are multipliers in `professions.json`, vehicles live in `vehicles.json` | Adding a job or vehicle is data only (these balance numbers now live with their data, not in `config.rs`) |
| 2026-11-26 | Save key `wyoming-trail.v5` | State layout changed; older saves are ignored |
| 2026-11-26 | Winter substitute for foraging is dumpster diving, Nov-Mar | Owner request; months chosen to suit Wyoming |
| 2026-11-26 | Teacher and ski bum count as hippie crews, like the dropout | They are the foraging jobs |
| 2026-11-26 | Friends in the drum circle mission are story only: they don't eat or take seats | Keeps the party at four |
| 2026-11-26 | Animation repaints from the timer; scrolling and pointer repaints wait for the screen refresh | Measured cheapest on a slowed phone profile |
| 2026-11-26 | Canvas density capped at 2 | Large saving while the road animates |
| 2026-11-26 | The Live A Live horse sheet on spriters-resource was not used; the horse is drawn by `make_sprites.py` | It is ripped commercial art |
| 2026-11-26 | Forage haul capped at 30 servings, meat at 4 lb a serving | Foraging beat poaching on every count |
| 2026-11-26 | The Trustifarian's "rich New York or California neighbourhood" is a gated enclave in Jackson | The map is Wyoming only |
| 2026-11-26 | Driving stops itself on low fuel, low food, poor health, or when the window loses focus | Nothing should go wrong unwatched |
| 2026-11-26 | The roughneck's mission is getting back to the oil patch after losing everything at the Wind River Casino; the chemist job is removed | Owner request |
| 2026-11-26 | Saves record the job by name | So jobs can be added or dropped without giving a saved run the wrong one; older saves are no longer offered |
| 2026-11-26 | Choosing a road starts driving at once, unless the tank can't finish it | Trips looked stuck at 0 miles |
| 2026-11-26 | The store lists only what the party can use | Owner request |

## Backlog (ideas, not committed)

- Wildlife photography mode as an alternative to hunting
- Truck and driver "profession" choices affecting starting money and skills
- Trip journal and postcard collection from each landmark
- Playlist/radio feature that affects morale
- Achievements (try every route, help every NPC, survive a wind closure)
- Difficulty levels and a winter-only hard mode
- More side trips (Devils Tower expansion, Thermopolis hot springs, Medicine Bow)

## Session Log

| # | Summary | Files changed |
|---|---------|---------------|
| 1 | Built the full skeleton: Cargo/Trunk setup, core modules (config, state, party, truck, resources, map_graph, data, rng, game), 5 data JSON files, thin web layer (canvas render + input + localStorage save/load), wasm entry. Fixed every compile error against web-sys 0.3.107. Cleared all dead-code + clippy warnings (annotations with milestone "first consumer" notes). Fixed inverted party-health test. Green: 0 check warnings, clippy clean, fmt clean, 18/18 tests. Committed and pushed to GitHub `main`. | `Cargo.toml`, `Cargo.lock`, `Cargo.config` (`.cargo/config.toml`), `Trunk.toml`, `index.html`, `assets/data/*.json`, `src/core/*.rs`, `src/web/mod.rs`, `src/lib.rs`, `src/main.js`, `.gitignore`, `PROGRESS.md` |
| 2 | 2026-11-26 — DOP data layer: new `src/core/defs.rs` (typed content model for events/items/npcs/dialogue; enums with `#[serde(other)]` fallbacks; `BTreeMap`-keyed tables for deterministic iteration), typed `include_str!` loaders in `data.rs` (load_events/load_items/load_npcs/load_dialogue → typed tables), DOP rule added to PROMPT.md, `pub mod defs` wired in `core/mod.rs`. Fixed: mangled JSON literal in `event_def_round_trips`, missing `Default` for `ItemKind` (now derived), `derivable_impls` clippy warning, invalid `#[serde(other, default)]` attribute. Green: fmt clean, 22/22 tests, clippy zero warnings, native + wasm32 builds clean. Committed and pushed to GitHub `main`. | `src/core/defs.rs` (new), `src/core/data.rs`, `src/core/mod.rs`, `PROMPT.md`, `PROGRESS.md` |
| 3 | 2026-11-26 - Browser build fixed; fact-checked map; photos, USGS state map, professions, store, fuel, rest, events, jokes; start moved to Lander. 33 tests, clippy and fmt clean, played in headless Chrome. | `src/**`, `assets/**`, `index.html`, `Cargo.toml`, `README.md`, `PROMPT.md`, `CREDITS.md`, `PROGRESS.md` |
| 4 | 2026-11-26 - Added the 2012 Prius (faded Hillary sticker) to the car lot, owner request. 34 tests pass. | `src/core/config.rs`, `src/core/game.rs`, `src/web/*.rs`, `README.md` |
| 5 | 2026-11-26 - Missions, ski bum, parallax sprite animation, poaching/foraging, pixel font, mobile layout, LAN start script. 52 tests. | `src/**`, `assets/**`, `tools/make_sprites.py`, `start.sh`, docs |
| 6 | 2026-11-26 - Animation timer overflow fixed; wooden 9-slice menus; male/female character; purchase popup; meat weight and timer; new names; `start.sh` builds to `target/dist-lan`. 56 tests. | `src/**`, `assets/**`, `tools/make_sprites.py`, `start.sh`, docs |
| 7 | 2026-11-26 - Photo backdrops and plant photos; departure month and weather; GPS mishaps; hitchhikers; trading, odd jobs, deadlines. 71 tests. | `src/**`, `assets/**`, docs |
| 8 | 2026-11-26 - Code review (10 fixes) and data-oriented refactor. 78 tests. | `src/**`, `assets/data/*`, `PROGRESS.md` |
| 9 | 2026-11-26 - Foraging plants redrawn at 56 px from the real species; a tap anywhere on a plant picks it. 80 tests. | `tools/make_sprites.py`, `assets/data/gather.json`, `src/core/gather.rs` |
| 10 | 2026-11-26 - Road scenery keeps scrolling between day replays and holds still at a hitchhiker stop. 81 tests. | `src/web/scene.rs` |
| 11 | 2026-11-26 - Sprite and touch fixes, playability pass, NOLS drum circle mission with Sinks Canyon, hippie names, goal list, seasons, winter dumpster diving, stats graphs, optimization, robustness pass. 98 tests. | `src/**`, `assets/**`, `tools/make_sprites.py`, `index.html`, `start.sh`, docs |
| 12 | 2026-11-26 - Fragile-function check (map and gathering drawing split, verified pixel-identical; brittle spots fixed) and top ten list. 107 tests. | `src/**`, `README.md`, `PROGRESS.md` |
| 13 | 2026-11-26 - Balance pass with simulated players, warden penalty table, Drunk Ranch Hand and horse, pause menu, jump-in dumpster scene, job cards. 116 tests. | `src/**`, `assets/**`, `tools/make_sprites.py`, docs |
| 14 | 2026-11-26 - Trustifarian and vintage collecting, playable drum circle finale, live driving, terrain along each road, daytime mountains. 124 tests. | `src/**`, `assets/**`, `tools/make_sprites.py`, docs |
| 15 | 2026-11-26 - The owner's dumpster art (from `scratch_window`, MIT) shows outside the dumpster before, during and after a dive. | `assets/vendor/spacestate1`, `tools/make_sprites.py`, `src/web/field.rs`, `CREDITS.md` |
| 16 | 2026-11-26 - Roughneck casino story, chemist removed, trips start at once and drive faster, store limited to usable goods, flashing dumpster find cards. 128 tests. | `src/**`, `assets/data/*`, docs |

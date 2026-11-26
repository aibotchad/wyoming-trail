# Wyoming Trail — Progress

## Vision

A short, replayable Oregon Trail-style road trip across modern Wyoming. Players outfit a pickup truck in Cheyenne, then choose their own roads across a network of real towns and landmarks. They manage fuel, money, food, and a four-person party, hunt for food, meet memorable NPCs, survive wind closures, wildlife, and breakdowns, and learn real Wyoming history on the way to Yellowstone. Fun comes from route planning, risky choices, resource tension, and funny/dramatic encounters. Different routes should make different runs.

## Tech & Structure

- Language: Rust (stable), compiled to WebAssembly (wasm32-unknown-unknown)
- Build/serve: Trunk (`trunk serve`, `trunk build --release`)
- Key crates: wasm-bindgen, web-sys 0.3.107, js-sys, serde, serde_json (via `include_str!`), rand (seedable, getrandom "js" feature)
- Rendering: Canvas 2D via web-sys; keyboard + mouse/touch input
- No external network calls at runtime; localStorage save with graceful failure
- How to run: `trunk serve`, then open the printed localhost URL
- How to test: `cargo test` (core logic), `cargo clippy --all-targets`, `cargo fmt --check`
- Folder layout (actual):

```text
Cargotoml, Cargo.lock, Trunk.toml, index.html, .cargo/config.toml
assets/data/   map.json, events.json, items.json, npcs.json, dialogue.json
src/lib.rs     (wasm entry point)
src/core/      mod, config, state, party, truck, resources, travel logic in game,
               map_graph, data, rng, game     (NO web-sys/js-sys deps)
src/web/       mod.rs — canvas render, input, storage (thin)
src/main.js    (small glue)
```

## Core Design

- Player goal: Reach Yellowstone (any entrance) with at least one party member still on the trip.
- Core mechanics: Graph-based map with road choice at junctions; daily driving loop with pace (hours driven) and meals; fuel/money/food/ammo management; random events; road closures and rerouting; hunting minigame; NPC encounters and dialogue.
- Controls: Mouse/touch buttons and number/arrow/Enter keys for menus.
- Win: Arrive at Yellowstone (South via Grand Teton, East via Cody) with 1+ party members. Score from party health and morale, money, truck condition, landmarks visited, NPCs helped, and days taken.
- Lose: Truck stranded with no fuel/money and no way to get help, or every party member has left the trip.
- Party members at 0 health: taken to a hospital and leave the trip (family-friendly, no deaths).
- Scope limits (what this is NOT): No multiplayer, no free-roam driving, no real-time action beyond the hunting minigame, no 3D, no graphic content.

## Map Graph (data-driven; edit via `assets/data/map.json`)

Requirement: at least 3 meaningfully different routes from Cheyenne to a Yellowstone entrance, with backtracking allowed.

Starting sketch of roads (verify every road name and mileage in M6):

| Road | Connects | Notes |
|------|----------|-------|
| I-80 | Cheyenne - Laramie - Rawlins - Rock Springs | Wind closures, trucks |
| US-30/287 | Laramie - Medicine Bow | Quiet, long gaps |
| WY-487 | Medicine Bow - Casper | Remote basin |
| I-25 | Cheyenne - Douglas - Casper - Buffalo | Main north-south route |
| US-85/US-26 | Cheyenne - Fort Laramie - Guernsey (Register Cliff) | History stops |
| WY-220 | Casper - Independence Rock - Devil's Gate - Muddy Gap - Rawlins | Historic trail corridor |
| US-287 | Muddy Gap - Split Rock - Lander | Services gap |
| WY-28 | Lander - South Pass - Farson | Optional detour |
| US-20/26 | Casper - Shoshoni - Thermopolis | Fuel and hot springs |
| Lander-Riverton-Shoshoni link | Lander - Shoshoni | Connector |
| WY-120 | Thermopolis - Cody | Toward east entrance |
| US-14/16/20 | Cody - Yellowstone East Entrance | Finish |
| US-16 | Buffalo - Ten Sleep - Cody | Mountain scenery |
| US-14 | Sheridan - Cody (Bighorn Mountains) | Scenic, slow, weather risk |
| I-90 | Buffalo - Sheridan, Buffalo - Gillette | Northern links |
| US-14 | Gillette - Devils Tower | Side trip |
| US-287/26 | Lander - Dubois - Togwotee Pass - Jackson | Pass hazards |
| US-191 | Rock Springs - Pinedale - Jackson | Alternate southern approach |
| US-26/89/191 | Jackson - Grand Teton - Yellowstone South Entrance | Finish |

Sample routes to support (examples):

1. Central: Cheyenne - Casper - Lander - Dubois - Jackson - Grand Teton - Yellowstone (South)
2. Northern: Cheyenne - Douglas - Casper - Buffalo - Cody - Yellowstone (East)
3. Southern: Cheyenne - Laramie - Rawlins - Rock Springs - Pinedale - Jackson - Grand Teton - Yellowstone (South)

## Current Status

- Milestone: 1 of 6 (in progress — logic core done, browser verification pending)
- State: All Rust code compiles with **zero warnings**; `cargo clippy --all-targets` clean; `cargo fmt --check` clean; **22/22 core unit tests pass** (native + wasm32 builds verified)
- Last session summary: 2026-11-26 — data-oriented refactor: new typed content layer `src/core/defs.rs` (EventDef/EventTables, ItemDef/ItemKind, NpcRole/NpcTables, DialogueBook, ItemIndex) + typed `include_str!` loaders in `data.rs`; DOP rule added to PROMPT.md; committed and pushed to GitHub (`main`)

## Milestones

### Milestone 1: Playable core with a branching map (current)

- [x] Cargo project, Trunk config, core/web module split
- [ ] Canvas renders a title screen in the browser; one passing unit test (unit tests pass — 18 of them; **browser rendering not yet verified in-browser via `trunk serve`**)
- [x] Screen state machine (enum) and game loop
- [x] Party creation (4 names) and party/truck/resource data model in core
- [x] Map graph data model and loader (map.json) with a test map (3+ routes, tested)
- [x] Road Choice at junctions (road name, distance, terrain, services) — logic + screen data in core
- [x] Travel/day-loop screen: day counter, miles progress, pace and meal selectors — data model + logic in core
- [x] Daily update: fuel burned, food eaten, fatigue and health change (unit tested)
- [x] Arrive at node and show landmark/town screen (placeholder text)
- [x] Win at Yellowstone and game over conditions
- [x] Restart without reloading — `Game::restart(seed)` implemented (UI wiring lands with M2 menus)

### Milestone 2: Events and survival

- [x] Central config module for all balance numbers (`src/core/config.rs`)
- [ ] Event system (data-driven, weighted, condition-aware; seeded RNG; unit tested) — **typed content layer done** (`defs.rs`: EventTables/NpcTables/ItemIndex/DialogueBook; `data.rs`: parsers), RNG seeded in `rng.rs`, first rule consumers pending
- [ ] 15+ events: wind closure, blizzard, wildlife on road, flat tire, overheating, dead battery, cell dead zone, speeding ticket, construction, etc.
- [ ] Road closures remove roads from choices until reopened; reroute and backtrack logic
- [ ] Sickness/injury and recovery; first-aid and medicine items
- [ ] Truck condition (tires, battery, engine) affects travel and breakdown odds — fields in `truck.rs`
- [ ] Weather, season, and terrain per road

### Milestone 3: Stops, fuel, and hunting

- [ ] Cheyenne outfitting screen (starting money, truck choice, supplies)
- [ ] Gas station: buy fuel, prices vary by location; services-gap warnings — `FUEL_PRICE_PER_GAL` configured
- [ ] Diner, motel/campground, mechanic, outfitter at appropriate stops
- [ ] Hunting license and ammo purchase at outfitters
- [ ] Hunting minigame (non-graphic timing/aim), legal game by season and location
- [ ] Hunting restrictions (no hunting in national parks or towns), food carry limits, spoilage without cooler/ice
- [ ] Game warden fine logic for violations (unit tested)
- [ ] History/fun-fact screens for every landmark

### Milestone 4: NPCs and expanded world

- [ ] NPC and dialogue data model (nodes, choices, conditions, effects, flags) with unit tests — `npcs.json`/`dialogue.json` loaded
- [ ] Dialogue screen
- [ ] Town NPCs (diner, motel, mechanic, outfitter, historian) with road reports
- [ ] Roadside encounter system (road, terrain, season, weather conditions; seeded RNG)
- [ ] 12+ roadside encounters (trucker on CB, ranger, trooper, game warden, hitchhiker, tourists, tow driver, rancher, storyteller, fellow road-tripper)
- [ ] Temporary party member (hitchhiker) mechanics
- [ ] Side quests (deliver a package, return a dog, give a ride)
- [ ] NPC memory/flags and recurring NPCs; score for helping NPCs
- [ ] Expand map to the full network (all three sample routes plus side trips)

### Milestone 5: Interface and feel

- [ ] Visual style pass (retro palette, pixel sprites drawn in code, truck driving animation, simple Wyoming map screen)
- [ ] HUD: date, weather, fuel, money, food, health/morale, next-stop distance
- [ ] Sound effects and optional music (generated or free, no runtime network calls)
- [ ] Pause menu, help screen, mobile/touch layout and canvas scaling
- [ ] Save/load via localStorage with graceful fallback — `save`/`load_save` implemented in web layer

### Milestone 6: Polish and release

- [ ] Balance pass (target win rate roughly 50-70% for average play; seeded simulation tests; no single route strictly best)
- [ ] Fact-check landmark text, road names, distances, and hunting rules at a high level
- [ ] Bug sweep, zero clippy warnings, console-error check
- [ ] Final score screen and local high score
- [ ] Release build (`trunk build --release`) and static hosting instructions

## Next Steps

1. Run `trunk serve`, verify the title screen and each M1 screen render in the browser (desktop + mobile layout), fix any rendering issues found.
2. Wire `Game::restart` into the title screen (Play Again) — closes the last M1 box.
3. Begin Milestone 2: event system on top of the existing `events.json` loader + seeded RNG (15+ events first).
4. M2–M6 placeholder APIs are already declared (dead-code annotated with "first consumer: M#") — implement against them.

## Known Issues

| # | Description | Severity | Status |
|---|-------------|----------|--------|
| 1 | Title/HUD rendering not yet verified in a real browser (code compiles; no `trunk serve` session run) | Unknown | Open |
| 2 | Inverted assertion in `party::tests::health_zero_means_left_trip_not_death` (`apply_health` returning `true` when a member leaves was negated) | Low | **Fixed 2026-11-25** (caught by `cargo test`) |

### Unverified (tooling or facts)

- [ ] In-browser rendering of all M1 screens (needs `trunk serve` + a browser session)
- [ ] All map road names/mileages are still the starting sketch — fact-check in M6
- [ ] web-sys 0.3.107 API signatures were verified against compiler feedback, not docs

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

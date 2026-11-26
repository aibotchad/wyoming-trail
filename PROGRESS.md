# Wyoming Trail — Progress

## Vision

A short, replayable Oregon Trail-style road trip across modern Wyoming. Players outfit a pickup truck in Cheyenne, then choose their own roads across a network of real towns and landmarks. They manage fuel, money, food, and a four-person party, hunt for food, meet memorable NPCs, survive wind closures, wildlife, and breakdowns, and learn real Wyoming history on the way to Yellowstone. Fun comes from route planning, risky choices, resource tension, and funny/dramatic encounters. Different routes should make different runs.

## Tech & Structure

- Language: Rust (stable), compiled to WebAssembly (wasm32-unknown-unknown)
- Build/serve: Trunk (`trunk serve`, `trunk build --release`)
- Key crates: wasm-bindgen, web-sys, js-sys, serde, serde_json, rand (seedable, getrandom "js" feature)
- Rendering: Canvas 2D via web-sys; keyboard + mouse/touch input
- How to run: `trunk serve`, then open the printed localhost URL
- How to test: `cargo test` (core logic), `cargo clippy`, `cargo fmt --check`
- Folder layout (update as it changes):

```text
Cargo.toml
Trunk.toml
index.html
assets/data/   map.json, events.json, items.json, npcs.json, dialogue.json
src/lib.rs     (wasm entry point)
src/core/      state, party, truck, travel, map, events, shop, hunting,
               npc, dialogue, rng, config, scoring                (NO web deps)
src/web/       render, input, audio, storage                      (thin)
tests/         integration tests for core
```

## Core Design

- Player goal: Reach Yellowstone (any entrance) with at least one party member still on the trip.
- Core mechanics: Graph-based map with road choice at junctions; daily driving loop with pace (hours driven) and meals; fuel/money/food/ammo management; random events; road closures and rerouting; licensed hunting minigame; NPC encounters and dialogue.
- Controls: Mouse/touch buttons and number/arrow/Enter keys for menus.
- Win: Arrive at Yellowstone (South via Grand Teton, East via Cody) with 1+ party members. Score from party health and morale, money, truck condition, landmarks visited, NPCs helped, and days taken.
- Lose: Truck stranded with no fuel/money and no way to get help, or every party member has left the trip.
- Party members at 0 health: taken to a hospital and leave the trip (family-friendly, no deaths).
- Scope limits (what this is NOT): No multiplayer, no free-roam driving, no real-time action beyond the hunting minigame, no 3D, no graphic content.

## Map Graph (data-driven; edit via `assets/data/map.json`)

Requirement: at least 3 meaningfully different routes from Cheyenne to a Yellowstone entrance, with backtracking allowed.

Starting sketch of roads (verify every road name and mileage; mark `// TODO verify`):

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

- Milestone: 1 of 6
- State: Not started
- Last session summary: -

## Milestones

### Milestone 1: Playable core with a branching map (current)

- [ ] Cargo project, Trunk config, core/web module split
- [ ] Canvas renders a title screen in the browser; one passing unit test
- [ ] Screen state machine (enum) and game loop
- [ ] Party creation (4 names) and party/truck/resource data model in core
- [ ] Map graph data model and loader (map.json) with a small test map (at least 2 routes)
- [ ] Road Choice screen at junctions (road name, distance, terrain, services ahead)
- [ ] Travel screen: day counter, miles progress, pace and meal selectors
- [ ] Daily update: fuel burned, food eaten, fatigue and health change (unit tested)
- [ ] Arrive at node and show landmark/town screen (placeholder text)
- [ ] Win at Yellowstone and game over conditions
- [ ] Restart without reloading

### Milestone 2: Events and survival

- [ ] Central config module for all balance numbers
- [ ] Event system (data-driven, weighted, condition-aware; seeded RNG; unit tested)
- [ ] 15+ events: wind closure, blizzard, wildlife on road, flat tire, overheating, dead battery, cell dead zone, speeding ticket, construction, etc.
- [ ] Road closures remove roads from choices until reopened; reroute and backtrack logic
- [ ] Sickness/injury and recovery; first-aid and medicine items
- [ ] Truck condition (tires, battery, engine) affects travel and breakdown odds
- [ ] Weather, season, and terrain per road

### Milestone 3: Stops, fuel, and hunting

- [ ] Cheyenne outfitting screen (starting money, truck choice, supplies)
- [ ] Gas station: buy fuel, prices vary by location; services-gap warnings
- [ ] Diner, motel/campground, mechanic, outfitter at appropriate stops
- [ ] Hunting license and ammo purchase at outfitters
- [ ] Hunting minigame (non-graphic timing/aim), legal game by season and location
- [ ] Hunting restrictions (no hunting in national parks or towns), food carry limits, spoilage without cooler/ice
- [ ] Game warden fine logic for violations (unit tested)
- [ ] History/fun-fact screens for every landmark

### Milestone 4: NPCs and expanded world

- [ ] NPC and dialogue data model (nodes, choices, conditions, effects, flags) with unit tests
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
- [ ] Save/load via localStorage with graceful fallback

### Milestone 6: Polish and release

- [ ] Balance pass (target win rate roughly 50-70% for average play; seeded simulation tests; no single route strictly best)
- [ ] Fact-check landmark text, road names, distances, and hunting rules at a high level
- [ ] Bug sweep, zero clippy warnings, console-error check
- [ ] Final score screen and local high score
- [ ] Release build (`trunk build --release`) and static hosting instructions

## Next Steps

1. Create the Cargo project, Trunk config, and module split.
2. Get a title screen rendering on the canvas with one passing unit test.
3. Build the core party/truck/resource model and a small branching test map.

## Known Issues

| # | Description | Severity | Status |
|---|-------------|----------|--------|
| - | None yet | - | - |

### Unverified (tooling or facts)

- [ ] Nothing logged yet. Record anything that could not be tested or fact-checked here.

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
| 1 | - | - |

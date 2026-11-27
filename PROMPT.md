# Wyoming Trail — Game Prompt

## Role

You are a game developer building Wyoming Trail, a text-and-sprite travel RPG in the style of The Oregon Trail, set in modern day, where the player drives a pickup truck across Wyoming along roads of their own choosing. It is written in Rust, compiled to WebAssembly, and runs in the browser.

## Game Summary

The player leads a small road-trip party (the driver plus three friends or family) from Lander to wherever their job's mission takes them (since 2026-11-26 each profession has its own goal). The map is a network of real Wyoming towns, landmarks, and highways, not a single line. At every junction the player chooses which road to take, weighing distance, fuel range, terrain, weather, services, and scenery. Along the way the party drives, burns fuel, eats, hunts, and meets NPCs (truckers, rangers, ranchers, hitchhikers, mechanics, game wardens, locals). Random events strike: wind closures, blizzards, wildlife, breakdowns, long gaps between gas stations. The player wins by reaching Yellowstone (by any entrance) with at least one party member still on the trip. The trip fails if the truck is stranded with no money or fuel and no way to get help, or the whole party has left the trip.

## Tone & Style

- Retro Oregon Trail feel: menu-driven screens, simple blocky/pixel visuals, short punchy event text, a little dry humor. The setting is modern (gas stations, motels, cell phones, highways, GPS, diners, CB radio).
- Educational flavor: each landmark has a short, accurate fun fact (2-4 sentences). Verify facts if unsure. If you cannot verify a detail, leave it out rather than invent it.
- Adult comedy: redneck and meth jokes (owner request, 2026-11-26). The title screen opens with "If you think this is full of Redneck and meth jokes then you'd be correct". No drug instructions, no jokes about real private people, and every joke is labelled as made up, apart from the true facts. Party members who reach 0 health are taken to a hospital and leave the trip (no deaths). Hunting is shown as a simple, non-graphic timing game. Do not glorify reckless driving.

## Owner changes (2026-11-26)

These override anything below: per-profession missions; redneck jobs poach and hippie jobs forage in a field minigame (no licence); animated parallax drive view; bundled CC0 art and a pixel font.

## Tech Stack (required)

- Rust (stable) targeting wasm32-unknown-unknown
- wasm-bindgen + web-sys + js-sys for browser bindings
- Trunk for building and serving (`trunk serve`, `trunk build --release`)
- serde + serde_json for data files embedded with `include_str!`
- rand with a seedable PRNG (enable the getrandom "js" feature for wasm) so runs are reproducible in tests
- Rendering: Canvas 2D via web-sys, with keyboard and mouse/touch input. Keep hand-written JavaScript to the bare minimum.
- No heavy game engines (no Bevy). Add a dependency only if it clearly saves work, and log it under Decisions.

## Architecture Rules

- Data-oriented programming (DOP) is the guiding style: model game content as simple, serializable data (plain structs and tables) that deterministic, pure rule/transition functions operate on — rather than encoding behavior in scattered code and deep object hierarchies. Prioritize clean architecture, clear separation of concerns (data / rules / state / presentation), and extensibility (adding a town, road, event, or NPC should be data-only); avoid over-engineering.
- Separate pure game logic from the web layer.
  - `src/core/` holds all game rules: party, truck, resources, travel, map graph, events, shop, hunting, NPCs and dialogue, scoring, RNG, config. No web-sys/js-sys here. Fully unit-testable with `cargo test`.
  - `src/web/` holds rendering, input, timers, audio, and localStorage. Keep it thin.
  - `src/lib.rs` is the wasm entry point.
- Data-driven content in `assets/data/` (JSON, loaded with `include_str!`): map.json (nodes and roads), events.json, items.json, npcs.json, dialogue.json. Adding a town, road, event, or NPC should require editing data only.
- Config: all balance numbers (fuel prices, MPG, tank size, event odds, hunting yields, fatigue rates) live in one config module. No magic numbers.
- State machine: model screens as a Rust enum (Title, Outfit, Travel, RoadChoice, Event, Encounter/Dialogue, Town, RoadClosure, Hunt, GameOver, Win). Game state is one serializable struct (save/load).
- Use `Result` and proper error handling. No `unwrap()` in game logic outside tests. Keep `cargo clippy` warnings at zero and code formatted with `cargo fmt`.

## Core Systems (build in this order)

1. Game loop and screen manager
2. Party: 4 members with name, health, fatigue, mood, status (healthy / sick / injured / left trip)
3. Truck: fuel, tank size, MPG, condition, tires, battery, spare parts. Truck type chosen at start (older pickup, standard pickup, diesel) with different stats.
4. Resources: money, food, water, ammo, hunting license, medicine/first-aid, spare tire, emergency kit, cooler/ice (keeps meat from spoiling)
5. Map graph and road choice (see below)
6. Travel: day counter, season/date, pace (relaxed / standard / rushed, by hours driven per day), meals (cheap / normal / diner), weather, road conditions
7. Random events: weighted, condition-aware (season, terrain, weather, road, truck condition)
8. Towns/stops: gas station, diner, motel/campground, mechanic, outfitter, locals, history
9. Road closures and detours: wind and snow closures, construction. Choices: wait it out, reroute via another road, or push on at risk.
10. Hunting minigame (see below)
11. NPC encounters and dialogue (see below)
12. Win/lose and final score: survivors, morale, money, truck condition, days taken, landmarks visited, NPCs helped

## Map Graph and Road Choice (non-linear travel)

- Model the map as a graph: nodes are towns, landmarks, junctions, and park entrances; edges are real roads.
- Each node has: id, name, type, services (fuel, food, motel, mechanic, outfitter), history text, hunting allowed or not, and optional NPC spawns.
- Each road (edge) has: from, to, highway name, real driving distance, terrain type, speed/pace modifier, services-gap warning, seasonal risk (wind, snow, construction, wildlife), scenery rating, event table id, and NPC encounter table id.
- At every node with more than one outgoing road, show a Road Choice screen: road name, distance, terrain, last known conditions, services ahead, and a hint about hazards or scenery. Info can be incomplete or wrong until a local NPC, CB radio, or phone/GPS update improves it.
- The player may backtrack (costs fuel, time, and morale). Dead ends and long gaps exist, so planning fuel range is a core tension.
- There must be at least 3 meaningfully different routes from Lander to a Yellowstone entrance. Different routes should offer different landmarks, NPCs, hazards, and costs. Reaching Yellowstone through different entrances (South via Grand Teton, East via Cody) is a valid win.
- Closed roads are removed from the choices until they reopen, which forces rerouting.

## Hunting Minigame (required)

- Short timing/aim minigame (simple crosshair or timing bar). Targets are legal game such as pronghorn (antelope), deer, elk, and rabbits or grouse for small game. Presentation is non-graphic.
- Requires ammo and a hunting license (buy at an outfitter or sporting goods stop). Show a clear in-game reminder that rules and seasons apply.
- Restrictions (data-driven): no hunting in national parks (Grand Teton, Yellowstone) or inside towns; some roads or stops allow it, some do not; seasons affect which animals appear. Violations risk a game warden NPC fine. Verify real-world rules at a high level and keep it light.
- Food yield is limited by what the truck can carry. Without a cooler or ice, extra meat spoils. A hunt costs a half day of travel time.
- Results: food added to supplies, ammo spent, occasional mishaps via events.

## NPC System (required)

Two kinds of NPCs, both data-driven in `npcs.json` and `dialogue.json`:

1. Town NPCs at fixed locations (diner waitress, motel owner, mechanic, outfitter, local historian, rancher). They sell, repair, share local history, and give road reports (revealing hazards, closures, gas prices, or shortcuts).
2. Roadside encounters that can appear on specific roads, terrains, seasons, or weather. Examples: trucker on the CB radio, park ranger, state trooper, game warden, hitchhiker, stranded tourists, tow-truck driver, cowboy or rancher moving cattle, wandering storyteller, fellow road-tripper heading the same way, a lost dog's owner.

- Dialogue: simple branching dialogue (nodes with choices, conditions, and effects). Choices can cost or give money, items, food, fuel, morale, health, or information.
- Effects: items, money, hints that reveal road conditions or unlock hidden roads and landmarks, temporary party members (a hitchhiker who joins for a few stops and eats food but boosts morale or helps repair), and short side quests (deliver a package to a named town, return a dog, give someone a ride to a landmark).
- Memory: NPCs track flags (met, helped, refused). Some recurring NPCs (like the trucker on the CB) can reappear later and react to earlier choices. Helping NPCs adds to the final score.
- Never block progress with a mandatory NPC. Encounters are opportunities and risks, not gates.
- Seed encounters with the RNG so they are reproducible in tests. Write unit tests for dialogue conditions, effects, and encounter selection.

## Wyoming-Specific Flavor to Include

- Fuel anxiety: long stretches without services, low-fuel warnings, higher prices at remote stations
- Hazards: high winds (closures), sudden blizzards, hail, black ice, wildfire smoke, mud, deer/pronghorn on the road, bison jams, flat tires, overheating, dead battery, cell dead zones, altitude sickness, speeding tickets, construction
- Terrain types: high plains, sagebrush basin, river valley, badlands, mountain pass, geyser basin
- Seasons: player picks a start month. Summer brings tourists, construction, and fire smoke. Winter brings closures, ice, and cold.

## Starting Map Sketch (store in `assets/data/map.json`; verify everything)

- **Nodes (start):** Cheyenne, Laramie, Medicine Bow, Rawlins, Rock Springs, Wheatland/Douglas, Fort Laramie, Register Cliff (Guernsey), Casper, Independence Rock, Devil's Gate, Muddy Gap, Split Rock, South Pass, Lander, Shoshoni/Riverton, Thermopolis, Dubois, Pinedale, Buffalo, Sheridan, Gillette, Devils Tower, Cody, Jackson, Grand Teton National Park, Yellowstone (South Entrance, East Entrance).
- **Roads (start; check each against a trustworthy source):** I-80 Cheyenne-Laramie-Rawlins-Rock Springs; US-30/287 Laramie-Medicine Bow; WY-487 Medicine Bow-Casper; I-25 Cheyenne-Douglas-Casper-Buffalo; US-85/US-26 Cheyenne-Fort Laramie-Guernsey; WY-220 Casper-Independence Rock-Devil's Gate-Muddy Gap-Rawlins; US-287 Muddy Gap-Split Rock-Lander; WY-28 Lander-South Pass-Farson; US-20/26 Casper-Shoshoni-Thermopolis; Lander-Riverton-Shoshoni; WY-120 Thermopolis-Cody; US-14/16/20 Cody-Yellowstone East Entrance; US-16 Buffalo-Ten Sleep-Cody; US-14 Sheridan-Cody (Bighorn Mountains); I-90 Buffalo-Sheridan and Buffalo-Gillette; US-14 Gillette-Devils Tower; US-287/26 Lander-Dubois-Togwotee Pass-Jackson; US-191 Rock Springs-Pinedale-Jackson; US-26/89/191 Jackson-Grand Teton-Yellowstone South Entrance.
- Mark any unverified road or mileage with `// TODO verify` and list it in Known Issues.

## How You Work (follow every session)

1. START by reading `PROGRESS.md` in full. It is the source of truth for what is done, broken, and next. Do not rely on memory of earlier conversations.
2. Pick the first unchecked task in the current milestone. Work on ONE task at a time.
3. Keep the project building at all times. Never end a session with a broken build.
4. Verify your work before marking anything done:
   - `cargo fmt` and `cargo clippy` clean
   - `cargo test` passes (write unit tests for new game logic, including map pathing, road-choice availability, encounter selection, and hunting rules)
   - `trunk build` succeeds
   - if you can run the game in a browser, exercise the feature and check the console for errors
   - if a tool is unavailable in your environment, say so plainly in `PROGRESS.md` and list what remains unverified. Never claim something was tested when it was not.
5. END every session by updating `PROGRESS.md`: check off tasks, update Current Status and Next Steps, log bugs under Known Issues, record decisions, and add a Session Log entry.

## Rules

- Simplest thing that works first, then iterate. No speculative features. Put new ideas in the Backlog.
- Bundle freely licensed photos (Wikimedia Commons) and public-domain map tiles, with credits; generated or no audio until told otherwise. No external network calls at runtime.
- Support keyboard and mouse/touch. The canvas must scale to desktop and mobile widths.
- Persist saves with localStorage via web-sys; handle failure gracefully.
- Comment non-obvious logic and name things clearly.
- If a requirement is ambiguous, make a reasonable assumption, log it under Decisions, and keep going. Only stop to ask if the answer would change the architecture.

## Definition of Done (per task)

Implemented, formatted, clippy-clean, unit-tested where it is game logic, builds to wasm, manually checked if possible, and PROGRESS.md updated.

## First Session

If PROGRESS.md shows no completed tasks, set up the Cargo project and Trunk config, the core and web module split, a canvas that renders a title screen in the browser, and one passing unit test. Then begin Milestone 1.

# Wyoming Trail

> If you think this is full of Redneck and meth jokes then you'd be correct.

A travel RPG in the style of The Oregon Trail, set in modern Wyoming. Pick one of eight jobs (a ranch hand rides a horse, thanks to his DUIs; a Trustifarian collects vintage things for a Jackson Hole condo), each with its own mission, buy a vehicle (including a 2012 Prius with a faded Hillary sticker) and supplies, then set out from Lander on real highways without running out of gas, food or party members. Poach or forage when the grub runs low (in winter the foragers jump into dumpsters; poachers risk big warden fines), sell the surplus, take odd jobs, and mind the weather, the GPS, the hitchhikers and your mission deadline. Rust compiled to WebAssembly, built with Trunk, runs in the browser.

## Run it

```bash
./start.sh                # this computer and your local network, port 8080
./start.sh --open         # the same, and open a browser
trunk serve --open        # this computer only
```

`start.sh` prints both addresses. Anyone on your network can open the game while it runs. It rebuilds when files change but never reloads an open game; refresh the page to update, or run `RELOAD=1 ./start.sh` while developing. (`trunk serve` on its own does reload.)

Needs Rust with the `wasm32-unknown-unknown` target and `cargo install trunk --locked`.

Controls: click or tap, drag or flick to scroll, arrows and Enter, number keys 1-9, `M` for the map, `S` (or the STATS button in the bottom bar) for graphs, `P` (or the MENU button) to pause and rest, `Esc` to go back. Gathering waits for a tap or Space before the clock starts; then arrows or WASD (or hold a finger) to move, Space or tap to shoot or pick. On a touch screen a second finger can shoot while the first keeps walking.

Finished missions go on a **Top ten** list (title screen and finish screen). It is kept by each browser, so every phone or computer has its own.

**Drive** keeps going day after day, with the map marker moving along the highway and the scenery changing with the country, until you arrive, stop it, or something comes up. A town further off can be headed for from its map card. Missions that end in a drum circle let you drum it: tap as the ring closes.

## Balance

`cargo test --release balance_report -- --ignored --nocapture` has a simulated player play every job hundreds of times and prints win rates, scores and fines; `outing_report` compares one poaching or foraging round by job and place. `BALANCE="job:money:days" cargo test ... balance_report` tries numbers without editing the data. A normal `cargo test` fails if the jobs drift out of step.

## Changing the game

Content lives in `assets/data/*.json`: add a town, vehicle, job, item, event, odd job, weather type, plant or animal there, then run `cargo test`, which checks that every name the files use exists and plays random games to catch crashes and dead ends (`STRESS_RUNS=5000 cargo test --release random_play` for a longer soak). See the data-file table in [PROGRESS.md](PROGRESS.md).

## Art

`python3 tools/make_sprites.py` (needs Pillow) rebuilds the sprite sheet, its atlas (`assets/data/sprites.json`) and the parallax layers from `assets/vendor`.

## Docs

| File | What it is |
|------|-----------|
| [PROMPT.md](PROMPT.md) | The full game design prompt: requirements, systems, workflow rules |
| [PROGRESS.md](PROGRESS.md) | Progress tracker: milestones, status, decisions, known issues |
| [CREDITS.md](CREDITS.md) | Photo, map and text sources and licenses |

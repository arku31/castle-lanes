# Castle Lanes

Castle Lanes is a native desktop Rust/Bevy MVP inspired by the builder-autobattler shape of classic custom RTS maps, using original names and original 2D isometric-ish art.

## Run

With [Task](https://taskfile.dev/), common commands are available through `Taskfile.yml`:

```bash
task run-server
task run-client
task run-demo-client
task run-bot
task verify
```

Start the authoritative dedicated server:

```bash
cargo run --bin castle_lanes_server -- 127.0.0.1:4000
```

Start two clients in separate terminals:

```bash
cargo run --bin castle_lanes_client -- --name Alice --server 127.0.0.1:4000
cargo run --bin castle_lanes_client -- --name Bryn --server 127.0.0.1:4000
```

Or run one visible client against a headless scripted peer:

```bash
cargo run --bin castle_lanes_client -- --name Alice --server 127.0.0.1:4000 --race grove --auto-ready --auto-build-demo
cargo run --bin castle_lanes_bot -- --name Bryn --server 127.0.0.1:4000 --race ember
```

## Controls

- `Enter`: join or reconnect to the server.
- `Q`: choose Vanguard.
- `W`: choose Grove.
- `E`: choose Ember.
- `Space`: ready up in the lobby.
- `1`, `2`, `3`: select one of your race's three buildings.
- Click the race/build buttons in the bottom command frame for mouse-first play.
- Left click your highlighted build grid to place the selected building.
- Left click a unit, building, castle, or build cell to inspect it in the command frame.
- Arrow keys: pan the camera.
- `+`, `-`: zoom the camera.
- `Home`: reset the camera.
- `R`: vote for rematch after game over.
- `--race vanguard|grove|ember`: client/bot flag for demo/test race selection.
- `--auto-ready`: client flag for demo/test runs that readies after joining.
- `--auto-build-demo`: client flag for demo/test runs that places your first race building after match start.

Players must choose a race before readying. The server rejects ready commands until a race is selected.

## Balance Config

Tune races, castle HP, building costs, spawn timers, income bonuses, and unit stats in:

```text
config/balance.json
```

The dedicated server loads this file at startup and includes the active balance in replicated snapshots so clients display the server's names/costs.

## Art

Generated original art lives in:

```text
assets/art/
```

The client uses `assets/art/isometric_battlefield.png` as the main map backdrop and code-drawn RTS-style panels/buttons for the current UI pass.
Unit sprites live in `assets/art/units/`; the generated atlas source is kept alongside the cropped transparent PNGs for future edits.
Building command icons live in `assets/art/buildings/` and are used by the build toolbar.

See [docs/assets.md](docs/assets.md) for the asset creation pipeline, prompt shape, crop/key process, and client integration notes.

## MVP Rules

- 1v1 only: Left vs Right.
- Races: Vanguard, Grove, Ember.
- Each race has unique castle HP and three unique buildings.
- Starting gold, base income, race HP, buildings, and unit stats are configured in `config/balance.json`.
- Units have attack and armor types. The current table is Warcraft-3-inspired: Magic is strong into Heavy and weak into Light, Pierce is strong into Light and weak into Heavy, and castles use Fortified armor.
- Units also expose movement speed, attack speed, attack range, and attack mode (`Melee` or `Ranged`) through `config/balance.json`; the client shows these in the inspect/build UI.
- Unit `damage` is a midpoint, not a fixed number. `damage_variance: 0.10` means a unit with `50` damage rolls from `45` to `55` before attack/armor multipliers are applied.
- The client renders inferred combat readability effects from server snapshots: floating high-contrast damage numbers, attack-type streaks, and hit bursts.
- Sudden death starts at 3:00. Castles take pressure damage based on the enemy's buildings and units, which helps matches resolve instead of stalling forever.

The server owns all gameplay state. Clients send only join, ready, placement, and rematch intents.

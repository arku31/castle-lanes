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
- Choose a race in the opening modal, then click `Ready`.
- `1`-`8`: select one of your race's buildings.
- Click the 3x3 command card in the bottom command frame for mouse-first play.
- `Esc` or the `X` command-card slot cancels building placement.
- Left click your highlighted build grid to place the selected building while placement mode is active.
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

Tune races, castle HP, economy timing/interest, building costs, spawn timers, income bonuses, unit stats, and unit bounties in:

```text
config/balance.json
```

The dedicated server loads this file at startup and includes the active balance in replicated snapshots so clients display the server's names/costs.

## Art

Generated original art lives in:

```text
assets/art/
```

The client uses a procedural neutral grass battlefield with subtle animated blades and code-drawn RTS-style panels/buttons for the current UI pass.
Unit sprites live in `assets/art/units/`; the generated atlas source is kept alongside the cropped transparent PNGs for future edits.
Building command icons live in `assets/art/buildings/` and are used by the build toolbar.

See [docs/assets.md](docs/assets.md) for the asset creation pipeline, prompt shape, crop/key process, and client integration notes.

## MVP Rules

- 1v1 only: Left vs Right.
- Races: Vanguard, Grove, Ember.
- Each race has unique castle HP and eight unique buildings: seven unit producers plus one economy building.
- Starting gold, base income, income interval, interest rate, race HP, building HP, buildings, and unit stats are configured in `config/balance.json`.
- Economy ticks every configured interval. The default is 10 gold every 10 seconds plus floored interest from unused banked gold, currently 4%.
- The battlefield has two lanes: Top and Bottom. Buildings are placed into lane-specific Front or Back zones.
- Unit-producing buildings spawn units from their own lane and grid position, so top buildings feed the top lane and bottom buildings feed the bottom lane.
- Units prioritize enemy units first. Lanes are separate in the field, but connect inside castle junction zones so defenders can attack cross-lane enemies near a castle. After units, attackers target same-lane enemy Front buildings, then the enemy castle. Back buildings do not tank for the castle.
- Units have attack and armor types. The current table is Warcraft-3-inspired: Magic is strong into Heavy and weak into Light, Pierce is strong into Light and weak into Heavy, and castles use Fortified armor.
- Units also expose movement speed, attack speed, attack range, and attack mode (`Melee` or `Ranged`) through `config/balance.json`; the client shows these in the inspect/build UI.
- Each unit has a configured bounty. Killing a unit awards that gold to the killer's team.
- Unit `damage` is a midpoint, not a fixed number. `damage_variance: 0.10` means a unit with `50` damage rolls from `45` to `55` before attack/armor multipliers are applied.
- The client renders inferred combat readability effects from server snapshots: floating high-contrast damage numbers, attack-type streaks, and hit bursts.
- Sudden death starts at 3:00. Castles take pressure damage based on the enemy's buildings and units, which helps matches resolve instead of stalling forever.

The server owns all gameplay state. Clients send only join, ready, placement, and rematch intents.

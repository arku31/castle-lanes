# Architecture map — where things live

Binary layout (`Cargo.toml`): `castle_lanes_server` (authoritative),
`castle_lanes_client` (Bevy, `src/bin/client/` modules), `castle_lanes_bot`
(headless peer). Shared library `castle_lanes` (sim + net + record) is
engine-free so headless tooling compiles fast.

## Authoritative simulation — `src/sim.rs`

- Per-player state: `economies: Vec<Economy>`, `castles: Vec<Castle>`
  (one castle + one economy per seat; `Castle.owner` is a `PlayerId`).
- `Unit.owner` / `Building.owner` are `PlayerId`; sides resolve through
  `side_of` / the borrow-free `lookup_side` table inside hot loops.
- Fixed 30 Hz `GameSim::tick(dt)`; deterministic per seed
  (`GameSim::with_seed`, ordered-targeting with id tie-breaks — two runs with
  the same seed + command log are byte-identical; tested).
- Combat: typed damage matrix (`attack_multiplier`), abilities
  (`AbilityConfig`: heal/splash/slow/regeneration/berserk) read from
  `config/balance.json`, branch upgrades (`upgrade_building`, BuildingConfig
  `.upgrades` / `.upgraded_from`), sell at 70%.
- Victory: every castle of a side dead; adjudication score on double KO.
- Sudden death (480 s): regen off, pressure = board × (1 + ramp·min) + flat
  escalation, lands on the side's first surviving castle.

## Netcode — `src/net.rs` + `src/bin/server.rs`

- UDP JSON with a 1-byte format tag; tag 1 = bincode (default), tag 0 =
  tagged JSON (`--legacy-json`), untagged JSON auto-decodes (legacy peers).
  Protocol version gate: `PROTOCOL_VERSION` (currently 9).
- Server ticks rooms at 30 Hz, broadcasts per-client delta snapshots at
  10 Hz, chunked to fit safe UDP payloads; baseline resync on phase change.
- Fog is server-side: `filter_snapshot_for_viewer` emits only entities the
  viewer's side sees (scouted enemy buildings are re-sent stale for
  silhouettes); the client's fog memory is presentation only.
- Reconnect grace: disconnect during a match reserves the seat for 90 s
  (`ReconnectDeadlines`); rejoin by name restores it.
- Commands are seq-acked (`ServerPacket::Ack`); clients retry placements for
  1.5 s, server dedupes at-most-once.

## Client — `src/bin/client/` modules

- `main.rs` — app setup, shared types (SnapshotState, SceneRegistry,
  ClientSettings, UnitFrameSets, FontAssets, UiOverlays…).
- `net.rs` — packet pump + SnapshotState accessors.
- `scene.rs` — persistent entity registry: units interpolate between the
  last two snapshots (RenderInterp) every frame; statics rebuild only on a
  structure hash change; fog tiles mutate color in place; selection is one
  overlay entity.
- `ui.rs` — HUD, lobby browser, command card, inspect/tooltip, overlays
  (help `H`, settings `O`), minimap, hint ticker.
- `input.rs` — menu/build/placement/mouse picking/camera; `U`/`I` upgrade,
  `Delete` sell, `Ctrl+Q` concede.
- `vfx.rs` — combat inference from snapshot diffs (hits, deaths, castle
  alarms, bounty coins) + the Vanguard ranged bolt pilot.
- `audio.rs` — SfxQueue with per-sound cooldowns; `ClientSettings` volume.

## Match recording — `src/record.rs`

Server writes `recordings/game{N}_{unix}.json`: seed + players + intent log
(with sim ticks) + kill feed + 1 Hz timeline + result. Deterministic seeds
make the file a replay.

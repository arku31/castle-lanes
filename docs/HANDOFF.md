# Handoff: Castle Lanes — 2026-09-06 (evening)

## Where the project stands (short)

Castle Fight-inspired multiplayer builder-autobattler (Rust/Bevy 0.18, authoritative UDP
server, deterministic sim). Governing roadmap is `plan.md` (Phases 0–4, all substantially
implemented). Server, bot, client, tests (`cargo test`, 49 unit + settings test) and
`cargo build --bins` are all green.

New in this session (commit "UI overhaul: verified 1600x900"): the HUD overflow is fixed
and visually verified, screenshot batch refreshed under `docs/screenshots/`, README
images refreshed.

## What was actually wrong with the UI (Blocker 2 — RESOLVED)

The overflow was NOT the layout constants. `redraw_game_ui` (despawns + respawns every
UI entity each frame) and `pin_ui_to_camera` (re-pins them to screen space) had NO
ordering edge — the inner system tuples in `main()` were not `.chain()`ed — so every UI
entity lived exactly one frame and was never pinned. UI rendered at raw pin coordinates
in WORLD space; with the camera parked at the player's base (~-850 x), the entire HUD
was shifted right ~850px and clipped. It only looked correct while the camera sat at the
origin (pre-join lobby).

Fixes now in:
- `main()` + `run_replay()`: systems chained (`update_ui_layout` first; render tuple
  `.chain()`ed so `redraw_game_ui` → `pin_ui_to_camera` is ordered and Bevy inserts the
  command sync point). Watch out: Bevy caps function systems at 16 params —
  `redraw_game_ui` is AT the limit; remove a param before adding one.
- New `UiLayout` resource (window half-extents, updated by `update_ui_layout` each
  frame). Top bar, hint strip, bottom console, minimap, command grid, tooltips, context
  bay, team scoreboard, and the race-popup dim layer all derive from it
  (`UiLayout::top_bar_y/panel_y/command_grid_origin/minimap_center/...`). Hit-testing in
  `input.rs` uses the same resource, so clicks always match what's drawn.
- Team scoreboard now hangs below the hint strip (it used to draw off the top edge for
  >2 players). Race popup dim now covers the full window. Race-card blurbs pre-wrapped
  (they used to overlap neighboring cards).
- Verified live at 1600×900 AND at a 1470×895 window (macOS resized it) — everything
  fits with no scrolling.

## Client self-exit (old Blocker 1 — CHARACTERIZED, not fixed)

Not the user closing it (they did once); it also happens unattended. Caught with a
`wait`-supervisor run:

```
INFO bevy_window::system: No windows are open, exiting
WARN bevy_winit::state: Skipped event Destroyed for unknown winit Window Id
exit code 0
```

The WINDOW entity is destroyed at the winit/macOS layer first; Bevy then notices zero
windows and exits cleanly. No panic, no crash report (`crashes/`, DiagnosticReports),
nothing in the unified log. Observations: multiple clients died 60–130s into live
matches; one vanguard client survived 17+ minutes through a full 8-minute match AND
game over; lobby-only clients survived indefinitely. Suspect winit/macOS 26 (darwin 25)
window teardown under load, not project code — the client has no exit paths besides
replay-Escape. If it matters, run the client focused/foreground for demos and consider
an upstream bevy_winit investigation.

## Workflow gotchas learned here (save hours)

- **cargo silently no-ops on this volume** (`/Volumes/Projects`): after editing sources,
  `cargo build` may say "Finished" WITHOUT recompiling. `touch` the edited files (or the
  whole `src/bin/client/`) before building. Verify with a `strings` grep for a new
  string literal, or by checking that eprintln debug output actually appears.
- **`pkill` from a sandboxed shell cannot signal processes launched from an
  unsandboxed one** (fails silently). Launch and kill test clients/bots from the same
  privilege context, and confirm with `pgrep` after killing.
- **Orchestration for a demo match**: start the SERVER fresh (the server keeps
  stale/rematch-cycling games alive, which otherwise grab the bot), then the CLIENT
  (its auto-flow creates "Name's Game"), wait ~6s, then the BOT — `bot.rs` joins
  `games.first()` or creates its own, which deadlocks if no game exists yet.
- **Fast balance matches are short** (~80–130s) and the server auto-rematches: several
  "lobby" screenshots were actually between-rounds reset frames. Capture in bursts and
  check the top bar says "Battle m:ss".

## Screenshot recipe (works)

Screen Recording is granted. Window id via Quartz CGWindowListCopyWindowInfo (owner
`castle_lanes_client`), then `screencapture -x -o -l<WID> out.png` (captures the full
window buffer even on multi-display; title bar included). New demo flags:
`--show-help` / `--show-settings` open the client with that overlay visible — added
because H/O keys are only handled in-game... actually now they also spawn in the
pre-game lobby (fixed this session), but the flags make captures deterministic.
Camera pans via System Events `key code 2` (D) held ~1.6s worked; `--camera-x` exists
in ClientOptions but is NOT parsed by `parse_args` yet (easy add if needed).

Current batch in `docs/screenshots/`: `battle.png` (mid-lane clash), `battle_armies.png`
(big Ember push), `siege.png` (castle under fire), `lobby.png` (game browser),
`help.png`, `settings.png`. README references all of them.

## After this commit (unchanged priorities)

1. **Commander powers** (user: KEEP): design + implement per plan.md — per-player
   strategic abilities (Rally / Entangle / Meteor style), protocol packet, sim effects,
   command-card UI, balance.json costs.
2. **Grove-vs-Vanguard balance iteration 3** via the harness (`docs/balance-workflow.md`,
   counter-aware bots; document in `docs/balance-report.md`).
3. Remaining plan.md tail: team-play client polish (4-lane minimap/camera partly done),
   spectator UI, distribution/packaging check, crash-reporting test.
4. Optional: decide whether to chase the winit self-exit upstream (see above).

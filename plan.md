# Castle Lanes — Path to a Really Good Game

Status: **Phase 0 COMPLETE** and **Phase 1 partially complete** (implemented & committed 2026-09-05).

Phase 0 (all items): determinism + `sim_balance` harness, besieged regen + sudden-death ramp (`docs/balance-report.md`), persistent/interpolated rendering, team accent colors, procedural audio, help overlay + hints + counter tooltips, lightyear removed.

Phase 1 progress:
- [x] Server-side fog/interest management — clients only receive entities their side can see (`position_revealed_to` in sim.rs, `filter_snapshot_for_viewer` in net.rs, per-session scouted-building sets on the server)
- [x] Reconnect grace — disconnect no longer resets a live match; 90s seat reservation with server-side expiry
- [x] Surrender — Ctrl+Q concedes; castle falls, normal victory path
- [x] Command acks — placements/sells carry a seq, retried 350ms/1.5s, server dedupes at-most-once
- [x] Sell/refund — Delete sells the selected own building for 70% (income bonus removed too)
- [x] Pacing hardening — sudden death disables regen and adds flat escalation (8/min); passive
      one-building stalls that ran 20+ min now resolve (~8 min verified through the live server);
      demo bot builds continuously like a player
- [x] Match recording (item 7) — src/record.rs; JSON per match under
      recordings/ (seed + intent log + kills + 1 Hz timeline + result); verified
      live through the server
- [x] CI (item 9) — .github/workflows/ci.yml: fmt, check, serial tests,
      release sim_perf + sim_balance smokes on push/PR
- [ ] Binary encoding (item 4)
- [ ] Client module refactor (item 8)
- [ ] Settings panel (item 10)

Screenshots for the README remain blocked: ZCode needs Screen Recording permission (System Settings → Privacy & Security → Screen Recording → grant to ZCode Computer Use, then restart ZCode).
Original baseline: commit `a79f462` + working tree (landed as `469d93d`).

---

## 1. What this game is

Castle Lanes is a lane builder-autobattler in the shape of classic WC3 custom maps (Castle Fight lineage), with original races/art. Currently 1v1; **committed direction (user-confirmed): support any even player count — 1v1, 2v2, 3v3, 4v4…** — teams of equal size on opposing sides.

- Authoritative dedicated server (30 Hz sim, 10 Hz delta snapshots over UDP/JSON).
- Clients send intents only (join / race / ready / place building / rematch).
- 2 lanes × front/back build zones, 3 races × 8 buildings, 21 unit types, WC3-style attack/armor matrix, income + interest + bounty economy, sudden death at 10:00.

The genre promise is: **build decisions between waves, watch battles resolve, out-econ and out-tempo your opponent.** Everything in this plan serves that promise.

### 1.1 What a good Castle Fight copy actually is (design north star)

The fun is not the units — it is this loop, which the MVP already implements and the plan sharpens:

1. **Read the enemy's build** (their buildings and waves are visible information).
2. **Counter-build or out-tempo them** (typed damage/armor matrix + unit roles + econ-vs-army gold tension).
3. **Watch waves accumulate** — surviving units merge with the next spawn, so battles snowball into the "big wave" drama the genre lives on. The sim already does this (units persist across spawns); pacing work must *preserve* it.
4. **Get a route back** after a bad call — bounties reward aggression into a losing board; sell/refund enables pivots; regen/sudden death prevents stalemates but shouldn't erase comebacks.
5. **Decide under uncertainty**, not solve a spreadsheet.

Litmus test (adopted as the bar for C2): *two skilled players watching the same battlefield should be able to disagree about the best next building and both have defensible reasoning.* If one purchase is always mathematically obvious, there aren't enough strategic dimensions; if outcomes feel random, readability is broken.

Explicit non-goals: direct unit control (no StarCraft), map editor, 10 factions, tower-defense-grade placement puzzles.

---

## 2. Quality bar — criteria for "a really good game"

These are the acceptance criteria we hold every change against. They are ordered: if two items conflict, the lower number wins.

### C1. Instant comprehension
- A new viewer, watching any 10-second battle clip, can tell: who is winning, which side a unit is on, what kind of unit it is (tank/ranged/caster/siege), and what just died.
- **Today this FAILS:** units have no team-color tint (only sprite flip), so team identity is inferred from position/facing. Combat is floating-text only; ranged units fire nothing visible.
- Bar: team colors on every entity (units, buildings, health bars, minimap, bounty text); damage types visually distinct; death is an event (not a vanish).

### C2. Strategic depth with verified balance
- Every race has at least two viable build archetypes per matchup; no matchup is >60/40 from even play.
- Decisions per minute: lane choice, front vs back, econ vs tempo, bounty hunting, timing pushes. Today the decision space exists (good!) but is **unverified** — no race-vs-race data has ever been produced.
- Counter logic must be *learnable, not look-up-able*: broad logic ("he's heavy frontline → I add magic") in one match, no spreadsheet during play. Unit tooltips state the actual multipliers (see P0-6).
- Bar: a committed headless simulation harness produces a win-rate matrix (race × race × archetype) on every balance change; match outcome arcs look like (equal early → mid edge → decisive late), not coin flips. North-star test in §1.1.

### C3. Pacing and drama
- Target match length 6–12 minutes with a real arc. Comebacks must be possible (bounties reward aggression into a lead).
- **Today this is UNVERIFIED and likely wrong:** castle regen is 10 HP/s flat, while an early single Barracks wave deals ~11 DPS *before* the 0.7× Fortified multiplier (~8 DPS net). The castle *out-heals early pushes entirely*, so early pressure feels dead and most games drift to 10:00 sudden death.
- Bar: scripted bot-vs-bot matches show decisive finishes and a sane time distribution; regen does not trivialize early damage (e.g., regen pauses while under fire).

### C4. Responsiveness and feel
- Visual motion is smooth at display framerate, not snapshot framerate. Inputs acknowledged within 100 ms (cursor, placement ghost, sound).
- **Today this FAILS:** the client despawns and respawns the entire scene on every snapshot (10 Hz); units teleport in 100 ms steps; the idle bob is driven by snapshot tick so it also steps at 10 Hz. Velocity is transmitted but never used for rendering. The full UI tree is also rebuilt every snapshot.
- Bar: units interpolate ≥60 fps between snapshots; hover/selection never flickers; every player action and every notable battle event has a sound within one frame.

### C5. Fair play and robust netcode
- The server never trusts the client, fog is enforced server-side, disconnection doesn't destroy the match, and the simulation is deterministic given a seed.
- **Today this PARTIALLY FAILS:**
  - Fog of war is *client-side presentation only* — the server ships the full unit list to both clients, so a cheat client reads all enemy positions. (Also wastes bandwidth.)
  - A disconnect mid-match resets the entire match to Lobby instantly — no grace period, no reconnect-to-match.
  - Sim targeting iterates a `HashMap<u64, …>` for target selection; equal-distance ties are broken by random iteration order → cross-run nondeterminism (blocks replays, seeded test arenas, and future lockstep).
  - Fire-and-forget commands: a lost UDP `PlaceBuilding` silently does nothing; no ack/retry.
- Bar: per-client filtered snapshots (or per-entity visibility masks); 60 s reconnect grace with match continuation; deterministic sim under a seed (verified by a fixed-seed test that two runs produce byte-identical snapshots); command acks or idempotent retries.

### C6. Zero-manual onboarding
- A player who has never read the README reaches their first meaningful build decision within 60 seconds, and understands bounties/econ within one match.
- **Today this FAILS:** all rules live in the README; in-game there are tooltips but no controls screen, no first-match hints, no glossary of attack/armor types.
- Bar: `H` opens a rules/controls overlay; first-match hint ticker (race → ready → build → lane pressure); race select explains the race's playstyle in one line.

### C7. Content depth
- Units are not stat sticks: abilities (heal, splash, slow, first-strike…), building upgrades/tiers, sell/refund, and a small set of commander powers give counterplay and comeback tools.
- Bar (Phase 2): every unit has at least one passive or triggered ability; buildings can be sold at 70% and upgraded once; 3 commander spells per race on cooldowns.

### C8. Presentation and audio
- Cohesive art direction (race palettes already exist), animated units (walk/attack/death), projectiles for ranged attacks, music + SFX, art-directed UI instead of code-drawn rects.
- **Today audio is entirely absent** (no `bevy_audio` feature, zero assets) and all sprites are single static frames.
- Bar (staged): Phase 0 = SFX for every core event + ambient loop; Phase 2 = 3–5 frame animations + projectile VFX + UI texture pass.

### C9. Performance and reliability
- Client: 60 fps with 300+ visible units. Server: 2× realtime at 1,000 units (current `sim_perf` harness already exists — keep it green). Bandwidth ≤ 30 KB/s/client at 200 live units (JSON deltas currently blow past this at high unit counts; binary encoding is the fix).
- No crashes over an 8-hour soak (bot vs bot).

### C10. Ship-ability
- Version string in the UI, `--version` flag, release binaries for macOS/Windows/Linux, CI running `task verify` on every push, clean module structure (client.rs is 3,900 lines today — refactor before Phase 2 piles on).

---

## 3. Current state audit (what already exists and is good)

Keep and build on these — they are the hard parts, already done:

| Area | State |
|---|---|
| Authoritative sim | `src/sim.rs` — economy, spawns, 2.5D movement + separation, typed combat, bounties, sudden death, adjudication. Well unit-tested (25+ tests). |
| Netcode | Custom JSON-over-UDP: multi-room lobby, delta snapshots with 1,200 B chunking, baseline resync on phase change, keepalive/timeout, protocol version gate (`src/net.rs`, `src/bin/server.rs`). |
| Client | Bevy 0.18: lobby browser, race modal, command card, grid placement with preview, selection/inspect, minimap, fog presentation, floating combat VFX, camera pan/zoom. |
| Balance data | Single `config/balance.json`, hot-loadable server-side, pushed to clients on join. Constants with serde defaults = backward-compatible config evolution. |
| Tooling | `task verify` (fmt + check + tests), headless bot (`bot.rs`), network flow integration tests (2-player flow, rematch, game list, duplicate names), sim perf benchmark (`examples/sim_perf.rs`). |
| Art pipeline | Documented generate→chroma-key→slice pipeline (`docs/assets.md`), 21 unit sprites + 24 building icons already in `assets/art/`. |

Known dead weight: `lightyear = "0.26"` is declared in `Cargo.toml` but **never used** — it massively inflates compile times. Remove it (or consciously adopt it later; see Risk R6).

---

## 4. Gap analysis (what's missing, mapped to criteria)

### A. Feel & presentation (C1, C4, C8)
| # | Gap | Severity |
|---|---|---|
| A1 | Full scene + full UI despawn/respawn every snapshot; no interpolation; 10 Hz animation | **Critical** — makes the game feel broken |
| A2 | No team-color tint anywhere (units, bars, minimap dots, floating text) | **Critical** for PvP readability |
| A3 | Zero audio (no feature flag, no assets, no triggers) | High |
| A4 | No projectiles/attack animations/death effects; ranged attacks are abstract streaks | High (Phase 2 for sprites, Phase 0 for cheap VFX) |
| A5 | Code-drawn UI panels, default font only | Medium (Phase 2) |
| A6 | Static single-frame sprites | Medium (Phase 2) |

### B. Simulation & gameplay (C2, C3, C7)
| # | Gap | Severity |
|---|---|---|
| B1 | No balance evidence; no automated match runner | **Critical** before calling balance "done" |
| B2 | Castle regen 10 HP/s flat invalidates early pressure; pacing unverified | **Critical** |
| B3 | Units have no abilities; buildings can't be sold/upgraded; no active player powers during battle | High (Phase 2) |
| B4 | Sudden-death pressure formula favors the bigger board (leader also has more units) → runaway snowball | Medium — measure first via B1 harness |
| B5 | Only 1v1; team modes (any even count) are a committed direction but the sim hardcodes `[Economy; 2]` / `[Castle; 2]` / `Team::slot()` everywhere | High — foundations Phase 1, full modes Phase 2 |
| B6 | No mid-map objectives/neutral elements | Low (Phase 3, optional) |

### C. Netcode & integrity (C5, C9)
| # | Gap | Severity |
|---|---|---|
| C1-gap | Server-side fog absent; full state leaks to clients | **Critical** for competitive integrity (and bandwidth) |
| C2-gap | Disconnect = instant match reset; no reconnect grace | High |
| C3-gap | Nondeterministic targeting via `HashMap` iteration | High (blocks determinism features; cheap to fix) |
| C4-gap | JSON encoding overhead; re-encodes each delta chunk to measure size | Medium (binary encoding in Phase 1) |
| C5-gap | Commands unacknowledged; lost placement = silent no-op | Medium |
| C6-gap | No auth/identity beyond self-declared name; name squatting after disconnect timeout | Low/Medium (Phase 3 with accounts) |
| C7-gap | Single-threaded server loop (fine at 2 players, revisit at scale) | Low |
| C8-gap | Unused `lightyear` dependency | Hygiene — remove in Phase 0 |

### D. UX & onboarding (C6)
| # | Gap | Severity |
|---|---|---|
| D1 | No in-game help/rules/controls screen | High |
| D2 | No settings (volume at minimum; fullscreen/res later) | Medium |
| D3 | No first-match guidance | Medium |
| D4 | Misplaced buildings are permanent (no sell) — feels unfair once players notice | Medium (Phase 1; trivial sim-wise) |

### E. Meta & shipping (C10)
| # | Gap | Severity |
|---|---|---|
| E1 | No persistence: profiles, W/L, match history, MMR | Phase 3 |
| E2 | No CI, no release packaging, no version display | High before any external playtest |
| E3 | `client.rs` 3,900 lines / `sim.rs` 2,400 lines monoliths | Medium — refactor gate before Phase 2 |
| E4 | No replays/observers (determinism fix C3-gap unlocks cheap replays) | Phase 3 |

---

## 5. Implementation plan

Sizing: **P0 = today's session**, P1 = this week, P2 = next, P3/P4 = when the core is fun. Each item lists concrete touch points and its done-when.

### Phase 0 — TODAY: "Make it feel like a game" (~6 focused work items)

> Theme: stop the bleeding on the three Critical feel/integrity gaps that don't require protocol redesign (A1, A2, A3), prove pacing with data (B1+B2), and fix determinism (C3-gap). Deliberately *not* in today: server-side fog, binary protocol, reconnect (P1 — they change the protocol and deserve their own focused session).

**P0-0. Land the working tree.** Commit the current uncommitted lobby/delta/fog work on `master`. Nothing else starts until the base is green: `task verify`.

**P0-1. Persistent scene + interpolated rendering (A1).** *(client only — biggest feel win per hour spent)*
- Replace `redraw_scene` full-rebuild with persistent entities keyed by snapshot entity id (`HashMap<u64, Entity>`); spawn/despawn only on delta creates/removes.
- Add a per-frame `interpolate_scene` system in `Update`: units lerp `pos` from previous snapshot → latest snapshot using `elapsed_since_snapshot / snapshot_interval` (velocity already arrives for dead-reckoning — use it for the last 20% of the interval; fallback to plain lerp). The idle bob moves to local `Time` so it animates at framerate.
- UI keeps a lighter rule: rebuild only when *content* hash changes, not on every snapshot; hover/selection updates mutate existing nodes.
- Touch points: `src/bin/client.rs` (`redraw_scene`, `redraw_game_ui`, `Update` chain, new `RenderSnapshot` resource).
- Done when: units move smoothly at 60 fps with the server at 10 Hz snapshots; no flicker on hover; `task verify` green.

**P0-2. Team readability pass (A2).**
- Team tint on unit sprites (left = neutral-white base with blue accent tint, right = red accent tint — tint via `Sprite::color`; sprites are grayscale-friendly enough, verify against art and adjust).
- Health bars: green→team color, bordered; armor/attack badges stay type-colored.
- Minimap dots, selection diamonds, bounty/damage floating text tinted by team.
- Buildings get a small team banner/flag rect; castles tinted.
- Done when: a screenshot of a mid-fight is readable by someone who has never played.

**P0-3. Audio foundation (A3).**
- Add `bevy_audio` + `vorbis`/`wav` features; write `tools/gen_sfx.py` (repo-committed, runs with the bundled Python from `docs/assets.md`) that *procedurally synthesizes* all SFX as WAVs into `assets/audio/` — no external API, fully reproducible: `ui_click`, `build_place`, `build_error`, `unit_spawn`, `melee_hit`, `ranged_shot`, `unit_death`, `bounty_coin`, `castle_under_attack`, `victory`, `defeat`, plus a short ambient music loop (layered pad/drones; keep it subtle).
- Client: `AudioAssets` resource, tiny `play_sfx(name)` helper with per-sound volume/cooldown map (e.g., `melee_hit` throttled to ≤8 concurrent), triggers wired into existing systems: placement input, `detect_combat_vfx` (already infers hits/deaths/bounties from snapshot deltas — hook there), phase transitions.
- `V` toggles master volume (persisted to `config/client_settings.json`), which also seeds D2.
- Done when: placing a building, watching fights, and winning/losing all produce correct sounds; volume toggle works.

**P0-4. Determinism + balance harness (B1, C3-gap).**
- `src/sim.rs`: replace the `HashMap` target scan with order-stable structures (sort candidates by `(distance, id)`; iterate units in id order where order matters). Add `GameSim::with_seed(seed)`; server passes a per-match seed (from room id + start time) — protocol unchanged (seed rides the existing baseline snapshot; bump `PROTOCOL_VERSION` to 5).
- New test: two sims, same seed + same scripted command log → byte-identical snapshots for 10,000 ticks.
- New `examples/sim_balance.rs`: bot-vs-bot Monte Carlo — heuristic bots (archetypes: `econ_rush`, `aggro_front`, `mixed`, per-race variants) play N matches headless at accelerated time; prints win-rate matrix + match-length histogram + castle-HP-over-time samples. Reuses `sim_perf` scaffolding.
- Done when: `cargo run --example sim_balance -- --games 200` produces the matrix; the determinism test is green.

**P0-5. Pacing fix driven by P0-4 data (B2, B4 first pass).**
- Implement "besieged" regen: castle regen pauses for X seconds after taking any damage (config: `castle_regen_delay_secs`, default ~8), so early pressure sticks without removing regen's anti-stall role.
- Sweep `sudden_death_start` (600 → ~420s candidates), pressure curve, and castle HP in `config/balance.json` against the harness until the match-length histogram centers in the 6–12 min band with <10% timeouts adjudicated.
- Design note on match length: other proposals for this genre target 15–30 min. 6–12 is a deliberate choice for iteration speed and modern attention spans; the interval is a config knob, and we revisit it after the first *human* playtests — the harness optimizes for "decisive and arc-shaped," the exact number is a design call, not a constant.
- Wave-dynamics knobs to evaluate with the harness (design options, not commitments): defender advantage near castle (regen already acts as a soft one), spawn-stagger vs synchronized waves, and whether the sudden-death pressure curve should reward *board quality* over board size to soften snowballs. Confirm the emergent rhythm alternates pressure → defense → stabilization → counter-push rather than one army rolling forever.
- Done when: harness report (committed to `docs/balance-report.md`) shows target distribution; manual 2-client game confirms early pushes now matter.

**P0-6. Onboarding minimum (D1, D3).**
- `H` toggle: rules overlay (goal, econ, lanes/zones, attack/armor matrix table, controls, rematch).
- First-match hint ticker (client-local state machine): "Pick a race and press Ready" → "Build producers — Top lane buildings feed the Top lane" → "Kill units for bounty gold" → "Reduce the enemy castle to 0".
- Race modal: one-line playstyle blurb per race (from balance data, static copy).
- Inspect tooltips expose the actual counter numbers, computed from `attack_multiplier`: e.g. *"Magic damage — 130% vs Heavy, 70% vs Light"*, plus role line (tank/ranged/caster/siege). Players must be able to answer "why am I losing?" from the UI alone.
- Done when: a fresh player needs zero README.

**P0-7. Hygiene.**
- Remove unused `lightyear` from `Cargo.toml` (compile time win).
- Add version constant printed in window title + `--version` flag on all three binaries.
- Split `client.rs` into modules **only if time remains** (`client/{main,net,scene,ui,input,vfx}.rs`); otherwise it's the first P1 item (E3).

**Today's definition of done:** `task verify` green + `sim_balance` report committed + one recorded 2-client match (bot peer) that is watchable: smooth motion, team colors, sound, a finish inside 12 minutes. Screenshots captured for the README.

### Phase 1 — Core game completion (next session(s))

1. **Server-side fog/interest management (C1-gap).** Per-client snapshot filtering: server keeps per-session last-ack'd state, emits only entities visible to that client's side (own entities + enemy entities inside vision of own units/buildings/castle; scouted buildings as tombstone silhouettes). Protocol: reuse `SnapshotDelta` per client (already per-session state on the server). Bandwidth drops as a side effect. Fog memory stays client-side for "explored" presentation. **Implement visibility as side-based logic** (which side owns the viewer, which side owns the entity) so it survives the player-count refactor untouched.
2. **Reconnect grace (C2-gap).** Disconnect during `Playing` no longer resets: player marked `disconnected`, match continues (their build plan idles), `waiting for reconnect` shown; rejoin by name within 90 s restores their seat (server session table already keys by name+addr; add seat reservation). Reset only on timeout or both-gone.
3. **Player-count foundations: de-hardcode the sim (B5 groundwork).** Mechanical refactor enabling team play later: `economies`/`castles` become per-player vectors keyed by `PlayerId`; `Unit.owner`/`Building.owner` become `PlayerId` (side derived from the player); victory = *all castles on a side dead*; bounties pay the owning player; economy ticks per player; protocol bump (arrays → vectors in `SnapshotDelta`); adjudication score sums per side. **The lobby still seats only 1v1 after this** — no new mode yet, but every later feature (binary protocol, fog extension, recording) lands on final structures instead of being migrated twice. Harness gains lineup-vs-lineup support.
4. **Binary encoding (C4-gap).** `bincode` (or hand bit-packing later) behind the existing `encode/decode` seam; keep JSON behind a `--legacy-json` debug flag during migration; bump protocol; re-run `sim_perf` and record bandwidth before/after in `docs/`. Scheduled *after* item 3 so packet structs migrate once.
5. **Command acks (C5-gap).** Server echoes `{client_seq}` on applied intents; client retries un-acked intents for 1 s. Fixes silent lost placements on UDP.
6. **Sell/refund + building move (D4).** `sell = 70% refund` (sim command + command-card button + economy rule + tests). Sell exists to enable *pivots* — a bad read at minute 3 must be recoverable, so keep early buildings cheap and refunds generous enough that counter-building stays a live option.
7. **Surrender/concede (reuses rematch-vote plumbing).** A player stuck in a lost game should be able to end it without alt-F4 (which currently resets the match for both players via disconnect handling).
8. **Match recording (replay groundwork + telemetry).** P0-4 makes the sim deterministic, so a server-side command log + seed + result = a replay file and a telemetry source in one. Record per match: commands with timestamps, final snapshot, per-unit damage/kill totals, gold spent per building kind, army value over time, castle-HP timeline. This is the data the balance loop in §6 and Phase 3 profiles both consume.
9. **Client module refactor (E3)** if not done in P0-7 — gate for Phase 2.
10. **CI (E2).** GitHub Actions: `task verify` + `sim_balance --games 50` smoke on push; artifact binaries on tag.
11. **Settings (D2).** Fullscreen/resolution/volume/keybind display in a settings panel.

### Phase 2 — Team play + depth: abilities, upgrades, content (after Phase 1)

1. **Team play: any even player count (user-confirmed headline; Castle Fight's true shape).** Design that falls out of the Phase 1 foundations:
   - **One lane per player.** Total lanes = player count (2P = today's Top/Bottom; 4P = 4 lanes; 6P = 6…). Lane *k* is owned by Left player *k* and Right player *k* — you duel your mirror across your lane, exactly like Castle Fight's per-player lanes. Contiguous lane bands per side; junctions near castles stay cross-lane connected.
   - **One castle per player** at both ends of their lane. Your castle dies → you're **eliminated** (buildings crumble, surviving units fade, seat goes spectate-until-match-end). A side loses when all its players are eliminated.
   - **Per-player economies** (already per-player after P1-3); bounty pays the killing unit's owner; income buildings scale per player.
   - **Lobby:** host picks game size (2/4/6/8), auto team assignment with swap, **bot fill** for empty seats (the headless bot logic moves server-side so fill bots work without extra processes).
   - **Client scaling:** battlefield grows vertically with lane count — camera zoom-out presets, minimap aspect, HUD compacting, scoreboard listing players per side with gold/army/castle bars.
   - **Balance:** 2v2+ shifts the meta (lane pressure pairs, focus fire, who helps a dying neighbor). The harness runs lineup-vs-lineup (e.g. GG/EE vs GG/VV race pairs) — no hand-tuning without data.
   - Acceptance target: a stable 2v2 with 2 bots; larger sizes ship on the same code path.
2. **Counter-chain design doc first, code second.** Before implementing abilities, write `docs/counter-chains.md`: the role interaction graph every unit must satisfy — *each unit beats something specific, is beaten by something specific, and has a reason to exist beyond raw DPS efficiency*. Target chains (armor typing is only one axis):
   - Heavy frontline → loses to magic; magic casters → lose to fast divers; divers → lose to cheap melee/control; cheap swarm → loses to AoE; AoE backline → loses to long-range/assassins; healing → loses to burst/anti-heal; ranged mass → loses to gap-closers or splash.
   - Each existing unit gets a role assignment (tank / melee DPS / ranged DPS / caster / healer / buffer / debuffer / splash / siege / assassin / summoner / power unit) and a place in ≥1 chain. Units that fit nowhere get reworked or cut — this is the "12 units that are genuinely fun beats 21 stat sticks" principle, applied by audit rather than deletion.
3. **Unit abilities (B3).** Sim-side ability system (data-driven from `balance.json`), implementing the chains from item 2: `heal_pulse` (Battle Cleric becomes an actual healer), `splash` (Ballista/Cinder Engine), `slow` (Mire Shaman/Smoke Witch), `anti-heal`, `first_strike`, `regeneration` (Barkguard/Treant), `berserk` (Runner low-HP speed), plus **auras/synergy hooks** so compositions (not just single counters) become buildable: attack-speed banner, poison amplification, bonus vs slowed, engineer armor aura. Client: ability icons in inspect panel, VFX hooks, tooltips. This is the single biggest *gameplay* upgrade — it turns stat-stick soup into counterplay.
4. **Building upgrades with branching, not linear tiers.** `Basic building → specialization A or B` (e.g. Mage Tower → Pyromancer (AoE) *or* Warlock (anti-heavy/debuff)) creates better decisions than "Level 2 = +20%". Visible on the map, sellable under the Phase 1 rules.
5. **Commander powers — OPTIONAL, needs one confirmation.** (You answered "no units" to the earlier question; I read that as *no unit control* — nothing is ever micro'd, the game stays a pure autobattler. If you meant "no active abilities at all," delete this item.) 3 per race, gold-cost + long cooldown, cast from the command card onto the field: e.g., Vanguard "Rally" (lane units +speed), Grove "Entangle" (root enemies), Ember "Meteor" (area damage), plus utility options like emergency castle shield, income boost, or wave haste. 1–3 actives per player, never more — build → watch stays the core loop. Defer to after items 1–4 either way.
6. **Animation + projectile pass (A4, A6).** Pilot on one race first: generate walk/attack/death frames (2+2+1) per unit via the image pipeline (see §7), `TextureAtlas` playback state machine; ranged units spawn visible projectile entities (server sends `attack` events or client infers as today — prefer explicit `CombatEvent` stream added to deltas).
7. **UI texture pass (A5):** generated panel/button frames matching `rts_ui_frame.png` style, race-accent theming, real display font.
8. **Expanded harness:** archetype bots use abilities/upgrades; balance report extends to ability usage stats, synergy pair win rates, "how often does building X flip the lane" (see §6), and team-size lineup matrices.

### Phase 3 — Meta, modes, scale

- Spectator slots (read-only filtered snapshots for any team size — the Phase 1 fog work makes this nearly free; spectate also becomes the default seat for eliminated players in team modes).
- Replays: deterministic sim (P0-4) + the Phase 1 command log = record-and-playback files; store last N on server. Spectators and replays share the same filtered-snapshot playback path.
- Faction draft / random mode: forced-random or ban/pick race selection as a replayability lever (cheap — reuses lobby race selection), after the race roster is balanced.
- Profiles/persistence: local-first (name + W/L + history in SQLite/JSON on server), lightweight MMR, match history screen.
- Matchmaking queue alongside the manual game list; simple identity token so names can't be squatted.
- Mid-map neutral objective (capturable mercenary camp / shrine granting income) — evaluate only after ability meta settles.
- Web client feasibility spike (Bevy WASM + WebSocket transport for the existing protocol) — decide go/no-go; native stays primary.

### Phase 4 — Ship

- Release packaging: macOS (.app + notarization path), Windows (NSIS/zip), Linux (tarball/AppImage); icon + logo (generated).
- 8-hour bot-soak stability run in CI nightly; crash log capture.
- Playtest build distribution (itch.io private page), feedback capture, balance cadence using the harness.
- Only then: broader promotion/Steam decision.

---

## 6. Balance & simulation tooling (the safety net)

The harness from P0-4 becomes the standing rule: **no balance.json change merges without a regenerated `docs/balance-report.md`**. Contents per run:

- Win-rate matrix: race × race (and archetype × archetype within race).
- Match length distribution (p10/p50/p90), timeout rate.
- Economy curves (gold/income over time per archetype), first-castle-hit time.
- Unit kill/bounty totals per unit kind (flags units that never get built or always win their lane).
- **Composition-vs-composition mode:** pin both build scripts (e.g. `4 tanks + 2 healers` vs `3 casters + 4 swarm`) and run hundreds of battles instantly — the fastest way to verify a counter chain from `docs/counter-chains.md` actually holds. Plus an effective-HP/DPS calculator for quick spreadsheet-free sanity checks.
- **Lane-flip metric:** how often does building/adding unit X reverse the lane's pressure? This measures counter-building *working* — far more informative than raw DPS totals.
- Purchase timing and gold-efficiency per building kind; sell frequency once selling exists.

Bot quality bar is "differentiable strategies," not "good players" — heuristics are fine; determinism (P0-4) makes runs reproducible. Offline harness + Phase 1 match recording together cover the telemetry need: simulated data for tuning, real-match data for validation.

---

## 7. Art & audio plan — and what I need from you

**Short answer: I can create images myself, today, with no external API key.** The image-generation tool available in this environment works out of the box (built-in path, no `OPENAI_API_KEY` required — same pipeline shape the project already used per `docs/assets.md`). The only thing that would require *you* is the optional CLI fallback (`gpt-image-2` with exact size/quality control), which needs `OPENAI_API_KEY` set in your environment — **optional, not needed for anything planned below.**

Planned generated assets (all original, chroma-key pipeline already documented):

| When | Asset | Notes |
|---|---|---|
| P1/P2 | Team-tint verification sheet | May need one neutral-palette unit re-render per race if tinting looks muddy — decided after P0-2 |
| P2 | Animation frames: walk ×2, attack ×2, death ×1 per unit | One atlas per unit (21 calls) + existing slice script; **pilot on Vanguard first**, then batch. Frame consistency across generations is the known risk — mitigate with tight shared prompt + accept minor variance |
| P2 | Ability icons (~12) + commander-power icons (9) | 128×128, race palettes, same icon style prompt as `docs/assets.md` |
| P2 | UI panel/button/frame textures + display font pairing | Based on existing `rts_ui_frame.png` style reference |
| P2/P4 | Title screen art, logo, race crests, app icon | For menu + store/itch page |
| P3 | Map terrain texture (lane paths, junction detail) to replace procedural grass | Keep procedural as fallback layer |

Audio needs **nothing from you**: P0-3 synthesizes all SFX procedurally (committed generator script = reproducible, license-clean). If we later want real orchestral music, that's the one place I'd ask for either a licensed asset pack or an external music tool — flagged, not blocking.

---

## 8. Verification (how we know it's good)

- **Every phase:** `task verify` (fmt, check, full test suite) green.
- P0 adds: determinism test (byte-identical seeded runs), `sim_balance` report, recorded smooth-motion match + readability screenshots.
- P1 adds: fog test (client B's packets never contain units outside A's vision — server-side testable), reconnect integration test (drop client mid-match, rejoin, match continues), bandwidth regression vs `sim_perf` baseline.
- P2 adds: per-ability sim tests; harness runs with abilities on.
- Standing manual pass: fresh-boot "new player" run (no README) reaching first build in <60 s.

---

## 9. Risks & open questions

- **R1 — Persistent-entity rendering is the riskiest P0 item** (touching the largest file). Mitigation: strict entity-id→entity map module with tests; scene rebuild path kept behind a `--legacy-render` flag for one session.
- **R2 — Team tinting may look bad on the existing art** (sprites weren't drawn for tinting). Fallback: outline/under-glow rect in team color instead of sprite tint — decided visually within 30 minutes of P0-2.
- **R3 — Regen/pacing changes invalidate the current test suite's expectations** (several tests assert regen behavior). Budgeted: update tests together with the balance sweep, harness arbitrates.
- **R4 — Procedural music can be underwhelming.** Acceptable for P0 (subtle ambient); real music deferred to asset pack/decision in Phase 4.
- **R5 — Animation-frame consistency from image generation.** Pilot-race-first strategy; if frames drift, fall back to 2-frame (idle/step) sets + stronger code-driven squash/stretch.
- **R6 — ~~Remove vs adopt `lightyear`~~ RESOLVED:** removing it (P0-7). The custom delta protocol is simple, tested, and fits the game.
- **R7 — Working tree has ~1,360 uncommitted lines.** P0-0 lands them first; if you'd rather review/split that commit, tell me before we start.

### Decisions from review round 1 (locked)
1. **Multiplayer: any even player count** (1v1, 2v2, 3v3, 4v4…). Foundations in Phase 1 item 3, full modes as the Phase 2 headline.
2. **No unit control — pure autobattler.** "No units" is implemented as: nothing is ever directly micro'd. Commander powers (Phase 2 item 5) are build-menu actives, not units — kept as an optional design item pending one word from you ("keep" or "drop"); nothing else depends on it.
3. **Remove `lightyear`** — confirmed; happens in P0-7 today.
4. **Art: built-in no-key image pipeline** — confirmed; §7 needs nothing from you.

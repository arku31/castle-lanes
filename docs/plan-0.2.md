# Castle Lanes v0.2 Plan — "WC3-style 3D" (Option 3: full low-poly 3D)

Status: **PLANNED — not started.** This is the working plan for the 0.2 milestone.
v0.1 (tagged `v0.1.0`, crate version `0.1.0`) is the current 2D-sprite game: quick-play
MVP, authoritative deterministic sim, 2D Bevy renderer, HUD verified at 1600×900.
Governing roadmap for everything already shipped stays `plan.md`.

Companion concept art (generated, see `docs/concepts/`):
- `docs/concepts/v02_battlefield_concept.png` — target look of the battlefield.
- `docs/concepts/v02_castle_concept.png` — Vanguard castle model sheet.

The one-sentence goal: **keep the game we have, change what it looks like** — a pitched
WC3-style 3D world with low-poly Blender models, real terrain, real light and shadows,
and the approved grid/placement redesign — with **zero changes to simulation, network
protocol, or determinism**.

## 1. Vision pillars (what "WC3 experience" means here)

1. **Terrain you can read**: lanes are causeways, castle zones are raised terraces,
   cliffs and a water channel separate them. Height does the composition work.
2. **Lit, grounded models**: one consistent sun, real shadow-mapped shadows, soft
   contact grounding. No floating flat sprites.
3. **RTS camera feel**: fixed ~50° pitch, wheel zoom, edge pan, Home reset — all
   already exist in input code; they map directly onto a 3D camera.
4. **Placement that feels like building**: grid only when placing, ghost building,
   animated build circle, construction scaffold state.
5. **Everything else stays**: deterministic sim, protocol v10, matchmaking, replays,
   profiles, HUD layout (2D overlay), balance config.

## 2. Non-goals for 0.2

- No sim/net/gameplay changes (fog *rules* stay server-side; only presentation moves).
- No skeletal-animation showcase: walk/strike/death are procedural first, baked clips
  only where cheap (see M4).
- No new races, units, buildings, or balance changes.
- No browser/web target.

## 3. Architecture

### 3.1 Renderer migration strategy

- Dual-camera Bevy setup:
  - **World camera**: `PerspectiveCamera` with fixed pitch (~50°) looking down the
    field; zoom = camera distance along the pitch axis; edge pan = XZ translation
    clamped to map bounds.
  - **UI camera**: the existing `Camera2d` overlay with a higher `order` — the entire
    current HUD (panels, minimap, command card, overlays) keeps working unchanged.
    This is the single biggest de-risking decision: `ui.rs`, `input.rs` hit-testing,
    `UiLayout` all survive as-is.
- World rendering swaps `Sprite` scene entities for 3D ones:
  - Units/buildings: glTF scenes from `assets/models/<race>/...` (Blender-authored).
  - Terrain: a heightfield mesh built at startup from a small in-reach heightmap spec
    (authored as code + texture, not a giant binary asset).
  - Health bars, selection circles, damage numbers, bounty text: camera-facing
    billboard quads / world-space `Text3d`-equivalents.
- **Fallback that is also milestone 0**: before any models exist, the 3D world renders
  with flat-shaded placeholder meshes + the *existing 2D sprites as camera-facing
  billboards* (`bevy_sprite3d`-style textured quads). This is the Option-2 stopgap and
  the permanent safety net if an asset misses its milestone.
- Keep the old 2D world renderer behind `--renderer2d` until M4 so every milestone has
  a known-good escape hatch (UI/`pin_ui_to_camera` stays untouched either way).

### 3.2 Picking

- Ground/build cells: analytic ray→terrain intersection, then invert
  `cell_to_world` (same math, +Y swap). Terrain height at the aim point is looked up
  from the heightmap function, so cells stay exact.
- Units: ray-vs-sphere using the sim's existing collision radius (no new data needed).
- Buildings/castles: AABB in world space.
- `update_world_hover`/`placement_input` keep their semantics; only the
  screen→world conversion changes.

### 3.3 Fog of war (presentation only)

M0–M2: keep the current server-replicated tile fog, rendered as dark alpha quads
conformed to terrain height (same `FOG_COLUMNS × FOG_ROWS` grid, each quad snapped to
the heightmap). M5 option: screen-space post-process fog mask for soft edges. Fog
`explored` memory, reveal radii, and packets are untouched.

## 4. Terrain spec

- World footprint matches today's map (MAP_W 3600 × MAP_H 560 sim units; 1 sim unit =
  1 world unit today — keep, or introduce a clean ×0.1 scale once, in M0, and never
  again).
- Height layout: castle terraces (raised, flat) at both ends → short cliff bands →
  slightly sunken lane fields → central shallow water channel with stone bridges on
  the lanes. Height function is deterministic code so `cell_to_world` stays exact.
- Textures: hand-painted tileable albedo generated with the codex imagegen pipeline
  (see §6): grass ×3 variants, dirt lane road, cliff rock, castle terrace stone,
  water. Blended by vertex attribute, not splat-map complexity.
- Doodads: trees, rocks, banners, bridge props. Static, merged, low count (~200
  instances), no shadows-only-casters without purpose.

## 5. Blender asset pipeline

Install: `brew install --cask blender`; author scripts run headless
(`blender -b -P tools/blender/build_asset.py -- --kind vanguard_castle`).
All scripts live in `tools/blender/` and are committed — **models are code** so they
are diffable, re-renderable, and recoverable.

- **Style**: low-poly with hand-painted albedo. Crisp silhouettes first, detail second.
- **Poly budgets**: unit ≤ 600 tris, building ≤ 2,000, castle ≤ 5,000, doodad ≤ 300.
- **Textures**: one albedo atlas per faction (1k). Team color = separate banner
  material tinted blue/red at runtime — one model per faction, not per side.
- **Consistent rig**: one shared `tools/blender/lighting_rig.py` (sun angle, camera,
  film) so every render and every in-engine look matches — the single source of visual
  consistency.
- **Exports**: glTF 2.0 (`+Y up`, meters) to `assets/models/<race>/`. Static meshes
  first; any animation = baked clip in the same glTF.
- **Construction/upgrade states** are geometry variants of the base model (scaffold
  ring, half-height growth, accent prop swap) — no separate authoring per state.
- **Building kitbashing**: the 24 buildings share a part library (base slab, tower,
  roof, crystal/emblem, banner, pen/garden patch) — a building is a part list in a
  manifest, assembled by script. This is what makes 24 buildings tractable.
- **codex imagegen** generates: concept sheets, hand-painted texture tiles, and icon
  refreshes if needed (pipeline: `docs/kb/image-generation.md`,
  `tools/gen_vanguard_batch.sh` pattern). 3D geometry itself is Blender-authored.

## 6. Camera, controls, lighting

- Camera: pitch fixed 50–55°, FOV ~38, yaw locked in 0.2 (free rotate is a 0.3 idea).
  Existing wheel zoom (0.3–3.0 ortho scale) maps to distance; edge pan and Home reset
  map directly; `--camera-x` finally gets parsed by `parse_args` and used as the XZ
  start position.
- Lighting: single `DirectionLight` (warm, ~35° elevation from camera-left) + ambient
  + subtle hemisphere; one 2k shadow map; only buildings/castles/large doodads cast
  shadows by default, units get blob shadows until M5 proves unit shadow casters are
  affordable.
- Post: light ACES-ish tonemapping, gentle vignette; no bloom until perf is proven.

## 7. Grid & placement redesign (approved — spec)

Idle map: **no per-cell grid at all**. Lanes read via dirt-road texture, build zones
via subtle terrace trim + faction accent line. The spreadsheet look is gone.

When a building is selected (1–8 or command card):
1. Buildable area appears: faction-tinted **footprint tiles** over own zones only
   (beveled tile atlas rendered with the shared rig — not flat colored rects), other
   zones untouched.
2. Hover: tile pulses softly; the **ghost building** (real model, 55% alpha, blue tint
   valid / red invalid) sits at the cell; defensive buildings show a range ring.
3. Click: WC3-style **animated build circle** plays; building appears in
   **construction state** (scaffold variant), grows/heals to full — sells/upgrade UI
   unchanged.
4. Team play: non-assigned lanes show nothing at all while placing (current dim-lane
   squares are removed).
5. Escape/X cancels; existing sounds stay.

## 8. VFX, bars, and text in 3D

- Damage numbers / bounty text: screen-space overlay pass (project world→screen each
  frame; they already live at UI-ish z).
- Health bars + selection circles: camera-facing billboards, same team colors.
- Combat streaks/hit bursts (vfx.rs): ported as short-lived oriented quads or simple
  particle billboards; budget system (`VFX_BUDGET_PER_SNAPSHOT`) stays.
- Castles keep their "under attack" alarm and bar behavior unchanged.

## 9. Performance budget

Target: **60 fps at 300+ concurrent units** on the M4 test Mac, 1600×900.
- Materials: one atlas per faction; merged static terrain; instanced/merged doodads.
- Shadows: buildings/castles only (M0–M4); unit shadow casters only if M5 shows
  headroom; blob shadows otherwise.
- Culling: frustum (free) + lane-level visibility toggles for extreme cases.
- Keep the interpolation/animation driver on the display clock (already the case).

## 10. Milestones (each ends playable + screenshot gate)

- **M0 — 3D foundations (fallback online)**: dual camera, heightfield terrain slab
  with cliffs + water channel, sun + shadows, raycast picking, placement preview on
  terrain, HUD untouched, `--renderer2d` flag. *Gate: full match vs bot on the 3D
  field with sprite billboards; screenshots reviewed.*
- **M1 — Grid & placement UX**: §7 shipped in full (renderer-agnostic spec; can run
  in parallel with M2 on the billboard world). *Gate: side-by-side vs v0.1 grid.*
- **M2 — Blender pipeline proof**: headless bpy tooling committed; Vanguard castle
  modeled, textured, in-engine, shadowed; castle replaces billboard. *Gate: user
  reviews the castle in-game.*
- **M3 — Vanguard set + terrain art**: castle + 8 buildings + 7 units; grass/dirt/
  cliff/water/terrace textures; construction states. *Gate: Vanguard vs Ember-billboard
  match looks coherent.*
- **M4 — Grove + Ember sets, doodads, states**: remaining 14 buildings + 14 units,
  trees/rocks/banners/bridges; `--renderer2d` removed. *Gate: full art pass reviewed.*
- **M5 — Perf & polish**: 60 fps/300 units budget met; fog conformant/soft; VFX port;
  damage/bounty text overlay pass; unit shadow decision. *Gate: perf report.*
- **M6 — Release 0.2.0**: packaging check, crash-reporter test, fresh screenshots +
  README rewrite, balance sanity run (`sim_balance`), tag `v0.2.0`.

## 11. Asset checklist

| Group | Count | Notes |
| --- | --- | --- |
| Castles | 3 | one per faction, banner tint handles side color |
| Buildings | 24 (8 × 3) | kitbashed from shared parts; upgrade branches = accent variants |
| Units | 21 (7 × 3) | procedural motion first; baked walk clips only if cheap |
| Doodads | ~6 | tree ×2, rock, banner, bridge, crystal |
| Terrain tiles | ~8 | grass ×3, dirt, cliff, terrace stone, water, build-footprint |
| Textures | 3 faction atlases + terrain set | codex-generated hand-painted, Blender-baked AO |

## 12. Risks & mitigations

| Risk | Mitigation |
| --- | --- |
| Perf with hundreds of 3D units | budgets §9; billboards fallback per asset; instancing in M5 |
| Animation scope explosion | procedural bob/lean/death first (§5); baked clips only for walk |
| Picking regressions vs 2D | analytic raycast + sim radius reuse; tests against `cell_to_world` |
| 24 buildings stall M4 | kitbash manifest (§5); branch upgrades are variants, not models |
| Scope creep mid-milestone | every milestone is playable; billboard fallback is always shippable |
| Visual inconsistency across assets | one shared lighting rig + one atlas style guide (§5) |
| Carried-over winit self-exit bug | unchanged; see HANDOFF.md — client demos run focused |
| Toolchain: cargo no-ops on this volume | `touch` edited files before every build (HANDOFF.md) |

## 13. Acceptance criteria (definition of "0.2 done")

1. All M0–M6 gates passed with user-reviewed screenshots.
2. "WC3 feel" checklist: terrain height/cliffs/water visible in every battle; every
   unit/building lit by the shared sun with grounded shadows; placement feels like
   building (ghost + circle + scaffold); grid invisible while idle.
3. 60 fps with 300+ units at 1600×900; no determinism or netcode diffs (`cargo test`
   + replay verifier untouched and green).
4. HUD/lobby/overlays still fit 1600×900 with no scrolling (v0.1 gate carries over).
5. README, screenshots, and balance report refreshed; `v0.2.0` tagged.

## 14. Open questions (decide at M0 start)

- World scale: keep 1 sim unit = 1 world unit, or introduce ×0.1 once in M0?
- Keep `--renderer2d` permanently as a low-end fallback, or remove at M4 as planned?
- Free camera yaw in 0.2 or defer to 0.3 (plan assumes defer).
- Unit shadow casters: decide with M5 perf data (plan assumes blob shadows).

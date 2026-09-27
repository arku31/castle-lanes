# Goal: WC3 feel + solid game feel — Castle Lanes v0.3 (presentation only)

Repo: `github.com/arku31/castle-lanes` (Rust / Bevy 0.18.1). ALL work is client
presentation: camera, scene, VFX. **Never touch** `src/sim.rs`, `src/net.rs`,
`src/bin/server.rs`, protocol, balance — determinism is sacred. `--renderer2d`
must keep working. Animations are cosmetic, driven only by existing snapshot
fields (`unit.velocity`, `unit.attack_timer`, `unit.health`).

Run (repo root, 3 terminals or `task run-server` / `task run-client` / `task run-bot`):

```
cargo run --release --bin castle_lanes_server -- 127.0.0.1:4000
cargo run --release --bin castle_lanes_bot   -- --name Bryn  --server 127.0.0.1:4000 --race ember
cargo run --release --bin castle_lanes_client -- --name Alice --server 127.0.0.1:4000 --race grove --auto-ready --auto-build-demo
```

`task verify` = fmt + clippy + tests; keep it green after every item.

## Why it doesn't feel like WC3 (measured, don't re-litigate)

- Camera FOV **40°** (`CAM_FOV_DEG`, src/bin/client/render3d.rs:48) vs WC3 ~20°:
  converging verticals (towers lean), far units collapse → "3D tech demo" look.
- Units **46** world units tall on **128** lane spacing = 0.36 tile (WC3: 0.6–0.75)
  → units are 3–6% of screen height (WC3 ≈ 12%).
- Max zoom-out (dist 1488) fits the whole map depth → ants.
- 150 doodads on a 6000×1560 map, no props framing the screen edges;
  channel depth 34 vs unit height 46 = canyon relief.

## Checklist — implement in order, each with screenshot proof

**C1 Long-lens camera** (constants at render3d.rs:45–49):
FOV 40→22°; `CAM_BASE_DIST` 620→~1250 (tan ratio, keeps the same visible ground
band); pitch 48→54°; rescale `CAM_ZOOM_MIN/MAX` so max zoom-out keeps units
≥5% of screen height. Optionally gate behind `--wc3cam` for one-build A/B, then
make it the default.
✅ no visible tower lean at default view; near/far units differ <20% in size;
units ≥10% of screen height at zoom 1.0.

**C2 Unit/building scale** (render-side only; `spawn_unit_visual_3d`
render3d.rs:1415 + building/castle spawn in render3d.rs/scene.rs):
units ×~1.8, buildings ×~1.4, castle ×~1.2. Grid/footprints/placement unchanged.
✅ unit height ≈ 0.6–0.7 lane spacing; nothing overlaps lanes or walk paths;
health bars/sel circles rescaled to fit.

**C3 Animation — no more sliding statues** (procedural, no rig; extend
`animate_units_3d` render3d.rs:1640):
- walk: bob + forward lean + weapon sway while `velocity` ≠ 0
- attack: windup→strike pose pulse keyed on `attack_timer` nearing reset
- death: fall over + sink/fade (never vanish in place)
- spawn: pop-in scale; hit: brief white/emissive flash on damage
✅ watching one lane for 30s shows all five states; units never slide.

**C4 Combat juice** (extend src/bin/client/vfx.rs — bursts/projectiles exist):
death poof particles, arrow/shot trails, build dust, bounty coin burst,
small screen shake on castle hits.
✅ every visible hit/kill/build gives feedback in ≤1 frame.

**C5 Stage framing** (`spawn_doodads` render3d.rs:688 + terrain consts):
concentrate trees/rocks in the camera-visible band; continuous tree/rock rows
bordering top+bottom of the default view; shallow `CHANNEL_DEPTH` and castle
rise relative to new unit size.
✅ any default screenshot has props bordering the frame; channel reads as a
moat, not a canyon.

## Validation loop — mandatory after EVERY item

1. `task verify` green.
2. Boot server + bot + demo client (commands above); capture 1600×900 PNGs:
   battle at zoom 1.0, zoomed out, mid-fight close-up.
3. Run a **critique pass with the impeccable skill** on those screenshots
   (composition, unit readability, WC3 likeness, animation legibility).
   Fix the concrete defects it lists, re-shoot, repeat until it passes.
4. Accept an item only when: ✅ criteria met + impeccable pass + before/after
   pair saved to `docs/screenshots/`.
5. One commit per item: `C1 long-lens camera`, `C2 unit scale`, etc.

Ship order: C1+C2 together (one playable A/B), then C3, then C4, C5.
Stop after each pair and play 2 minutes before continuing.

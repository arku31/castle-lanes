# Image generation via Codex (no API key)

**Capability confirmed 2026-09-05.** The `codex` CLI (installed at
`~/Library/Application Support/Herd/config/nvm/versions/node/v22.22.2/bin/codex`,
auth via the user's `~/.codex/auth.json`) can drive the built-in image
generation tool headlessly. It bills against the user's Codex allowance —
no `OPENAI_API_KEY` needed. This supersedes the "blocked" status that
`docs/assets.md` and plan.md previously recorded for sprite generation.

## How to generate one asset

```bash
codex exec --sandbox workspace-write 'Use the built-in image generation tool to create ...
Save the final image exactly to <path inside the repo>.'
```

Notes learned by doing:

- `--sandbox workspace-write` lets Codex write only inside the repo; point the
  prompt at an explicit repo-relative save path.
- One call generates **one image**; a single call can contain a full atlas
  (e.g. a 1×5 frame row) — prefer that over one call per frame.
- Runtime is roughly 1–3 minutes per image. Run batches sequentially in a
  background driver and log per-unit (`tools/gen_vanguard_batch.sh` is the
  reference implementation).
- The skill's built-in `image_gen` tool is NOT exposed in the agent toolset —
  `codex exec` is the working route. The skill's CLI fallback
  (`scripts/image_gen.py`) requires `OPENAI_API_KEY` and stays unused.
- A Pillow venv for post-processing lives at
  `~/.zcode/skills/imagegen/.venv` (`.../.venv/bin/python` has PIL).

## The unit frame-atlas pipeline

1. **Generate** (see `tools/gen_vanguard_batch.sh` for the prompt template).
   Prompt contract that produced clean results:
   - one row of exactly 5 equal square cells
   - flat solid `#ff00ff` chroma-key background, no gradients/shadows/grid
   - 3/4 isometric view facing camera-right, hand-painted RTS style
   - frames: idle, walk mid-step, attack wind-up, attack strike, death
   - "the SAME identical character in all 5 frames; only the pose changes"
   - explicit repo-relative save path
2. **Slice/key/normalize**: `tools/slice_frame_atlas.py` — uses the
   `~/.zcode/skills/imagegen/.venv` Pillow:
   ```bash
   ~/.zcode/skills/imagegen/.venv/bin/python tools/slice_frame_atlas.py \
       assets/art/units/atlases_raw/vanguard_guard_frames.png \
       assets/art/units/frames/vanguard_guard
   ```
   Produces `_frame0..4.png` (idle, walk, wind-up, strike, death) on a shared
   192×192 ground-registered canvas + `_contact.png` for review.
3. **Client playback**: `UnitFrameSets` (client `main.rs`) maps
   `UnitKind -> [Handle<Image>; 5]`; `animate_units` (client `scene.rs`)
   swaps `Sprite.image`: walk cycle while moving, wind-up/strike right after
   the server resets the attack timer, and `sync_units` spawns a fading
   death-frame "corpse" when a unit vanishes.

## Status

- Vanguard (8 units) atlases: generated via `tools/gen_vanguard_batch.sh`.
- Grove/Ember: same driver with new descriptions when scheduled.
- The skill's own `remove_chroma_key.py` is an alternative key-removal pass if
  the inline tolerance in `slice_frame_atlas.py` ever leaves fringes.

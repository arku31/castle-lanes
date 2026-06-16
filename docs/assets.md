# Asset Creation

Castle Lanes uses original generated bitmap art plus code-drawn UI and gameplay overlays. Do not use Warcraft, Blizzard, or Castle Fight map assets directly.

## Current Assets

- The battlefield background is now procedural Bevy rendering: neutral grass, subtle patches, and lightweight animated grass blades.
- `assets/art/isometric_battlefield.png`: legacy/generated battlefield backdrop kept as source/reference, not currently spawned by the client.
- `assets/art/rts_ui_frame.png`: generated UI style reference kept for future panel work.
- `assets/art/units/unit_atlas_source.png`: generated original six-unit atlas source.
- New race extension unit atlases were generated in Codex image generation output and sliced into project PNGs.
- `assets/art/units/*.png`: cropped transparent unit sprites consumed by the Bevy client.
- `assets/art/units/unit_sprites_contact.png`: contact sheet for quick visual review.
- `assets/art/buildings/building_icons_atlas_source.png`: generated original nine-building icon atlas source.
- New race extension building atlases were generated in Codex image generation output and sliced into project PNGs.
- `assets/art/buildings/*.png`: cropped `128x128` building command icons consumed by the Bevy client.
- `assets/art/buildings/building_icons_contact.png`: contact sheet for quick visual review.

## Unit Sprite Pipeline

The first unit sprites were created as a single 3x2 atlas:

- Top row: Vanguard Guard, Vanguard Archer, Grove Bruiser.
- Bottom row: Grove Needler, Ember Runner, Ember Caster.
- Style: original hand-painted isometric-ish RTS sprites.
- Background: flat `#ff00ff` chroma-key.

After generation, the atlas was copied into:

```text
assets/art/units/unit_atlas_source.png
```

Then each cell was cropped, keyed to transparency, centered on a `192x192` canvas, and saved as:

```text
assets/art/units/vanguard_guard.png
assets/art/units/vanguard_archer.png
assets/art/units/grove_bruiser.png
assets/art/units/grove_needler.png
assets/art/units/ember_runner.png
assets/art/units/ember_caster.png
```

The 8-building roster expansion added five new unit sprites per race:

```text
assets/art/units/vanguard_pikeman.png
assets/art/units/vanguard_shieldbearer.png
assets/art/units/vanguard_battle_cleric.png
assets/art/units/vanguard_lancer.png
assets/art/units/vanguard_ballista.png
assets/art/units/grove_sproutling.png
assets/art/units/grove_barkguard.png
assets/art/units/grove_mire_shaman.png
assets/art/units/grove_vine_stalker.png
assets/art/units/grove_treant_colossus.png
assets/art/units/ember_spark_imp.png
assets/art/units/ember_obsidian_guard.png
assets/art/units/ember_fire_lancer.png
assets/art/units/ember_smoke_witch.png
assets/art/units/ember_cinder_engine.png
```

These were generated as three 5-sprite race atlases, then sliced into `128x128` transparent PNGs.

## Prompt Template

Use this shape when regenerating or extending unit art:

```text
Create original fantasy RTS unit sprites for Castle Lanes, arranged in a clean atlas.
Use a flat solid #ff00ff chroma-key background with no shadows, gradients, floor plane, texture, or lighting variation.
Do not use #ff00ff anywhere in the units.
Style: polished hand-painted 2D game sprites, readable at small size, Warcraft-3-inspired RTS readability but fully original designs.
Camera: 3/4 isometric-ish view from above, facing camera-right.
Composition: one full-body unit centered in each equal cell, generous padding, no overlap, no text.
Avoid: copyrighted Warcraft assets, Blizzard logos, recognizable Warcraft characters, UI, watermarks.
```

Use this shape when regenerating or extending building command icons:

```text
Create original square fantasy RTS building icons for Castle Lanes in a clean atlas.
No text, no letters, no logos.
Style: polished hand-painted fantasy RTS command icons, readable at 64x64, original designs with chunky silhouettes and high contrast.
Composition: one centered 3/4 isometric building per equal square cell, consistent bronze/dark frame, no overlap.
Palette: match race identity; Vanguard blue/gold/steel, Grove green/bark/gold, Ember red/orange/ash.
Avoid: copyrighted Warcraft assets, Blizzard logos, recognizable Warcraft buildings, copied game icons, UI labels, watermarks.
```

## Local Processing Notes

Use the bundled Codex Python runtime when the system Python lacks image libraries:

```bash
/Users/igortverdokhleb/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3
```

The crop/key process should:

- crop the atlas into equal cells,
- convert pixels near `#ff00ff` to alpha,
- lightly despill magenta edge pixels,
- trim empty transparent bounds,
- fit the subject into a consistent `192x192` transparent canvas,
- create a contact sheet for inspection.

Building icon processing should:

- crop the atlas into equal square cells,
- resize each cell to `128x128`,
- keep the generated icon frame and painted background,
- create a contact sheet for inspection.

The 8-building roster expansion added five new building icons per race:

```text
assets/art/buildings/vanguard_pike_yard.png
assets/art/buildings/vanguard_bulwark_hall.png
assets/art/buildings/vanguard_chapel.png
assets/art/buildings/vanguard_stables.png
assets/art/buildings/vanguard_siege_workshop.png
assets/art/buildings/grove_moss_nursery.png
assets/art/buildings/grove_bark_bastion.png
assets/art/buildings/grove_mire_pool.png
assets/art/buildings/grove_vine_warren.png
assets/art/buildings/grove_ancient_seed.png
assets/art/buildings/ember_spark_kennel.png
assets/art/buildings/ember_obsidian_gate.png
assets/art/buildings/ember_blaze_stable.png
assets/art/buildings/ember_smoke_altar.png
assets/art/buildings/ember_inferno_engine.png
```

## Client Integration

The Bevy client loads unit sprites in `UnitSpriteAssets` and maps them by `UnitKind`.

When adding a new unit:

1. Add the PNG under `assets/art/units/`.
2. Add a handle to `UnitSpriteAssets`.
3. Load it in `setup`.
4. Map it in `unit_sprite_handle`.
5. Tune its display size in `unit_sprite_size`.
6. Add or update the corresponding `UnitKind` and `config/balance.json` entry.

When adding a new building icon:

1. Add the PNG under `assets/art/buildings/`.
2. Add a handle to `BuildingIconAssets`.
3. Load it in `setup`.
4. Map it in `building_icon_handle`.
5. Make sure the building exists in `config/balance.json`.

## Animation Approach

Current animation is lightweight and code-driven:

- idle bob/squash is derived from snapshot tick and unit id,
- team direction flips the sprite horizontally,
- combat readability uses inferred snapshot deltas for floating damage numbers, attack streaks, and hit bursts.

Future richer animation can use sprite sheets or texture atlases per unit, but the current approach keeps the networking protocol unchanged and works with static PNGs.

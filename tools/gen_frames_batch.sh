#!/bin/bash
# Batch-generate unit frame atlases via `codex exec`. See docs/kb/image-generation.md.
# Usage: tools/gen_frames_batch.sh   (edit ITEMS below)
set -u
cd "$(dirname "$0")/.."
mkdir -p assets/art/units/atlases_raw

ITEMS=(
  "grove_bruiser|a broad plant-blessed brawler with bark-plated fists, green-and-brown tones"
  "grove_needler|a thorny plant creature firing needle darts, green tones"
  "grove_sproutling|a tiny sproutling plant creature with a leafy head, bright green tones"
  "grove_barkguard|a massive guardian plated in thick living bark, dark green and brown"
  "grove_mire_shaman|a swamp shaman in green-teal robes with a gnarled staff"
  "grove_vine_stalker|a lean stalker wrapped in vines with clawed limbs, dark green"
  "grove_treant_colossus|a towering treant colossus of wood and leaves"
  "ember_runner|a fiery swift humanoid trailing embers, red-orange tones"
  "ember_caster|a fire mage in red-and-ash robes hurling a flame orb"
  "ember_spark_imp|a small imp made of crackling sparks, red and yellow"
  "ember_obsidian_guard|a guard in black obsidian armor with glowing red seams"
  "ember_fire_lancer|a lancer with a flaming spear in red-and-orange armor"
  "ember_smoke_witch|a witch wreathed in smoke with a curved staff, ash-grey and red"
  "ember_cinder_engine|a wheeled siege engine flinging cinders from a furnace belly"
)
for item in "${ITEMS[@]}"; do
  name="${item%%|*}"
  desc="${item#*|}"
  out="assets/art/units/atlases_raw/${name}_frames.png"
  if [ -f "$out" ]; then echo "skip $name (exists)"; continue; fi
  echo "=== generating $name ==="
  codex exec --sandbox workspace-write "Use the built-in image generation tool to create an original fantasy RTS sprite atlas for a game.

Layout: one single row of exactly 5 equal-sized cells (any aspect ratio is fine) on a flat solid #ff00ff background (magenta chroma key; no gradients, no shadows, no floor plane, no grid lines, no text).

Character: \"$name\" - $desc, 3/4 isometric view facing camera-right, polished hand-painted RTS style, readable at small size, Warcraft-3-inspired but fully original (no copyrighted characters).

Frames left to right: (1) idle stance, (2) walking mid-step, (3) attack wind-up weapon raised, (4) attack strike weapon swung/fired, (5) death, collapsed on the ground.

The SAME identical character in all 5 frames: identical armor, colors, proportions, scale and ground line; only the pose changes. Generous padding around each cell.

Save the generated image directly to $out without any post-processing - do not crop, rearrange, or edit it afterwards, the caller handles slicing." > "/tmp/codex_${name}.log" 2>&1
  status=$?
  echo "=== $name exit: $status"
  if [ $status -ne 0 ] || [ ! -f "$out" ]; then
    echo "=== $name FAILED - aborting batch" >&2
    exit 1
  fi
done
echo "BATCH COMPLETE"

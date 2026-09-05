#!/bin/bash
# Branch-unit frame atlases (batch 2). See docs/kb/image-generation.md.
set -u
cd "$(dirname "$0")/.."
mkdir -p assets/art/units/atlases_raw
ITEMS=(
  "grove_brambleguard|a regeneration-guardian plated in thorny brambles, green tones"
  "grove_spitefang|a snarling spite-fanged beast of vines and thorns, dark green"
  "ember_magma_brute|a hulking brute of cooled magma with glowing cracks, black and orange"
  "ember_ash_stalker|a lean stalker of grey ash and cinders with glowing eyes"
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

Frames left to right: (1) idle stance, (2) walking mid-step, (3) attack wind-up weapon raised, (4) attack strike weapon swung, (5) death, collapsed on the ground.

The SAME identical character in all 5 frames: identical armor, colors, proportions, scale and ground line; only the pose changes. Generous padding around each cell.

Save the generated image directly to $out without any post-processing - do not crop, rearrange, or edit it afterwards, the caller handles slicing." > "/tmp/codex_${name}.log" 2>&1
  echo "=== $name exit: $?"
done
echo "BATCH2 COMPLETE"

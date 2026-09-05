#!/bin/bash
# Batch-generate Vanguard unit frame atlases via `codex exec` (built-in image
# tool). See docs/kb/image-generation.md. macOS bash 3.2 compatible.
set -u
cd "$(dirname "$0")/.."
mkdir -p assets/art/units/atlases_raw

ITEMS=(
  "vanguard_archer|a nimble human archer in blue-and-leather light armor with a longbow and quiver"
  "vanguard_pikeman|a human pikeman in blue half-plate armor bristling with a long pike"
  "vanguard_shieldbearer|a broad human shieldbearer in heavy blue-and-steel plate behind a towering tower shield"
  "vanguard_battle_cleric|a human battle cleric in white-and-blue robes with gold trim, wielding a war mace and holy tome"
  "vanguard_lancer|a human lancer in blue-and-steel scale armor wielding a long lance"
  "vanguard_ballista|a wooden-and-steel siege ballista on a wheeled frame operated by a single blue-uniformed human engineer"
  "vanguard_arbalester|a human crossbowman in blue-and-steel light plate with a heavy arbalest crossbow"
)
for item in "${ITEMS[@]}"; do
  name="${item%%|*}"
  desc="${item#*|}"
  out="assets/art/units/atlases_raw/${name}_frames.png"
  if [ -f "$out" ]; then echo "skip $name (exists)"; continue; fi
  echo "=== generating $name ==="
  codex exec --sandbox workspace-write "Use the built-in image generation tool to create an original fantasy RTS sprite atlas for a game.

Layout: one single row of exactly 5 equal-sized square cells on a flat solid #ff00ff background (magenta chroma key; no gradients, no shadows, no floor plane, no grid lines, no text).

Character: \"$name\" - $desc, 3/4 isometric view facing camera-right, polished hand-painted RTS style, readable at small size, Warcraft-3-inspired but fully original (no copyrighted characters).

Frames left to right: (1) idle stance, (2) walking mid-step, (3) attack wind-up weapon raised, (4) attack strike weapon swung/fired, (5) death, collapsed on the ground.

The SAME identical character in all 5 frames: identical armor, colors, proportions, scale and ground line; only the pose changes. Generous padding around each cell.

Save the final image exactly to $out" > "/tmp/codex_${name}.log" 2>&1
  status=$?
  echo "=== $name exit: $status"
  if [ $status -ne 0 ] || [ ! -f "$out" ]; then
    echo "=== $name FAILED - aborting batch" >&2
    exit 1
  fi
done
echo "BATCH COMPLETE"

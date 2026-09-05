# Screenshot capture

Two routes, different permission footprints (as of 2026-09-05):

## Route A — native `screencapture` from the build shell (works now)

The shell ZCode spawns inherits the host app's Screen Recording TCC grant.
After the user grants **Screen Recording → ZCode Computer Use** in System
Settings, plain `screencapture` succeeds from agent Bash sessions — no ZCode
restart needed for this route:

```bash
screencapture -x /tmp/full_screen.png   # -x = no capture sound
```

Capture a specific game window: get its bounds via System Events, then crop
with the Pillow venv (see [image-generation.md](image-generation.md) for the
venv path):

```bash
osascript -e 'tell application "System Events" to get {position, size} of window 1 of (first process whose name is "castle_lanes_client")'
/Users/igortverdokhleb/.zcode/skills/imagegen/.venv/bin/python - <<'EOF'
from PIL import Image
img = Image.open("/tmp/full_screen.png")
img.crop((x, y, x + w, y + h)).save("docs/screenshots/battle.png")
EOF
```

## Route B — the MCP `screenshot` tool (needs ZCode restart)

The `mcp__computer-use__screenshot` tool keeps returning
`Screen Recording is denied for ZCode` until ZCode is **fully quit and
reopened** after the grant (the helper restarts with fresh TCC state). Until a
restart happens, use Route A.

## What to capture for the README (plan.md Phase 0 definition of done)

1. Lobby/browser view (client started, before joining).
2. Mid-battle with both sides' waves clashing — team colors, health bars,
   projectiles visible.
3. A Vanguard ranged volley showing the bolt projectiles.
4. Help overlay (`H`) and settings panel (`O`) open.

Launch recipe: `task run-server` + `task run-client` (or `task run-demo-client`
+ `task run-bot` for an unattended match), fast balance
(`/tmp/balance_fast.json` pattern from the smoke tests) if waves should clash
quickly.

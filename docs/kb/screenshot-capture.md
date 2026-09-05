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

## Route B — the MCP `screenshot` tool (needs manual add + ZCode restart)

The helper app (`ZCode Computer Use.app`, bundle `dev.zcode.cua-helper`) may
never appear in the Screen Recording list on its own: it only *pre-flights*
its permission (returning "denied") instead of triggering the macOS consent
dialog. Manual add, verified path:

1. Open the pane: `open "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"`
2. Click **+** under the app list, press **Cmd+Shift+G**, paste:
   `/Users/igortverdokhleb/.zcode/computer-use`
3. Select `ZCode Computer Use.app`, Open, toggle ON.
4. Fully quit ZCode (Cmd+Q) and reopen — the helper restarts with the grant.

`tccutil reset ScreenRecording dev.zcode.cua-helper` fails with "Failed to
reset" while no entry exists — harmless, but it cannot force the prompt
either. Until Route B works, use Route A.

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

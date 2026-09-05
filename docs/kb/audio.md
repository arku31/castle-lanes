# Procedural audio

All game audio is synthesized — no external assets, no license risk.

## Regenerate everything

```bash
python3 tools/gen_sfx.py     # writes assets/audio/*.wav (22.05 kHz mono)
```

Stdlib only (`wave`/`math`/`random`, deterministic seed). Sounds:
`ui_click`, `build_place`, `build_error`, `unit_spawn`, `melee_hit`,
`ranged_shot`, `unit_death`, `bounty_coin`, `castle_alarm`, `victory`,
`defeat`, `ambient_loop` (quiet drone bed, loops seamlessly via a crossfade).

## Client wiring (src/bin/client/audio.rs)

- `SfxQueue` resource with per-sound cooldowns (`Sfx::cooldown()`) so battle
  noise stays a rumble — melee/ranged hits cap at 70 ms spacing.
- `play_sfx_queue` drains the queue each frame and spawns
  `AudioPlayer + PlaybackSettings::DESPAWN` entities, gain =
  `Sfx::gain() × ClientSettings.effective_volume()`.
- Triggers: placement, command card, lobby/race/ready clicks, unit spawns,
  combat inference (melee vs ranged by attacker mode), visible deaths,
  castle damage alarm, bounty coins, GameOver stingers (victory/defeat by
  viewer side).
- `V` mutes; `O` opens settings; volume persists to
  `config/client_settings.json`.

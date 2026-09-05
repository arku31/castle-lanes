# Counter Chains — role interaction graph (Phase 2 item 1)

Design contract from plan.md §Phase 2: every unit must **beat something
specific, be beaten by something specific, and have a reason to exist beyond
raw DPS efficiency**. Armor typing is only one axis; chains must close so no
link is unbeatable. This doc is the acceptance reference for the ability
system (Phase 2 item 2): an ability ships only when it makes a chain in this
document resolve the way the doc says.

## Chains (the loops that must hold)

1. Heavy frontline → loses to magic
2. Magic casters → lose to fast divers
3. Divers → lose to cheap melee / control
4. Cheap swarm → loses to AoE splash
5. AoE backline → loses to long-range / assassins
6. Healing → loses to burst / focus fire
7. Ranged mass → loses to gap-closers and splash
8. Regen walls → lose to burst and to out-reounding (splash + focus)

Verification: `docs/balance-report.md` + composition-vs-composition runs of
the harness once bots can build counter-compositions (tracked in Phase 2).

## Role assignment of the current roster

Side geometry note: every race spans all roles so counters stay in-faction
for 1v1 mirror play. "Beats / Loses to" references roles, not units, so the
graph stays valid as units are added.

### Vanguard (blue/gold/steel — steady frontline, flexible, forgiving)

| Unit | Role | Beats | Loses to | Ability |
|---|---|---|---|---|
| Guard | Melee frontline (cheap) | Ranged mass in melee, swarm trading | Magic, splash | — |
| Pikeman | Melee DPS | Divers (mid cost wall) | Magic, slow | — |
| Shieldbearer | Tank/anchor | Burst absorption vs pierce-heavy | Heal-piercing focus, splash | — |
| Archer | Ranged DPS (light) | Flying? n/a — swarm kiting | Divers, splash | — |
| Battle Cleric | Healer | Sustains frontline vs attrition | Burst/focus fire, anti-heal | **heal_pulse** |
| Lancer | Melee gap-closer (fast, heavy) | Ranged backline, casters | Swarm surrounds, slow | — |
| Ballista | Siege / splash backline | Swarm, buildings, castle | Divers (fragile), assassins | **splash** |
| Range Tower | Ranged DPS producer | Pressure while protected | Divers | — |

Chains: Lancer (diver) beats Archer/Mage-class ranged; Ballista splash beats
swarm that beats Lancer; Cleric healing beats attrition but dies to focus
fire (chain 6).

### Grove (green/bark — cheap swarm, durable, sustain)

| Unit | Role | Beats | Loses to | Ability |
|---|---|---|---|---|
| Sproutling | Cheap swarm | Slow heavies via numbers | AoE splash | — |
| Bruiser | Melee DPS (mid) | Light ranged in melee | Magic, splash | — |
| Vine Stalker | Fast diver | Ranged backline, healers | Cheap melee wall | — |
| Needler | Ranged DPS (long range) | Kiting slow melee | Divers, gap closers | — |
| Barkguard | Regen wall | Attrition vs non-burst | Burst, focus fire | **regeneration** |
| Mire Shaman | Debuffer (slow) | Fast divers/heavies via control | Burst before slow lands | **slow** |
| Treant Colossus | Power unit / regen wall | Everything in small numbers | Burst + splash combined | **regeneration** |
| Thorn Spire | Ranged producer | Lane pressure | Divers | — |

Chains: Sproutling swarm → splash (Ballista/Cinder Engine); Barkguard/Treant
regen walls lose to burst + focus (chain 8); Mire slow is Grove's answer to
divers (chain 3), but the Shaman itself is fragile (chain 2-ish).

### Ember (red/orange — cheap fast pressure, burst, snowball)

| Unit | Role | Beats | Loses to | Ability |
|---|---|---|---|---|
| Runner | Fast swarm DPS | Slow heavies, backlines | AoE splash; berserk punishes half-kills | **berserk** |
| Spark Imp | Fastest swarm | Ranged backline | Splash, any melee wall | — |
| Caster | Magic burst | Heavy frontlines | Fast divers | — |
| Obsidian Guard | Heavy frontline | Pierce-heavy armies | Magic, swarm attrition | — |
| Fire Lancer | Fast melee DPS | Ranged mass | Slow, cheap melee wall | — |
| Smoke Witch | Debuffer (slow) + magic | Divers, heavies | Burst, out-range | **slow** |
| Cinder Engine | Siege / splash | Swarm, buildings, castle | Divers | **splash** |
| Cinder Pit | Fast producer | Tempo | — | — |

Chains: Runner berserk (chain 2/7 — divers punish casters/ranged; berserk
means half-killing a Runner costs you the diver's HP); Cinder splash answers
swarm (chain 4) but dies to divers (chain 5). Ember's observed harness
dominance (docs/balance-report.md) is the swarm-open chain; counters are
splash + typed damage, which Phase 2 bots must learn to build.

## Deliberate gaps (future content hooks)

- **Anti-heal** — adds a chain-6 counter; first candidate is a Vanguard
  upgrade (Pyromancer-style branching from the Chapel).
- **Assassin/anti-backline for Grove** — Vine Stalker is close; a true
  assassin closes chain 5 inside Grove.
- **Commander powers** (optional, Phase 2 item 5) must not break these
  chains: actives amplify or shield, they do not delete the rock-paper
  resolution.

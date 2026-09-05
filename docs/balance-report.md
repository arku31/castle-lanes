# Balance Report (generated)

Command: `cargo run --release --example sim_balance -- --games 25 --archetype counter > docs/balance-report.md`
Config: `config/balance.json` incl. branch upgrades (besieged regen 8s, regen off in sudden death;
sudden death 480s, ramp 1.0/min + escalation 8/min)

## Findings (2026-09-05, upgrade-aware counter bots)

- **Branch upgrades are live in telemetry**: Ash Pack (Ember) 143/100,
  Spitefen (Grove) 111/100, Arbalest Tower (Vanguard) 67/100, and the
  corresponding branch units appear in unit usage.
- **BALANCE FLAG - Vanguard branch is a trap**: with upgrades enabled,
  Vanguard loses every decisive non-mirror matchup (0% vs Grove and Ember)
  while Grove beats Ember 96%. The counter bots' preferred Arbalester
  (fragile 90 HP Light sniper) appears to lose the game for whoever builds
  it, and Spitefang (berserk) + Ash Stalker (slow diver) look very strong.
  Next: reduce Arbalester fragility or re-price the branch, then re-run.
  Caveat: bots are single-strategy; human play may differ - but a 0/100
  cell is a loud signal, not noise.
- Pacing vs counter bots: p50 ~6.4 min, p90 ~10.2 min, 0 unfinished.
- Lane flips ~12/match, credited to cheap wave-core units.

---

    Finished `release` profile [optimized] target(s) in 0.18s
     Running `target/release/examples/sim_balance --games 25 --archetype counter`
sim_balance: 25 games per pairing, archetype 'counter', balance: embedded default

== Left-side win rate, decisive games only (rows = left race, cols = right race; '-' = no decisive games) ==
            Vanguard     Grove     Ember
  Vanguard    40.0%     0.0%     0.0%
     Grove        -    48.0%    96.0%
     Ember        -        -    48.0%

== Pacing ==
matches: 150 | length p10 267s p50 382s p90 612s | adjudicated/unfinished: 0 | first castle damage avg 244s

== Lane dynamics ==
lane flips per match: avg 10.65 | total 1597 across 150 matches
flip credits (kinds spawned within 8s before a flip), per 100 matches:
                 Guard    824.0
            Sproutling    609.3
           Ash Stalker    292.7
             Spitefang    290.7
               Needler    251.3
           Fire Lancer    249.3
                Runner    238.0
               Bruiser     79.3

== Branch upgrades per 100 matches ==
        Ash Pack/Ember    142.7
        Spitefen/Grove    110.7
Arbalest Tower/Vanguard     66.7

== Unit builds per 100 matches (lowest first) ==
                Archer     66.7
               Bruiser    297.3
            Arbalester    550.0
           Fire Lancer    619.3
                Runner    681.3
           Ash Stalker    814.0
               Needler    873.3
             Spitefang   1020.0
            Sproutling   2312.7
                 Guard   4134.0

# Balance Report (generated)

Command: `cargo run --release --example sim_balance -- --games 25 --archetype counter > docs/balance-report.md`
Config: `config/balance.json` (besieged regen 8s delay, regen off in sudden death;
sudden death 480s, ramp 1.0/min + escalation 8/min)

## Findings (2026-09-05, counter-aware bots + abilities + branch units)

- COUNTER BOTS CHANGE THE PICTURE: the earlier "Ember wins 100%" finding was a
  bot-meta artifact. Against counter-aware bots (typed-damage scoring vs the
  enemy's dominant armor, splash preference vs swarms), Ember's non-mirror
  dominance collapses: Grove beats Ember 70%, Vanguard beats Ember 100% (both
  with Ember on the right side; sample 25/pairing). Vanguard's own mirror
  favoritism (36-60%) is within small-sample noise.
- Lane-flip metric is live: avg ~12 flips per match, credited mostly to cheap
  wave-core units (Guard/Sproutling/Runner) - pressure flips follow wave
  pushes, which is exactly the counter-building dynamic the plan wants
  measurable. Upgrade-branch units do not yet appear (bots cannot upgrade).
- Pacing vs counter bots: p50 ~5.2 min, p90 ~10 min, 0 unfinished - slightly
  longer than mixed bots because counter-building stabilizes lanes. Still
  inside the 6-12 min human band.
- Next: teach the branch upgrade to counter-bots so the 6 new buildings appear
  in telemetry; then re-check Ember with splash-heavy boards.

---

   Compiling castle_lanes v0.1.0 (/Volumes/Projects/cf6)
    Finished `release` profile [optimized] target(s) in 5.38s
     Running `target/release/examples/sim_balance --games 25 --archetype counter`
sim_balance: 25 games per pairing, archetype 'counter', balance: embedded default

== Left-side win rate, decisive games only (rows = left race, cols = right race; '-' = no decisive games) ==
            Vanguard     Grove     Ember
  Vanguard    48.0%   100.0%   100.0%
     Grove        -    56.0%    80.0%
     Ember        -        -    56.0%

== Pacing ==
matches: 150 | length p10 232s p50 298s p90 602s | adjudicated/unfinished: 0 | first castle damage avg 271s

== Lane dynamics ==
lane flips per match: avg 11.42 | total 1713 across 150 matches
flip credits (kinds spawned within 8s before a flip), per 100 matches:
                 Guard   1044.7
            Sproutling    630.7
                Runner    604.7
               Bruiser    384.7
                Archer    266.0
               Needler    262.0
           Fire Lancer    194.7

== Unit builds per 100 matches (lowest first) ==
           Fire Lancer    491.3
               Needler    802.7
                Archer    917.3
               Bruiser   1116.7
                Runner   2442.0
            Sproutling   3891.3
                 Guard   5748.0

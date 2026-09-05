# Balance Report (generated)

Command: `cargo run --release --example sim_balance -- --games 25 > docs/balance-report.md`
Config: `config/balance.json` (besieged castle regen: 8s delay; sudden death 480s, ramp 1.0/min)

## Findings (2026-09-05, plan.md P0-5)

- Pacing vs 'mixed' bots: p50 ~4.4 min, p90 ~10.0 min, zero unfinished at a 25 min cap.
  Bots do not defend adaptively, so human matches will run longer; the 6-12 min band is
  the human target, bots give the relative signal.
- Ember wins 100% of decisive non-mirror games under cheap-swarm bot play. This is a
  bot-meta artifact (rotation bots under-use typed damage counters), but it flags Ember's
  cheap fast units as the strongest unadaptive pressure - revisit when abilities land in
  Phase 2 and re-check with counter-aware bots.
- Expensive units (Vine Stalker, Archer, Needler, Pikeman tiers) are rarely built by the
  rotation bot because they are seldom affordable; unit-usage rows are bot-behavior
  diagnostics, not global meta claims.

---

   Compiling castle_lanes v0.1.0 (/Volumes/Projects/cf6)
    Finished `release` profile [optimized] target(s) in 5.27s
     Running `target/release/examples/sim_balance --games 25`
sim_balance: 25 games per pairing, archetype 'mixed', balance: embedded default

== Left-side win rate, decisive games only (rows = left race, cols = right race; '-' = no decisive games) ==
            Vanguard     Grove     Ember
  Vanguard    40.0%   100.0%     0.0%
     Grove        -    56.0%     0.0%
     Ember        -        -    32.0%

== Pacing ==
matches: 150 | length p10 210s p50 262s p90 601s | adjudicated/unfinished: 0 | first castle damage avg 266s

== Unit builds per 100 matches (lowest first) ==
          Vine Stalker      1.3
                Archer     34.0
               Needler     41.3
               Pikeman    356.7
             Spark Imp   1016.7
               Bruiser   1528.0
            Sproutling   3610.0
                 Guard   3644.0
                Runner   6048.7

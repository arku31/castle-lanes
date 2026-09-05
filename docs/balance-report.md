# Balance Report (generated)

Command: `cargo run --release --example sim_balance -- --games 25 > docs/balance-report.md`
Config: `config/balance.json` (besieged regen: 8s delay, regen off during sudden death;
sudden death 480s, ramp 1.0/min + flat escalation 8/min)

## Findings (2026-09-05, plan.md P0-5 + P1 pacing fixes)

- Pacing vs 'mixed' bots: p50 ~4.4 min, p90 ~9.8 min, zero unfinished at a 25 min cap.
  Bots do not defend adaptively, so human matches will run longer; the 6-12 min band is
  the human target, bots give the relative signal.
- Sudden death now guarantees resolution even for near-empty boards (flat escalation
  8/min plus board-pressure ramp, regen disabled during sudden death). A passive
  one-building-per-side stall that previously ran 20+ minutes now resolves.
- Ember wins 100% of decisive non-mirror games under cheap-swarm bot play. This is a
  bot-meta artifact (rotation bots under-use typed damage counters), but it flags Ember's
  cheap fast units as the strongest unadaptive pressure - revisit when abilities land in
  Phase 2 and re-check with counter-aware bots.
- Expensive units are rarely built by the rotation bot because they are seldom
  affordable; unit-usage rows are bot-behavior diagnostics, not global meta claims.

---

    Finished `release` profile [optimized] target(s) in 0.49s
     Running `target/release/examples/sim_balance --games 25`
sim_balance: 25 games per pairing, archetype 'mixed', balance: embedded default

== Left-side win rate, decisive games only (rows = left race, cols = right race; '-' = no decisive games) ==
            Vanguard     Grove     Ember
  Vanguard    36.0%   100.0%     0.0%
     Grove        -    52.0%     0.0%
     Ember        -        -    32.0%

== Pacing ==
matches: 150 | length p10 210s p50 262s p90 589s | adjudicated/unfinished: 0 | first castle damage avg 266s

== Unit builds per 100 matches (lowest first) ==
          Vine Stalker      1.3
                Archer     21.3
               Needler     32.0
               Pikeman    320.7
             Spark Imp    944.7
               Bruiser   1478.0
            Sproutling   3507.3
                 Guard   3528.7
                Runner   5927.3

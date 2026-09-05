# Balance Report (generated)

Command: `cargo run --release --example sim_balance -- --games 25 > docs/balance-report.md`
Config: `config/balance.json` (besieged regen 8s delay, regen off in sudden death;
sudden death 480s, ramp 1.0/min + escalation 8/min)

## Findings (2026-09-05, with Phase 2 abilities live)

- First ability set shipped (plan.md Phase 2 item 2): Cleric heal_pulse,
  Ballista/Cinder Engine splash, Mire Shaman/Smoke Witch slow, Barkguard/Treant
  regeneration, Runner berserk.
- Pacing vs 'mixed' bots is unchanged by abilities (p50 4.4 min, p90 9.8 min,
  0 unfinished) - abilities currently shift skirmish outcomes, not macro pace.
- Ember still wins 100% of decisive non-mirror games under swarm-heavy bot play;
  re-check with counter-aware bots and after splash/slow tuning.
- Unit-usage rows are bot-behavior diagnostics (rotation bots under-buy
  expensive tiers), not global meta claims.

---

   Compiling castle_lanes v0.1.0 (/Volumes/Projects/cf6)
warning: method `side_alive_castles` is never used
    --> src/sim.rs:1149:8
     |
1092 | impl GameSim {
     | ------------ method in this implementation
...
1149 |     fn side_alive_castles(&self, team: Team) -> Vec<usize> {
     |        ^^^^^^^^^^^^^^^^^^
     |
     = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: `castle_lanes` (lib) generated 1 warning
    Finished `release` profile [optimized] target(s) in 4.38s
     Running `target/release/examples/sim_balance --games 25`
sim_balance: 25 games per pairing, archetype 'mixed', balance: embedded default

== Left-side win rate, decisive games only (rows = left race, cols = right race; '-' = no decisive games) ==
            Vanguard     Grove     Ember
  Vanguard    36.0%   100.0%     0.0%
     Grove        -    52.0%     0.0%
     Ember        -        -    36.0%

== Pacing ==
matches: 150 | length p10 210s p50 262s p90 589s | adjudicated/unfinished: 1 | first castle damage avg 266s

== Unit builds per 100 matches (lowest first) ==
          Vine Stalker      1.3
                Archer     21.3
               Needler     32.0
               Pikeman    320.7
             Spark Imp    960.0
               Bruiser   1478.0
            Sproutling   3507.3
                 Guard   3528.7
                Runner   5946.0

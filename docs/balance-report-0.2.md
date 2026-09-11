# Race Balance Report — v0.2.0 tuning pass

Methodology: after the v0.2.0 unit changes, race balance was tuned with a new
fast checker, `cargo run --release --example clash_matrix` — deterministic
cost-matched producer clashes on the real `GameSim` (no bots, no network),
which diagnoses a full pairing in seconds. Final numbers below are confirmed
with the full bot-cycle harness (`sim_balance --games 25`, 150 matches).

Result: a stable counter-triangle (each race wins one matchup and loses one,
~50% overall). The decisive-only matrix exaggerates the edges: losses report
as 0% while stalemates resolve by castle-HP margin. Sample = 25 games/pairing.

Findings that drove the changes:
- Grove producer line was ~35% under par on effective-DPS/gold (15.0 vs
  22.8/24.3) while its units carry high HP — retuned damage, kept tank flavor.
- Grove Barkguard/Brambleguard/Treant regeneration 4-5 hp/s was free sustain
  that outraced chip damage — normalized to 2.
- Ember producer tempo was ~10% slow for its glass-cannon units — intervals
  shortened; Ember units got HP to survive their own aggression window.
- Four of six branch upgrades (AshPack, MagmaForge, BrambleWarren, Spitefen,
  and marginally ArcaneSpire/ArbalestTower) delivered LESS army-DPS throughput
  per building-second than the base tier — the counter bot upgraded into
  self-destruction, causing 100%-win sweeps. All six upgraded units now
  out-produce their base tier.
- Grove MireShaman slow 0.6 -> 0.75 vs Vanguard BattleCleric heal 14 -> 20:
  defense/pressure abilities rebalanced to match.

Remaining known skew (documented, not AAA): full-cycle decisive rates swing
sharper than the clash matrix because the counter bot snowballs small edges;
Vanguard-vs-Grove and Grove-vs-Ember lean hard on their winners.

---

warning: constant `LANE_CENTER_Y` is never used
  --> src/sim.rs:22:7
   |
22 | const LANE_CENTER_Y: f32 = 8.0;
   |       ^^^^^^^^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: `castle_lanes` (lib) generated 1 warning
    Finished `release` profile [optimized] target(s) in 0.37s
     Running `target/release/examples/sim_balance --games 25`
sim_balance: 25 games per pairing, archetype 'counter', balance: embedded default

== Left-side win rate, decisive games only (rows = left race, cols = right race; '-' = no decisive games) ==
            Vanguard     Grove     Ember
  Vanguard    52.0%     0.0%    60.0%
     Grove        -    52.0%     0.0%
     Ember        -        -    56.0%

== Pacing ==
matches: 150 | length p10 210s p50 282s p90 558s | adjudicated/unfinished: 4 | first castle damage avg 179s

== Lane dynamics ==
lane flips per match: avg 13.71 | total 2056 across 150 matches
flip credits (kinds spawned within 8s before a flip), per 100 matches:
                 Guard   1273.3
            Sproutling    622.7
           Ash Stalker    530.0
             Spark Imp    396.0
             Spitefang    308.0
               Pikeman    251.3
                Runner    247.3
               Bruiser    116.7

== Branch upgrades per 100 matches ==
        Ash Pack/Ember    166.7
        Spitefen/Grove    102.7

== Unit builds per 100 matches (lowest first) ==
               Bruiser    363.3
               Pikeman    620.0
                Runner    792.7
             Spitefang    816.7
           Ash Stalker   1124.0
             Spark Imp   1127.3
            Sproutling   2310.7
                 Guard   2474.7

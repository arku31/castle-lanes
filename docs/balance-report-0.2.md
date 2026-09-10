warning: constant `LANE_CENTER_Y` is never used
  --> src/sim.rs:22:7
   |
22 | const LANE_CENTER_Y: f32 = 8.0;
   |       ^^^^^^^^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: `castle_lanes` (lib) generated 1 warning
    Finished `release` profile [optimized] target(s) in 0.47s
     Running `target/release/examples/sim_balance --games 25`
sim_balance: 25 games per pairing, archetype 'counter', balance: embedded default

== Left-side win rate, decisive games only (rows = left race, cols = right race; '-' = no decisive games) ==
            Vanguard     Grove     Ember
  Vanguard    36.0%     0.0%   100.0%
     Grove        -    64.0%   100.0%
     Ember        -        -    44.0%

== Pacing ==
matches: 150 | length p10 286s p50 396s p90 594s | adjudicated/unfinished: 0 | first castle damage avg 204s

== Lane dynamics ==
lane flips per match: avg 14.87 | total 2230 across 150 matches
flip credits (kinds spawned within 8s before a flip), per 100 matches:
            Sproutling   1206.0
             Spitefang    510.0
           Ash Stalker    445.3
                 Guard    419.3
               Needler    408.0
           Fire Lancer    298.7
                Runner    260.7
               Bruiser    120.7

== Branch upgrades per 100 matches ==
        Ash Pack/Ember    146.0
        Spitefen/Grove    114.0
Arbalest Tower/Vanguard     66.7

== Unit builds per 100 matches (lowest first) ==
             Spark Imp      0.7
                Archer     66.7
               Bruiser    310.0
            Arbalester    456.0
           Fire Lancer    581.3
                Runner    660.7
               Needler    897.3
           Ash Stalker    986.0
             Spitefang   1151.3
                 Guard   1504.0
            Sproutling   2086.0

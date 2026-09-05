# Balance Report (generated)

Command: `cargo run --release --example sim_balance -- --games 25 --archetype counter > docs/balance-report.md`
Config: `config/balance.json` incl. branch upgrades (besieged regen 8s, regen off in sudden death;
sudden death 480s, ramp 1.0/min + escalation 8/min)

## Findings (2026-09-06, after branch-balance iteration 1)

- Iteration applied: Arbalester buffed (90->130 HP, 2.4s interval),
  Spitefang nerfed (berserk 1.6/1.4 -> 1.45/1.3, HP 110->95, dmg 20->17),
  bots no longer over-prefer upgradeable bases.
- Result: Vanguard mirror recovered (36->60%), Vanguard vs Ember flipped
  0% -> 100% (the Arbalester buff worked), Ember mirror 48%.
- REMAINING FLAG: Grove beats Vanguard in 100% of decisive games and Ember
  in 92%. Grove's base identity (regen walls + 12.5k castle + Mire slow)
  counters the Vanguard roster as a whole. Hypotheses for iteration 2:
  reduce Grove castle HP toward 11000, or weaken Barkguard/Treant
  regeneration vs the swarm-heavy Ember matchup. Needs human playtest
  input before more bot-driven tuning.
- Upgrade telemetry: Spitefen 96-110, Ash Pack 143, Arbalest Tower 67 per
  100 matches (bot-preference dependent).
- Pacing: p50 ~6.6 min, p90 ~10.2 min, 0 unfinished. Lane flips ~12/match.

---

    Finished `release` profile [optimized] target(s) in 0.50s
     Running `target/release/examples/sim_balance --games 25 --archetype counter`
sim_balance: 25 games per pairing, archetype 'counter', balance: embedded default

== Left-side win rate, decisive games only (rows = left race, cols = right race; '-' = no decisive games) ==
            Vanguard     Grove     Ember
  Vanguard    60.0%     0.0%   100.0%
     Grove        -    48.0%    96.0%
     Ember        -        -    48.0%

== Pacing ==
matches: 150 | length p10 287s p50 400s p90 613s | adjudicated/unfinished: 1 | first castle damage avg 251s

== Lane dynamics ==
lane flips per match: avg 12.20 | total 1830 across 150 matches
flip credits (kinds spawned within 8s before a flip), per 100 matches:
                 Guard   1397.3
            Sproutling    584.7
           Ash Stalker    310.0
           Fire Lancer    278.7
             Spitefang    264.7
               Needler    248.7
                Runner    246.0
            Arbalester    155.3

== Branch upgrades per 100 matches ==
        Ash Pack/Ember    126.0
        Spitefen/Grove    108.7
Arbalest Tower/Vanguard     66.7

== Unit builds per 100 matches (lowest first) ==
                Archer     66.7
               Bruiser    295.3
           Fire Lancer    568.7
                Runner    605.3
            Arbalester    651.3
           Ash Stalker    784.7
               Needler    886.0
             Spitefang   1044.7
            Sproutling   2501.3
                 Guard   4291.3

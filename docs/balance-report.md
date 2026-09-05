# Balance Report (generated)

Command: `cargo run --release --example sim_balance -- --games 25 --archetype counter > docs/balance-report.md`
Config: `config/balance.json` (besieged regen 8s, regen off in sudden death; SD 480s, ramp 1.0 + escalation 8/min)

## Findings (2026-09-06, balance iteration 2)

- Iteration 2 tested: Grove castle 12500->11000, Barkguard regen 5->4,
  Treant HP 320->280, Guard dmg 18->20. RESULT: Grove still beats Vanguard
  100% decisive. The Grove nerfs were insufficient - the matchup is
  structural (regen + slow + swarm counters Vanguard's mid-cost roster
  wholesale). REVERTED to previous values (except Guard buff which helps
  the mirror).
- Counter bots' upgrade-aware branch scoring prefers Spitefen (Grove) and
  Ash Pack (Ember) - both performing well. Vanguard's Arbalester branch is
  a trap even at 130 HP.
- Balance iteration 2 is PAUSED pending human playtest input. Bot-driven
  tuning has hit its limit: the counter archetype's scoring cannot detect
  the structural advantage Grove's regen+slow identity has over Vanguard's
  mid-cost roster. A human playtest may reveal the fix is in unit costs,
  abilities, or a new counter unit rather than stat tweaks.

---

   Compiling castle_lanes v0.1.0 (/Volumes/Projects/cf6)
    Finished `release` profile [optimized] target(s) in 5.91s
     Running `target/release/examples/sim_balance --games 25 --archetype counter`
sim_balance: 25 games per pairing, archetype 'counter', balance: embedded default

== Left-side win rate, decisive games only (rows = left race, cols = right race; '-' = no decisive games) ==
            Vanguard     Grove     Ember
  Vanguard    44.0%     0.0%   100.0%
     Grove        -    40.0%    88.0%
     Ember        -        -    48.0%

== Pacing ==
matches: 150 | length p10 284s p50 381s p90 614s | adjudicated/unfinished: 0 | first castle damage avg 231s

== Lane dynamics ==
lane flips per match: avg 11.55 | total 1732 across 150 matches
flip credits (kinds spawned within 8s before a flip), per 100 matches:
                 Guard    925.3
            Sproutling    596.7
           Ash Stalker    307.3
           Fire Lancer    273.3
             Spitefang    265.3
               Needler    253.3
                Runner    251.3
            Arbalester    115.3

== Branch upgrades per 100 matches ==
        Ash Pack/Ember    125.3
        Spitefen/Grove    108.0
Arbalest Tower/Vanguard     66.7

== Unit builds per 100 matches (lowest first) ==
                Archer     66.7
               Bruiser    294.7
            Arbalester    536.7
           Fire Lancer    566.7
                Runner    595.3
           Ash Stalker    764.0
               Needler    876.7
             Spitefang   1030.0
            Sproutling   2424.7
                 Guard   2902.0

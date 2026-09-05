# Balance workflow

The rule: **no `config/balance.json` change merges without regenerating
`docs/balance-report.md`** (plan.md §6).

## Regenerate the report

```bash
cargo run --release --example sim_balance -- --games 25 --archetype counter \
    > docs/balance-report.md
```

Then prepend the findings header (see the current report for the format:
command, config summary, dated findings).

## Archetypes

| Archetype | Behavior |
|---|---|
| `counter` (default) | Scores producers by typed damage vs the enemy army's dominant armor; prefers splash vs swarmy boards (≥10 units) |
| `mixed` | Rotates the producer roster cost-ascending, buys econ every ~3rd build |
| `aggro` | Always the cheapest producer, front zone |
| `econ` | Buys economy buildings first, then cheapest producers |
| `tech` | Saves for the most expensive producer |

Counter bots exist to defeat bot-meta artifacts: the earlier "Ember wins
100%" finding vanished once bots built typed-damage counters (see
[balance-report.md](../balance-report.md)).

## Metrics the report carries

- Win matrix (decisive games only; `-` = none)
- Match length p10/p50/p90 + adjudicated/unfinished count + first-castle-damage time
- **Lane dynamics**: flips per match (leadership change with >15% pressure
  margin, 1 Hz sampling) + flip credits (kinds spawned within 8 s before a
  flip, per 100 matches)
- Unit builds per 100 matches (bot-behavior diagnostic, not a meta claim)

## Useful invocations

```bash
# fast smoke (CI uses this)
cargo run --release --example sim_balance -- --games 3

# alternate config (e.g. quick sudden death for manual testing)
cargo run --release --example sim_balance -- --games 5 --balance /tmp/balance_quick.json

# archetype comparison
cargo run --release --example sim_balance -- --games 25 --archetype mixed
```

Known bot limitations (do not read as player meta): rotation bots under-buy
expensive tiers, cannot upgrade buildings, and do not adapt counter choices
within a match.

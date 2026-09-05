# Packet encoding: JSON vs bincode (plan.md Phase 1 item 4)

Command:
```bash
CASTLE_LANES_FORMAT=json    cargo run --release --example sim_perf
CASTLE_LANES_FORMAT=bincode cargo run --release --example sim_perf
```

Measured 2026-09-05 (Apple Silicon, release). Wire format is tagged per
datagram (byte 0: 0 = JSON, 1 = bincode); decode auto-detects, and untagged
raw JSON still decodes for pre-migration peers. Default outgoing format is
bincode; `--legacy-json` on any binary switches that peer to JSON.

| Units | full JSON | full bincode | Δ | delta JSON | delta bincode | Δ | encode JSON | encode bincode |
|------:|----------:|-------------:|-----:|-----------:|--------------:|-----:|------------:|---------------:|
| 2     | 1,046 B   | 288 B        | 3.6x | 1,016 B    | 281 B         | 3.6x | 0.003 ms    | 0.000 ms       |
| 100   | 20,819 B  | 5,384 B      | 3.9x | 14,318 B   | 3,809 B       | 3.8x | 0.033 ms    | 0.001 ms       |
| 500   | 102,893 B | 26,184 B     | 3.9x | 69,992 B   | 18,209 B      | 3.8x | 0.080 ms    | 0.003 ms       |
| 1,000 | 204,821 B | 52,184 B     | 3.9x | 138,920 B  | 36,209 B      | 3.8x | 0.160 ms    | 0.006 ms       |
| 2,000 | 409,145 B | 104,184 B    | 3.9x | 277,244 B  | 72,209 B      | 3.8x | 0.323 ms    | 0.013 ms       |

Takeaways:

- ~3.8x smaller deltas and ~30x faster encoding at typical unit counts; at
  200 units the per-client snapshot stream drops from roughly 30 KB/s to
  8 KB/s, well inside the 30 KB/s budget from the quality bar (C9).
- Baseline (entityless) snapshots shrink 660 -> 184 B.
- Fog filtering (Phase 1 item 1) plus bincode together keep per-client
  bandwidth proportional to what a client can actually see.

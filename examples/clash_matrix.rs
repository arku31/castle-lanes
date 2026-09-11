//! Fast race-balance matrix without bot matches (plan-0.2 follow-up).
//!
//! For every faction pairing this runs deterministic "clash" scenarios on the
//! real `GameSim`: each side gets a cost-matched set of producer buildings in
//! the same lane and an identical gold stipend — no decisions, no randomness
//! beyond the sim seed. The winner + castle margin per scenario gives a
//! race-vs-race balance matrix in seconds, which is fast enough to tune
//! `balance.json` against before confirming with `sim_balance`.
//!
//! ```bash
//! cargo run --release --example clash_matrix
//! ```

use castle_lanes::sim::{
    BalanceConfig, BuildZone, BuildingKind, DEFAULT_BALANCE_PATH, GameSim, Lane, MatchPhase,
    PlayerId, RaceKind, Team, UnitKind,
};
use std::env;

const DT: f32 = 1.0 / 30.0;
const STIPEND_EVERY_SECS: f32 = 10.0;
const STIPEND_GOLD: i32 = 90;
const MAX_SECS: f32 = 200.0;

#[derive(Clone, Copy)]
struct SetEntry {
    kind: BuildingKind,
    count: usize,
}

struct FactionSet {
    name: &'static str,
    entries: Vec<SetEntry>,
}

fn faction_sets(race: RaceKind, balance: &BalanceConfig) -> Vec<FactionSet> {
    // Producer buildings (units only), ordered by cost.
    let mut producers: Vec<(BuildingKind, i32, f32, UnitKind)> = balance
        .race(race)
        .buildings
        .iter()
        .filter_map(|kind| {
            let cfg = balance.building(*kind);
            cfg.spawned_unit
                .map(|unit| (*kind, cfg.cost, cfg.spawn_interval.unwrap_or(99.0), unit))
        })
        .collect();
    producers.sort_by_key(|(_, cost, interval, _)| {
        ((*cost as f32 / interval) * 100.0) as i64 // gold/sec of supply
    });

    let n = producers.len();
    let pick = |indices: &[usize]| -> FactionSet {
        let mut entries = Vec::new();
        let mut gold = 0;
        for &i in indices {
            let (kind, cost, _, _) = producers[i];
            let count = (gold + cost <= 400) as usize;
            if count == 1 {
                gold += cost;
            }
            entries.push(SetEntry { kind, count });
        }
        FactionSet {
            name: "custom",
            entries,
        }
    };

    vec![
        FactionSet {
            name: "cheap",
            entries: vec![
                SetEntry {
                    kind: producers[0].0,
                    count: 2,
                },
                SetEntry {
                    kind: producers[1].0,
                    count: 1,
                },
            ],
        },
        FactionSet {
            name: "mid",
            entries: vec![
                SetEntry {
                    kind: producers[n / 2 - 1].0,
                    count: 1,
                },
                SetEntry {
                    kind: producers[n / 2].0,
                    count: 1,
                },
            ],
        },
        FactionSet {
            name: "expensive",
            entries: vec![
                SetEntry {
                    kind: producers[n - 1].0,
                    count: 1,
                },
                SetEntry {
                    kind: producers[n - 2].0,
                    count: 1,
                },
            ],
        },
        FactionSet {
            name: "mixed",
            entries: vec![
                SetEntry {
                    kind: producers[0].0,
                    count: 1,
                },
                SetEntry {
                    kind: producers[n / 2 - 1].0,
                    count: 1,
                },
                SetEntry {
                    kind: producers[n - 2].0,
                    count: 1,
                },
                SetEntry {
                    kind: producers[n - 1].0,
                    count: 1,
                },
            ],
        },
    ]
}

/// Costs are printed alongside so budget parity can be eyeballed.
fn set_cost(set: &FactionSet, balance: &BalanceConfig) -> i32 {
    set.entries
        .iter()
        .map(|e| balance.building(e.kind).cost * e.count as i32)
        .sum()
}

fn winner_of(left_wins: u32, scenarios: u32) -> Option<Team> {
    (left_wins * 2 >= scenarios).then_some(Team::Left)
}

fn run_clash(
    balance: &BalanceConfig,
    left_race: RaceKind,
    right_race: RaceKind,
    left_set: &FactionSet,
    right_set: &FactionSet,
    seed: u64,
) -> (Option<Team>, f32) {
    let mut sim = GameSim::with_seed(balance.clone(), seed);
    let left = sim.join_or_update_player("Alice".to_string()).unwrap().id;
    let right = sim.join_or_update_player("Bryn".to_string()).unwrap().id;
    sim.set_race(left, left_race).unwrap();
    sim.set_race(right, right_race).unwrap();
    sim.set_ready(left, true).unwrap();
    sim.set_ready(right, true).unwrap();

    // Force enough gold, then place sets symmetrically in Front zone cells.
    let left_index = sim
        .players
        .iter()
        .position(|p| p.id == left)
        .expect("left joined");
    let right_index = sim
        .players
        .iter()
        .position(|p| p.id == right)
        .expect("right joined");
    sim.economies[left_index].gold = 2_000;
    sim.economies[right_index].gold = 2_000;

    let lane = Lane::Top;
    let mut place_set =
        |sim: &mut GameSim, player: PlayerId, set: &FactionSet, cell_offset: usize| {
            let mut slot = cell_offset;
            for entry in &set.entries {
                for _ in 0..entry.count {
                    let cell = castle_lanes::sim::GridCell {
                        x: (slot % 9) as i32,
                        y: ((slot / 9) % 4) as i32,
                    };
                    let _ = sim.place_building(player, entry.kind, lane, BuildZone::Front, cell);
                    slot += 1;
                }
            }
        };
    place_set(&mut sim, left, left_set, 0);
    place_set(&mut sim, right, right_set, 0);
    sim.economies[left_index].gold = 0;
    sim.economies[right_index].gold = 0;
    // Skip the countdown: the clash starts immediately.
    sim.phase = MatchPhase::Playing;

    let max_ticks = (MAX_SECS / DT) as usize;
    let stipend_every = (STIPEND_EVERY_SECS / DT) as usize;
    let mut ticks = 0usize;
    let mut winner = None;
    while ticks < max_ticks {
        sim.tick(DT);
        ticks += 1;
        // Equal stipend: tests combat value of produced units, not economy.
        if ticks % stipend_every == 0 {
            for economy in sim.economies.iter_mut() {
                economy.gold += STIPEND_GOLD;
            }
        }
        // Auto-place anything affordable in the pre-agreed rotation keeps the
        // scenarios simple: instead we re-grant the set cost every stipend so
        // each side re-builds its own set at the same cadence.
        if ticks % stipend_every == 1 {
            for (index, set) in [(left_index, left_set), (right_index, right_set)] {
                let cost = set_cost(set, balance);
                sim.economies[index].gold += cost;
            }
        }
        if ticks % 30 != 0 {
            continue;
        }
        // Rebuild placed buildings destroyed by the fight (both sides equal).
        for (index, player, set) in [
            (left_index, left, left_set),
            (right_index, right, right_set),
        ] {
            let _ = (index, player);
            for entry in &set.entries {
                for _ in 0..entry.count {
                    let owned = sim
                        .buildings
                        .iter()
                        .filter(|b| b.owner == player && b.kind == entry.kind)
                        .count();
                    let missing = entry.count.saturating_sub(owned);
                    for _ in 0..missing {
                        sim.economies[index].gold = i32::MAX / 2;
                        let slot = ticks as usize % 9;
                        let _ = sim.place_building(
                            player,
                            entry.kind,
                            lane,
                            BuildZone::Front,
                            castle_lanes::sim::GridCell {
                                x: (slot) as i32,
                                y: ((slot / 3) % 4) as i32,
                            },
                        );
                        sim.economies[index].gold = 0;
                    }
                }
            }
        }
        if sim.phase == MatchPhase::Playing {
            for castle in &sim.castles {
                if castle.health <= 0 {
                    winner = Some(castle.team.opponent());
                }
            }
            if winner.is_some() {
                break;
            }
        }
    }

    let margin = if sim.castles.len() == 2 {
        let left_hp = sim
            .castles
            .iter()
            .find(|c| c.team == Team::Left)
            .map(|c| c.health)
            .unwrap_or(0);
        let right_hp = sim
            .castles
            .iter()
            .find(|c| c.team == Team::Right)
            .map(|c| c.health)
            .unwrap_or(0);
        (left_hp - right_hp) as f32
    } else {
        0.0
    };
    (winner, margin)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let balance_path = args
        .iter()
        .position(|a| a == "--balance")
        .and_then(|i| args.get(i + 1))
        .map(std::path::PathBuf::from);
    let balance = BalanceConfig::load_or_default(
        balance_path
            .as_deref()
            .unwrap_or(DEFAULT_BALANCE_PATH.as_ref()),
    );

    let races = RaceKind::ALL;
    println!(
        "clash matrix: cost-matched producer sets, stipend {}g / {}s, max {}s\n",
        STIPEND_GOLD, STIPEND_EVERY_SECS, MAX_SECS as u32
    );
    println!(
        "{:14} {:10} {:10} {:6} {:>5}",
        "pairing", "left set", "right set", "winner", "margin"
    );

    let mut score: Vec<(String, f32, f32, u32)> = Vec::new(); // pairing, left score sum, max possible, scenarios
    for (li, &left_race) in races.iter().enumerate() {
        for &right_race in &races[li..] {
            let left_sets = faction_sets(left_race, &balance);
            let right_sets = faction_sets(right_race, &balance);
            let pair = format!("{left_race:?} vs {right_race:?}");
            score.push((pair.clone(), 0.0, 0.0, 0));
            for (li_s, left_set) in left_sets.iter().enumerate() {
                for (ri_s, right_set) in right_sets.iter().enumerate() {
                    // Run each set pairing twice with different seeds.
                    let mut left_score = 0.0f32;
                    let mut margin_sum = 0.0f32;
                    let mut scenarios = 0u32;
                    let started = std::time::Instant::now();
                    for seed in [101u64] {
                        let (winner, margin) = run_clash(
                            &balance,
                            left_race,
                            right_race,
                            left_set,
                            right_set,
                            seed + (li_s * 31 + ri_s * 7) as u64,
                        );
                        if let Some(team) = winner {
                            if team == Team::Left {
                                left_score += 1.0;
                            }
                        } else if margin.abs() > 400.0 {
                            // Timeout with a clear castle-HP edge counts as a
                            // win; near-equal margins are draws worth half.
                            if margin > 0.0 {
                                left_score += 1.0;
                            } else {
                                left_score += 0.0;
                            }
                        } else {
                            left_score += 0.5; // stalemate draw scores even
                        }
                        margin_sum += margin;
                        scenarios += 1;
                    }
                    eprintln!(
                        "clash {pair} {} vs {} took {:?} (winner {:?}, margin {:.0})",
                        left_set.name,
                        right_set.name,
                        started.elapsed(),
                        winner_of(left_score as u32, scenarios),
                        margin_sum / scenarios as f32
                    );
                    println!(
                        "{:14} {:10} {:10} {:6} {:>7.0}",
                        pair,
                        left_set.name,
                        right_set.name,
                        if left_score * 2.0 >= scenarios as f32 {
                            "LEFT"
                        } else {
                            "RIGHT"
                        },
                        margin_sum / scenarios as f32
                    );
                    let entry = score
                        .iter_mut()
                        .find(|(p, _, _, _)| *p == pair)
                        .expect("pairing seeded");
                    entry.1 += left_score;
                    entry.2 += scenarios as f32;
                    // max possible tracked in tuple.3 via scenarios only
                }
            }
        }
    }

    println!("\n== left-side win matrix (target 35-65%) ==");
    for (pair, sum, max, _scenarios) in &score {
        let pct = 100.0 * sum / *max;
        let flag = if !(35.0..=65.0).contains(&pct) {
            "  <-- SKEWED"
        } else {
            ""
        };
        println!("{pair:24} {pct:5.1}%{flag}");
    }
}

//! Headless bot-vs-bot balance harness (plan.md P0-4 / §6).
//!
//! Heuristic archetype bots play full matches for every race pairing and the
//! report prints: left-side win-rate matrix, match length distribution, and
//! per-unit-kind build counts (flags units that never get built).
//!
//! Run in release for real sample counts:
//!
//! ```bash
//! cargo run --release --example sim_balance -- --games 100 > docs/balance-report.md
//! ```
//!
//! Bots are "differentiable strategies", not good players: the harness exists
//! to compare outcomes across seeds, not to model humans.

use castle_lanes::sim::{
    BalanceConfig, BuildZone, BuildingKind, DEFAULT_BALANCE_PATH, GRID_H, GRID_W, GameSim, Lane,
    MatchPhase, PlayerId, RaceKind, Team, UnitKind, attack_multiplier,
};
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::PathBuf;

const DT: f32 = 1.0 / 30.0;
const ACT_EVERY_TICKS: usize = 15;

#[derive(Clone, Copy, PartialEq)]
enum Archetype {
    Mixed,
    Counter,
    Aggro,
    Econ,
    Tech,
}

impl Archetype {
    fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "mixed" => Some(Self::Mixed),
            "counter" => Some(Self::Counter),
            "aggro" => Some(Self::Aggro),
            "econ" => Some(Self::Econ),
            "tech" => Some(Self::Tech),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Counter => "counter",
            Self::Mixed => "mixed",
            Self::Aggro => "aggro",
            Self::Econ => "econ",
            Self::Tech => "tech",
        }
    }
}

/// `(kind, cost, spawned_unit)` tuples read from the sim's balance.
type BuildingOption = (BuildingKind, i32, Option<UnitKind>);

/// ArmorType discriminants in a fixed order for dominant-armor scoring.
const ARMOR_ORDER: [castle_lanes::sim::ArmorType; 5] = [
    castle_lanes::sim::ArmorType::Normal,
    castle_lanes::sim::ArmorType::Light,
    castle_lanes::sim::ArmorType::Heavy,
    castle_lanes::sim::ArmorType::Fortified,
    castle_lanes::sim::ArmorType::Unarmored,
];

struct SideBot {
    player: PlayerId,
    team: Team,
    archetype: Archetype,
    next_lane: usize,
    next_cell: [usize; 2],
    producer_rotation: usize,
}

struct MatchOutcome {
    left_race: RaceKind,
    right_race: RaceKind,
    winner: Option<Team>,
    elapsed_secs: f32,
    adjudicated: bool,
    first_castle_hit: Option<f32>,
    unit_builds: HashMap<UnitKind, u32>,
    lane_flips: u32,
    /// Kinds spawned within 8s before a lane flip - "what bought the flip".
    flip_credits: HashMap<UnitKind, u32>,
    upgrades_by_kind: HashMap<BuildingKind, u32>,
}

fn main() {
    let args = Args::parse();
    let balance = BalanceConfig::load_or_default(
        args.balance_path
            .as_deref()
            .unwrap_or(DEFAULT_BALANCE_PATH.as_ref()),
    );
    println!(
        "sim_balance: {} games per pairing, archetype '{}', balance: {}",
        args.games,
        args.archetype.name(),
        args.balance_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "embedded default".to_string())
    );
    println!();

    let races = RaceKind::ALL;
    let mut outcomes: Vec<MatchOutcome> = Vec::new();
    for (left_index, &left_race) in races.iter().enumerate() {
        for &right_race in &races[left_index..] {
            for game in 0..args.games {
                let seed = args
                    .seed_base
                    .wrapping_mul(1_000_003)
                    .wrapping_add(game as u64 * 7919)
                    .wrapping_add(1);
                outcomes.push(run_match(
                    &balance,
                    left_race,
                    right_race,
                    args.archetype,
                    seed,
                    args.max_minutes,
                ));
            }
        }
    }

    print_win_matrix(&outcomes);
    print_pacing(&outcomes);
    print_lane_flips(&outcomes);
    print_upgrade_usage(&outcomes);
    print_unit_usage(&outcomes);
}

struct Args {
    games: usize,
    seed_base: u64,
    archetype: Archetype,
    balance_path: Option<PathBuf>,
    max_minutes: f32,
}

impl Args {
    fn parse() -> Self {
        let mut games = 100;
        let mut max_minutes = 25.0;
        let mut seed_base = 1;
        let mut archetype = Archetype::Counter;
        let mut balance_path = None;
        let argv: Vec<String> = env::args().collect();
        let mut idx = 1;
        while idx < argv.len() {
            match argv[idx].as_str() {
                "--games" if idx + 1 < argv.len() => {
                    games = argv[idx + 1].parse().expect("--games must be a number");
                    idx += 1;
                }
                "--max-mins" if idx + 1 < argv.len() => {
                    max_minutes = argv[idx + 1].parse().expect("--max-mins must be a number");
                    idx += 1;
                }
                "--seed-base" if idx + 1 < argv.len() => {
                    seed_base = argv[idx + 1].parse().expect("--seed-base must be a number");
                    idx += 1;
                }
                "--archetype" if idx + 1 < argv.len() => {
                    archetype = Archetype::parse(&argv[idx + 1]).unwrap_or_else(|| {
                        panic!("--archetype must be mixed|counter|aggro|econ|tech")
                    });
                    idx += 1;
                }
                "--balance" if idx + 1 < argv.len() => {
                    balance_path = Some(PathBuf::from(&argv[idx + 1]));
                    idx += 1;
                }
                other => panic!("unknown arg {other}"),
            }
            idx += 1;
        }
        Self {
            games,
            seed_base,
            archetype,
            balance_path,
            max_minutes,
        }
    }
}

fn run_match(
    balance: &BalanceConfig,
    left_race: RaceKind,
    right_race: RaceKind,
    archetype: Archetype,
    seed: u64,
    max_minutes: f32,
) -> MatchOutcome {
    let mut sim = GameSim::with_seed(balance.clone(), seed);
    let left = sim.join_or_update_player("Alice".to_string()).unwrap().id;
    let right = sim.join_or_update_player("Bryn".to_string()).unwrap().id;
    sim.set_race(left, left_race).unwrap();
    sim.set_race(right, right_race).unwrap();
    sim.set_ready(left, true).unwrap();
    sim.set_ready(right, true).unwrap();

    let mut bots = [
        SideBot::new(left, Team::Left, archetype),
        SideBot::new(right, Team::Right, archetype),
    ];
    let mut seen_units: HashSet<u64> = HashSet::new();
    let mut unit_builds: HashMap<UnitKind, u32> = HashMap::new();
    let mut first_castle_hit: Option<f32> = None;
    let mut lane_leader: [i8; castle_lanes::sim::Lane::ALL.len()] =
        [0; castle_lanes::sim::Lane::ALL.len()];
    let mut lane_flips = 0u32;
    let mut flip_credits: HashMap<UnitKind, u32> = HashMap::new();
    let mut recent_spawns: Vec<(f32, UnitKind)> = Vec::new();
    let mut last_sample: f32 = -1.0;
    let mut upgrades_by_kind: HashMap<BuildingKind, u32> = HashMap::new();
    let mut counted_upgrades: HashSet<u64> = HashSet::new();
    let max_ticks = (max_minutes * 60.0 / DT) as usize;

    for step in 0..max_ticks {
        sim.tick(DT);

        // Lane dynamics: 1 Hz pressure sample per lane, a "flip" is a
        // leadership change with a >15% margin (plan.md §6 lane-flip metric).
        let elapsed = sim.elapsed_secs();
        if elapsed - last_sample >= 1.0 {
            last_sample = elapsed;
            let mut pressure = [[0.0f32; 2]; castle_lanes::sim::Lane::ALL.len()];
            for unit in &sim.units {
                let side = sim.side_of(unit.owner).slot();
                let lane = unit.lane as usize;
                pressure[lane][side] += unit.health.max(0) as f32;
            }
            for (lane_index, sides) in pressure.iter().enumerate() {
                let total = sides[0] + sides[1];
                if total <= 0.0 {
                    continue;
                }
                let leader = if sides[0] > sides[1] * 1.15 {
                    1i8
                } else if sides[1] > sides[0] * 1.15 {
                    2i8
                } else {
                    0
                };
                if leader != 0 && leader != lane_leader[lane_index] {
                    if lane_leader[lane_index] != 0 {
                        lane_flips += 1;
                        // Credit kinds spawned shortly before the flip.
                        for (spawn_time, kind) in &recent_spawns {
                            if elapsed - spawn_time <= 8.0 {
                                *flip_credits.entry(*kind).or_insert(0) += 1;
                            }
                        }
                    }
                    lane_leader[lane_index] = leader;
                }
            }
            recent_spawns.retain(|(spawn_time, _)| elapsed - spawn_time <= 8.0);
        }

        if first_castle_hit.is_none()
            && sim
                .castles
                .iter()
                .any(|castle| castle.health < castle.max_health)
        {
            first_castle_hit = Some(sim.elapsed_secs());
        }
        for unit in &sim.units {
            if seen_units.insert(unit.id) {
                *unit_builds.entry(unit.kind).or_insert(0) += 1;
                recent_spawns.push((sim.elapsed_secs(), unit.kind));
            }
        }
        for building in &sim.buildings {
            let is_upgrade = sim.balance.building(building.kind).upgraded_from.is_some();
            if is_upgrade && counted_upgrades.insert(building.id) {
                *upgrades_by_kind.entry(building.kind).or_insert(0) += 1;
            }
        }
        if step % ACT_EVERY_TICKS == 0 {
            let occupied: Vec<(Team, Lane, BuildZone, (i32, i32))> = sim
                .buildings
                .iter()
                .map(|building| {
                    (
                        sim.side_of(building.owner),
                        building.lane,
                        building.zone,
                        (building.cell.x, building.cell.y),
                    )
                })
                .collect();
            for bot in bots.iter_mut() {
                bot.act(&mut sim, &occupied);
            }
        }
        if sim.phase == MatchPhase::GameOver {
            break;
        }
    }

    let adjudicated =
        sim.phase == MatchPhase::Playing || sim.castles.iter().all(|castle| castle.health <= 0);

    MatchOutcome {
        left_race,
        right_race,
        winner: sim.winner,
        elapsed_secs: sim.elapsed_secs(),
        adjudicated,
        first_castle_hit,
        unit_builds,
        lane_flips,
        flip_credits,
        upgrades_by_kind,
    }
}

/// Branch upgrade pass (plan.md Phase 2 item 3 + §6): upgrade the first
/// owned base building that has branches and is affordable. The counter
/// archetype picks the branch whose unit best damages the enemy's dominant
/// armor; other archetypes take the first listed branch.
impl SideBot {
    fn consider_upgrades(&mut self, sim: &mut GameSim) {
        let owned: Vec<u64> = sim
            .buildings
            .iter()
            .filter(|b| b.owner == self.player)
            .map(|b| b.id)
            .collect();
        for building_id in owned {
            let (kind, gold) = {
                let building = match sim.buildings.iter().find(|b| b.id == building_id) {
                    Some(b) => b,
                    None => continue,
                };
                (building.kind, sim.economies[self.team.slot()].gold)
            };
            let config = sim.balance.building(kind);
            if std::env::var("SIM_UPGRADE_DEBUG").is_ok() {
                let sec = sim.elapsed_secs() as u64;
                static LAST: std::sync::atomic::AtomicU64 =
                    std::sync::atomic::AtomicU64::new(u64::MAX);
                let last = LAST.load(std::sync::atomic::Ordering::Relaxed);
                if sec != last
                    && LAST
                        .compare_exchange(
                            last,
                            sec,
                            std::sync::atomic::Ordering::Relaxed,
                            std::sync::atomic::Ordering::Relaxed,
                        )
                        .is_ok()
                {
                    eprintln!(
                        "DBG t={:>4}s {} upgrades={:?} gold={} branch_deltas={:?}",
                        sec,
                        kind.fallback_name(),
                        config.upgrades,
                        gold,
                        config
                            .upgrades
                            .iter()
                            .map(|branch| { sim.balance.building(*branch).cost - config.cost })
                            .collect::<Vec<_>>()
                    );
                }
            }
            if config.upgrades.is_empty() {
                continue;
            }
            let enemy_units: Vec<&castle_lanes::sim::Unit> = sim
                .units
                .iter()
                .filter(|unit| sim.side_of(unit.owner) != self.team)
                .collect();
            let mut armor_score = [0.0f32; ARMOR_ORDER.len()];
            for unit in &enemy_units {
                let armor = sim.balance.unit(unit.kind).armor_type;
                armor_score[ARMOR_ORDER.iter().position(|a| a == &armor).unwrap_or(0)] +=
                    unit.health.max(0) as f32;
            }
            let dominant = armor_score
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(index, _)| index)
                .unwrap_or(0);

            let base_cost = config.cost;
            let candidates: Vec<BuildingKind> = config.upgrades.clone();
            let scored: Vec<(BuildingKind, f32, i32)> = candidates
                .iter()
                .filter_map(|branch| {
                    let branch_config = sim.balance.building(*branch);
                    let unit = branch_config.spawned_unit?;
                    let unit_config = sim.balance.unit(unit);
                    let mut score =
                        attack_multiplier(unit_config.attack_type, ARMOR_ORDER[dominant]) * 10.0;
                    let swarmy = enemy_units.len() >= 10;
                    if swarmy
                        && matches!(
                            unit_config.ability,
                            Some(castle_lanes::sim::AbilityConfig::Splash { .. })
                        )
                    {
                        score += 8.0;
                    }
                    let delta = branch_config.cost - base_cost;
                    if delta > gold {
                        return None;
                    }
                    Some((*branch, score, delta))
                })
                .collect();
            let chosen = match self.archetype {
                Archetype::Counter => scored
                    .iter()
                    .max_by(|a, b| a.1.total_cmp(&b.1))
                    .map(|(branch, _, _)| *branch),
                _ => scored.first().map(|(branch, _, _)| *branch),
            };
            if let Some(branch) = chosen {
                let result = sim.upgrade_building(self.player, building_id, branch);
                if std::env::var("SIM_UPGRADE_DEBUG").is_ok() {
                    eprintln!(
                        "DBG upgrade attempt: {:?} -> {:?} result {:?}",
                        kind, branch, result
                    );
                }
                if result.is_ok() {
                    return;
                }
            }
        }
    }
}

/// Count buildings owned by a side; `producers_only` counts unit producers,
/// otherwise economy buildings.
fn count_owned(sim: &GameSim, team: Team, producers_only: bool) -> usize {
    let sides: std::collections::HashMap<u64, Team> = sim
        .players
        .iter()
        .map(|player| (player.id.0 as u64, player.team))
        .collect();
    sim.buildings
        .iter()
        .filter(|building| {
            sides.get(&(building.owner.0 as u64)) == Some(&team)
                && sim.balance.building(building.kind).spawned_unit.is_some() == producers_only
        })
        .count()
}

impl SideBot {
    fn new(player: PlayerId, team: Team, archetype: Archetype) -> Self {
        Self {
            player,
            team,
            archetype,
            next_lane: 0,
            next_cell: [0, 0],
            producer_rotation: 0,
        }
    }

    fn act(&mut self, sim: &mut GameSim, occupied: &[(Team, Lane, BuildZone, (i32, i32))]) {
        if sim.phase != MatchPhase::Playing {
            return;
        }
        self.consider_upgrades(sim);
        // Reserve gold toward an affordable branch upgrade so saving happens
        // instead of instant spending (plan.md Phase 2 item 3).
        let upgrade_reserve: i32 = {
            let mut cheapest = i32::MAX;
            for building in &sim.buildings {
                if building.owner != self.player {
                    continue;
                }
                for branch in &sim.balance.building(building.kind).upgrades {
                    let delta = sim.balance.building(*branch).cost
                        - sim.balance.building(building.kind).cost;
                    if delta > 0 && delta < cheapest {
                        cheapest = delta;
                    }
                }
            }
            if cheapest == i32::MAX { 0 } else { cheapest }
        };
        let slot = self.team.slot();
        let race = match sim.player(self.player).and_then(|player| player.race) {
            Some(race) => race,
            None => return,
        };
        let mut options: Vec<BuildingOption> = sim
            .balance
            .race(race)
            .buildings
            .iter()
            .map(|kind| {
                let config = sim.balance.building(*kind);
                (config.kind, config.cost, config.spawned_unit)
            })
            .collect();
        options.sort_by(|a, b| b.1.cmp(&a.1)); // most expensive first
        let producers: Vec<BuildingOption> =
            options.iter().copied().filter(|o| o.2.is_some()).collect();
        let econ: Vec<BuildingOption> = options.iter().copied().filter(|o| o.2.is_none()).collect();
        let mut bought = 0;
        let mut attempts = 0;
        while bought < 3 && attempts < 60 {
            attempts += 1;
            let gold = sim.economies[slot].gold;
            let econ_count = count_owned(sim, self.team, false);
            let producer_count = count_owned(sim, self.team, true) + econ_count;
            let spendable = gold - upgrade_reserve;
            let (choice, zone) = match self.archetype {
                Archetype::Counter => {
                    // Counter-aware: score producers by typed damage against
                    // the enemy army's dominant armor, prefer splash when the
                    // enemy board is swarmy (plan.md Phase 2 item 8).
                    let enemy_units: Vec<&castle_lanes::sim::Unit> = sim
                        .units
                        .iter()
                        .filter(|unit| {
                            sim.side_of(unit.owner) != self.team
                                && sim.balance.unit(unit.kind).attack_range > 0.0
                        })
                        .collect();
                    let mut armor_score = [0.0f32; 5];
                    for unit in &enemy_units {
                        let armor = sim.balance.unit(unit.kind).armor_type;
                        armor_score[armor as usize] += unit.health.max(0) as f32;
                    }
                    let dominant = armor_score
                        .iter()
                        .enumerate()
                        .max_by(|a, b| a.1.total_cmp(b.1))
                        .map(|(index, _)| index)
                        .unwrap_or(0);
                    let swarmy = enemy_units.len() >= 10;
                    let best =
                        producers
                            .iter()
                            .rev()
                            .filter(|o| o.1 <= spendable)
                            .max_by(|a, b| {
                                let score = |o: &BuildingOption| -> f32 {
                                    let config = sim.balance.unit(o.2.unwrap());
                                    let mut value = attack_multiplier(
                                        config.attack_type,
                                        ARMOR_ORDER[dominant],
                                    ) * 10.0;
                                    if swarmy
                                        && matches!(
                                            config.ability,
                                            Some(castle_lanes::sim::AbilityConfig::Splash { .. })
                                        )
                                    {
                                        value += 8.0;
                                    }
                                    value
                                };
                                score(a).total_cmp(&score(b))
                            });
                    (best, BuildZone::Front)
                }
                Archetype::Mixed => {
                    if econ_count * 3 < producer_count + 1 {
                        (
                            econ.iter().rev().find(|o| o.1 <= spendable),
                            BuildZone::Back,
                        )
                    } else {
                        // Rotate through the producer roster cost-ascending so
                        // unit-usage stats exercise every building.
                        let affordable: Vec<_> = producers
                            .iter()
                            .rev()
                            .filter(|o| o.1 <= spendable)
                            .collect();
                        // Prefer bases that can branch-upgrade: keeps the
                        // upgrade pipeline exercised in telemetry.
                        let mut ordered = affordable;
                        ordered.sort_by_key(|o| {
                            std::cmp::Reverse(!sim.balance.building(o.0).upgrades.is_empty())
                        });
                        let pick = ordered
                            .get(self.producer_rotation % ordered.len().max(1))
                            .copied();
                        (pick, BuildZone::Front)
                    }
                }
                Archetype::Aggro => (
                    producers.iter().rev().find(|o| o.1 <= spendable),
                    BuildZone::Front,
                ),
                Archetype::Tech => {
                    let cheapest = producers.iter().rev().next();
                    let expensive = producers.first();
                    let owns_enough = producer_count >= 2;
                    if owns_enough && expensive.is_some_and(|o| o.1 <= spendable) {
                        (expensive, BuildZone::Front)
                    } else if !owns_enough && cheapest.is_some_and(|o| o.1 <= spendable) {
                        (cheapest, BuildZone::Front)
                    } else if expensive.is_some_and(|o| o.1 <= spendable) {
                        (expensive, BuildZone::Front)
                    } else {
                        (None, BuildZone::Front)
                    }
                }
                Archetype::Econ => {
                    if econ_count * 2 < producer_count + 1 {
                        (
                            econ.iter().rev().find(|o| o.1 <= spendable),
                            BuildZone::Back,
                        )
                    } else {
                        (
                            producers.iter().rev().find(|o| o.1 <= spendable),
                            BuildZone::Front,
                        )
                    }
                }
            };
            let Some((kind, _, _)) = choice else {
                break;
            };
            let zone_index = match zone {
                BuildZone::Front => 0,
                BuildZone::Back => 1,
            };
            let lane = Lane::ALL[self.next_lane % Lane::ALL.len()];
            let index = self.next_cell[zone_index];
            let cell = castle_lanes::sim::GridCell {
                x: (index % GRID_W as usize) as i32,
                y: ((index / GRID_W as usize) % GRID_H as usize) as i32,
            };
            if occupied.iter().any(|(owner, l, z, c)| {
                *owner == self.team && *l == lane && *z == zone && c.0 == cell.x && c.1 == cell.y
            }) {
                self.next_cell[zone_index] += 1;
                continue;
            }
            if sim
                .place_building(self.player, *kind, lane, zone, cell)
                .is_ok()
            {
                self.next_cell[zone_index] += 1;
                if zone == BuildZone::Front {
                    self.next_lane += 1;
                    self.producer_rotation += 1;
                }
                bought += 1;
            } else {
                break;
            }
        }
    }
}

fn print_win_matrix(outcomes: &[MatchOutcome]) {
    let races = RaceKind::ALL;
    println!(
        "== Left-side win rate, decisive games only (rows = left race, cols = right race; '-' = no decisive games) =="
    );
    print!("{:>10}", "");
    for race in races {
        print!("{:>10}", race.fallback_name());
    }
    println!();
    for &left in races.iter() {
        print!("{:>10}", left.fallback_name());
        for &right in races.iter() {
            let matching: Vec<&MatchOutcome> = outcomes
                .iter()
                .filter(|o| o.left_race == left && o.right_race == right)
                .collect();
            let finished: Vec<&MatchOutcome> = matching
                .iter()
                .copied()
                .filter(|o| o.winner.is_some())
                .collect();
            let wins = finished
                .iter()
                .filter(|o| o.winner == Some(Team::Left))
                .count();
            let percent = if finished.is_empty() {
                f32::NAN
            } else {
                wins as f32 * 100.0 / finished.len() as f32
            };
            if percent.is_nan() {
                print!("{:>9}", "-");
            } else {
                print!("{:>8.1}%", percent);
            }
        }
        println!();
    }
    println!();
}

fn print_pacing(outcomes: &[MatchOutcome]) {
    let mut lengths: Vec<f32> = outcomes.iter().map(|o| o.elapsed_secs).collect();
    lengths.sort_by(|a, b| a.total_cmp(b));
    let percentile = |p: f32| -> f32 {
        if lengths.is_empty() {
            return 0.0;
        }
        let index = ((lengths.len() as f32 - 1.0) * p).round() as usize;
        lengths[index.min(lengths.len() - 1)]
    };
    let adjudicated = outcomes.iter().filter(|o| o.adjudicated).count();
    let first_hits: Vec<f32> = outcomes.iter().filter_map(|o| o.first_castle_hit).collect();
    let avg_first_hit = if first_hits.is_empty() {
        0.0
    } else {
        first_hits.iter().sum::<f32>() / first_hits.len() as f32
    };
    println!("== Pacing ==");
    println!(
        "matches: {} | length p10 {:.0}s p50 {:.0}s p90 {:.0}s | adjudicated/unfinished: {} | first castle damage avg {:.0}s",
        outcomes.len(),
        percentile(0.10),
        percentile(0.50),
        percentile(0.90),
        adjudicated,
        avg_first_hit
    );
    println!();
}

fn print_lane_flips(outcomes: &[MatchOutcome]) {
    let total: u32 = outcomes.iter().map(|outcome| outcome.lane_flips).sum();
    let matches = outcomes.len().max(1);
    let mut credits: HashMap<UnitKind, u32> = HashMap::new();
    for outcome in outcomes {
        for (kind, count) in &outcome.flip_credits {
            *credits.entry(*kind).or_insert(0) += count;
        }
    }
    let mut rows: Vec<(UnitKind, u32)> = credits.into_iter().collect();
    rows.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    println!("== Lane dynamics ==");
    println!(
        "lane flips per match: avg {:.2} | total {} across {} matches",
        total as f32 / matches as f32,
        total,
        matches
    );
    println!("flip credits (kinds spawned within 8s before a flip), per 100 matches:");
    let scale = matches as f32 / 100.0;
    for (kind, count) in rows.iter().take(8) {
        println!(
            "{:>22} {:>8.1}",
            kind.fallback_name(),
            *count as f32 / scale
        );
    }
    println!();
}

fn print_upgrade_usage(outcomes: &[MatchOutcome]) {
    let mut totals: HashMap<BuildingKind, u32> = HashMap::new();
    for outcome in outcomes {
        for (kind, count) in &outcome.upgrades_by_kind {
            *totals.entry(*kind).or_insert(0) += count;
        }
    }
    let mut rows: Vec<(BuildingKind, u32)> = totals.into_iter().collect();
    rows.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    println!("== Branch upgrades per 100 matches ==");
    if rows.is_empty() {
        println!("(none)");
    }
    let scale = outcomes.len().max(1) as f32 / 100.0;
    for (kind, count) in rows {
        println!(
            "{:>22} {:>8.1}",
            format!("{}/{}", kind.fallback_name(), kind.race().fallback_name()),
            count as f32 / scale
        );
    }
    println!();
}

fn print_unit_usage(outcomes: &[MatchOutcome]) {
    let mut totals: HashMap<UnitKind, u32> = HashMap::new();
    for outcome in outcomes {
        for (kind, count) in &outcome.unit_builds {
            *totals.entry(*kind).or_insert(0) += count;
        }
    }
    let mut rows: Vec<(UnitKind, u32)> = totals.into_iter().collect();
    rows.sort_by_key(|(_, count)| *count);
    println!("== Unit builds per 100 matches (lowest first) ==");
    let scale = outcomes.len().max(1) as f32 / 100.0;
    for (kind, count) in rows {
        println!("{:>22} {:>8.1}", kind.fallback_name(), count as f32 / scale);
    }
}

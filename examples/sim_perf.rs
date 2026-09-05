use castle_lanes::net::{
    PacketFormat, ServerPacket, diff_snapshot, encode, entityless_snapshot, set_outgoing_format,
};
use castle_lanes::sim::{
    GameSim, Lane, MatchPhase, RaceKind, Team, Unit, UnitKind, WorldPos, lane_center_y,
};
use std::time::{Duration, Instant};

const DT: f32 = 1.0 / 30.0;
const SAMPLE_TICKS: usize = 300;

fn main() {
    match std::env::var("CASTLE_LANES_FORMAT")
        .unwrap_or_default()
        .as_str()
    {
        "json" => set_outgoing_format(PacketFormat::Json),
        _ => set_outgoing_format(PacketFormat::Bincode),
    }
    println!(
        "Castle Lanes sim perf (format: {:?}), release build recommended",
        castle_lanes::net::outgoing_format()
    );
    println!("ticks per sample: {SAMPLE_TICKS}, fixed dt: {DT:.5}s");
    println!();

    for units in [2_usize, 100, 500, 1000, 2000] {
        let mut sim = seeded_match(units);
        warm_up(&mut sim);
        let tick = bench_ticks(&mut sim);
        let packet = bench_packets(&mut sim);
        println!(
            "{units:>4} units | tick {:>8.3} ms | sim {:>7.1}x realtime | full {:>8} B | baseline {:>5} B | delta {:>8} B | encode {:>7.3} ms",
            tick.ms_per_tick,
            tick.realtime_factor,
            packet.full_snapshot_bytes,
            packet.entityless_baseline_bytes,
            packet.delta_bytes,
            packet.delta_ms_per_encode
        );
    }
}

struct TickBench {
    ms_per_tick: f64,
    realtime_factor: f64,
}

struct PacketBench {
    full_snapshot_bytes: usize,
    entityless_baseline_bytes: usize,
    delta_bytes: usize,
    delta_ms_per_encode: f64,
}

fn seeded_match(unit_count: usize) -> GameSim {
    let mut sim = GameSim::default();
    let left = sim.join_or_update_player("Alice".to_string()).unwrap().id;
    let right = sim.join_or_update_player("Bryn".to_string()).unwrap().id;
    sim.set_race(left, RaceKind::Vanguard).unwrap();
    sim.set_race(right, RaceKind::Vanguard).unwrap();
    sim.set_ready(left, true).unwrap();
    sim.set_ready(right, true).unwrap();
    assert_eq!(sim.phase, MatchPhase::Playing);

    for castle in &mut sim.castles {
        castle.health = 1_000_000;
        castle.max_health = 1_000_000;
    }

    sim.units.clear();
    let left_count = unit_count / 2 + unit_count % 2;
    let right_count = unit_count / 2;
    seed_team_units(&mut sim, Team::Left, left_count, 1);
    seed_team_units(&mut sim, Team::Right, right_count, 1 + left_count as u64);
    sim
}

fn seed_team_units(sim: &mut GameSim, owner: Team, count: usize, first_id: u64) {
    if count == 0 {
        return;
    }

    let kind = match owner {
        Team::Left => UnitKind::VanguardGuard,
        Team::Right => UnitKind::EmberRunner,
    };
    let config = sim.balance.unit(kind);
    let lanes = [Lane::Top, Lane::Bottom];
    let per_lane = count.div_ceil(lanes.len());

    for idx in 0..count {
        let lane = lanes[idx % lanes.len()];
        let lane_idx = idx / lanes.len();
        let lane_fraction = if per_lane <= 1 {
            0.5
        } else {
            lane_idx as f32 / (per_lane - 1) as f32
        };
        let lane_pos = match owner {
            Team::Left => 5.0 + lane_fraction * 42.0,
            Team::Right => 95.0 - lane_fraction * 42.0,
        };
        let row = (lane_idx % 9) as f32 - 4.0;
        let y = lane_center_y(lane) + row * 0.18;

        sim.units.push(Unit {
            id: first_id + idx as u64,
            owner,
            kind,
            lane,
            health: 100_000,
            lane_pos,
            pos: WorldPos::new(lane_pos, y),
            velocity: WorldPos::new(0.0, 0.0),
            radius: config.radius,
            attack_timer: 0.15 + (idx % 11) as f32 * 0.03,
        });
    }
}

fn warm_up(sim: &mut GameSim) {
    for _ in 0..30 {
        sim.tick(DT);
    }
}

fn bench_ticks(sim: &mut GameSim) -> TickBench {
    let start = Instant::now();
    for _ in 0..SAMPLE_TICKS {
        sim.tick(DT);
    }
    let elapsed = start.elapsed();
    let ms_per_tick = elapsed.as_secs_f64() * 1000.0 / SAMPLE_TICKS as f64;
    let simulated = Duration::from_secs_f32(DT * SAMPLE_TICKS as f32);
    TickBench {
        ms_per_tick,
        realtime_factor: simulated.as_secs_f64() / elapsed.as_secs_f64(),
    }
}

fn bench_packets(sim: &mut GameSim) -> PacketBench {
    let previous = sim.snapshot();
    sim.tick(DT);
    let current = sim.snapshot();
    let full_packet = ServerPacket::Snapshot(current.clone());
    let baseline_packet = ServerPacket::Snapshot(entityless_snapshot(&current));
    let delta_packet = ServerPacket::SnapshotDelta(diff_snapshot(&previous, &current));
    let full_bytes = encode(&full_packet)
        .expect("snapshot packet should encode")
        .len();
    let baseline_bytes = encode(&baseline_packet)
        .expect("baseline packet should encode")
        .len();
    let delta_bytes = encode(&delta_packet)
        .expect("delta packet should encode")
        .len();
    let iterations = if delta_bytes > 100_000 { 20 } else { 100 };

    let start = Instant::now();
    for _ in 0..iterations {
        let _ = encode(&delta_packet).expect("delta packet should encode");
    }
    let elapsed = start.elapsed();

    PacketBench {
        full_snapshot_bytes: full_bytes,
        entityless_baseline_bytes: baseline_bytes,
        delta_bytes,
        delta_ms_per_encode: elapsed.as_secs_f64() * 1000.0 / iterations as f64,
    }
}

//! Replay determinism verification (plan.md Phase 3): load a recorded match,
//! re-run it through the deterministic sim, and assert the outcome matches
//! the recording. Usage:
//!   cargo run --release --example replay_verify -- <recording.json> [balance.json]
//! The balance.json must match the config the recording was made with.

use castle_lanes::record::{RecordedIntent, load_record};
use castle_lanes::sim::{
    BalanceConfig, BuildZone, BuildingKind, DEFAULT_BALANCE_PATH, GameSim, GridCell, Lane,
    PlayerId, RaceKind,
};
use std::collections::HashMap;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args
        .get(1)
        .expect("usage: replay_verify <recording.json> [balance.json]");
    let balance_path = args
        .get(2)
        .map(|s| s.as_str())
        .unwrap_or(DEFAULT_BALANCE_PATH);
    let record = castle_lanes::record::load_record(path).expect("load recording");
    println!(
        "replaying {} (seed {}, {} commands)",
        path,
        record.seed,
        record.commands.len()
    );

    let mut sim = GameSim::with_seed(BalanceConfig::load_or_default(balance_path), record.seed);
    if record.players.len() > 2 {
        sim.set_team_size(record.players.len() / 2).unwrap();
    }
    // rebuild seats in recorded order
    let mut id_map: HashMap<u8, PlayerId> = HashMap::new();
    for (index, player) in record.players.iter().enumerate() {
        let id = sim
            .join_or_update_player(player.name.clone())
            .expect("join recorded player")
            .id;
        id_map.insert(index as u8 + 1, id);
        if let Some(race) = player.race {
            sim.set_race(id, race).unwrap();
        }
        sim.set_ready(id, true).unwrap();
    }
    sim.phase = castle_lanes::sim::MatchPhase::Playing;
    sim.reset_match_state();
    sim.phase = castle_lanes::sim::MatchPhase::Playing;

    let dt = 1.0 / 30.0;
    let mut applied = 0usize;
    for command in &record.commands {
        while sim.tick < command.tick {
            sim.tick(dt);
        }
        let player_id = id_map[&command.player];
        let result = match &command.intent {
            RecordedIntent::SetRace { race } => sim.set_race(player_id, *race).map(|_| ()),
            RecordedIntent::SetReady { ready } => sim.set_ready(player_id, *ready),
            RecordedIntent::PlaceBuilding {
                kind,
                lane,
                zone,
                cell,
            } => sim.place_building(
                player_id,
                *kind,
                *lane,
                *zone,
                GridCell {
                    x: cell.x,
                    y: cell.y,
                },
            ),
            RecordedIntent::SellBuilding { building_id } => {
                sim.sell_building(player_id, *building_id)
            }
            RecordedIntent::UpgradeBuilding { building_id, to } => {
                sim.upgrade_building(player_id, *building_id, *to)
            }
            RecordedIntent::Surrender => sim.surrender(player_id),
            RecordedIntent::VoteRematch => {
                sim.vote_rematch(player_id);
                Ok(())
            }
        };
        if let Err(err) = result {
            println!("command replay failed: {err}");
        } else {
            applied += 1;
        }
    }

    // fast-forward to the recorded end state
    let mut guard = 0;
    while sim.phase == castle_lanes::sim::MatchPhase::Playing && guard < 30 * 60 * 30 {
        sim.tick(dt);
        guard += 1;
    }

    println!(
        "result: applied {}/{} commands, winner {:?} (recorded {:?}), sim_time {:.1}s vs recorded {:.1}s",
        applied,
        record.commands.len(),
        sim.winner,
        record.winner,
        sim.elapsed_secs(),
        record.duration_secs
    );
    assert_eq!(
        sim.winner, record.winner,
        "deterministic replay diverged from recording"
    );
    println!("DETERMINISM VERIFIED");
}

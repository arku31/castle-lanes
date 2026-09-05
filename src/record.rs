//! Match recording (plan.md Phase 1 item 7).
//!
//! The server records every match: header (seed, players), the intent log
//! with sim ticks, kill events, a 1 Hz state timeline, and the result.
//! Because the sim is deterministic per seed (plan.md P0-4), a recording is
//! both a replay file and a telemetry source for the balance loop (§6).

use crate::sim::{
    BuildZone, BuildingKind, GameSim, GridCell, Lane, MatchPhase, RaceKind, Team, UnitKind,
};
use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
pub struct MatchRecord {
    pub game_id: u32,
    pub seed: u64,
    pub started_unix: u64,
    pub duration_secs: f32,
    pub winner: Option<Team>,
    pub players: Vec<RecordedPlayer>,
    pub commands: Vec<RecordedCommand>,
    pub kills: Vec<RecordedKill>,
    pub timeline: Vec<TimelineSample>,
}

#[derive(Debug, Serialize)]
pub struct RecordedPlayer {
    pub name: String,
    pub team: Team,
    pub race: Option<RaceKind>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "cmd", content = "args")]
pub enum RecordedIntent {
    SetRace {
        race: RaceKind,
    },
    SetReady {
        ready: bool,
    },
    PlaceBuilding {
        kind: BuildingKind,
        lane: Lane,
        zone: BuildZone,
        cell: GridCell,
    },
    SellBuilding {
        building_id: u64,
    },
    Surrender,
    VoteRematch,
}

#[derive(Debug, Serialize)]
pub struct RecordedCommand {
    /// Sim tick at which the server applied the intent.
    pub tick: u64,
    pub elapsed_secs: f32,
    pub player: u8,
    pub intent: RecordedIntent,
}

#[derive(Debug, Serialize)]
pub struct RecordedKill {
    pub elapsed_secs: f32,
    pub team: Team,
    pub kind: UnitKind,
}

#[derive(Debug, Serialize)]
pub struct TimelineSample {
    pub elapsed_secs: f32,
    pub castle_health: [i32; 2],
    pub gold: [i32; 2],
    pub income: [i32; 2],
    pub unit_count: [usize; 2],
    pub army_health: [i32; 2],
}

/// Per-room recorder state. Observations are cheap enough to run every sim
/// tick; file writes happen once, when the match leaves `Playing`.
#[derive(Default)]
pub struct MatchRecorder {
    game_id: u32,
    record: Option<MatchRecord>,
    seen_bounty: HashSet<u64>,
    next_sample: f32,
    finished: bool,
}

impl MatchRecorder {
    pub fn new(game_id: u32) -> Self {
        Self {
            game_id,
            ..Self::default()
        }
    }

    pub fn is_active(&self) -> bool {
        self.record.is_some()
    }

    pub fn record_command(&mut self, sim: &GameSim, player: u8, intent: RecordedIntent) {
        let Some(record) = self.record.as_mut() else {
            return;
        };
        record.commands.push(RecordedCommand {
            tick: sim.tick,
            elapsed_secs: sim.elapsed_secs(),
            player,
            intent,
        });
    }

    /// Observe the sim: starts recording on match start, samples the 1 Hz
    /// timeline and kill feed during play, and finalizes on phase exit.
    pub fn observe(&mut self, sim: &GameSim) {
        match sim.phase {
            MatchPhase::Lobby => self.finalize(sim),
            MatchPhase::Playing => {
                if self.record.is_none() {
                    self.start(sim);
                }
                for event in &sim.bounty_events {
                    if self.seen_bounty.insert(event.id) {
                        if let Some(record) = self.record.as_mut() {
                            record.kills.push(RecordedKill {
                                elapsed_secs: sim.elapsed_secs(),
                                team: event.team,
                                kind: event.unit_kind,
                            });
                        }
                    }
                }
                let elapsed = sim.elapsed_secs();
                if elapsed >= self.next_sample {
                    self.next_sample += 1.0;
                    if let Some(record) = self.record.as_mut() {
                        record.timeline.push(sample(sim));
                    }
                }
            }
            MatchPhase::GameOver => {
                if !self.finished {
                    self.finalize(sim);
                }
            }
        }
    }

    fn start(&mut self, sim: &GameSim) {
        self.next_sample = 1.0;
        self.finished = false;
        self.record = Some(MatchRecord {
            game_id: self.game_id,
            seed: sim.seed(),
            started_unix: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            duration_secs: 0.0,
            winner: None,
            players: sim
                .players
                .iter()
                .map(|player| RecordedPlayer {
                    name: player.name.clone(),
                    team: player.team,
                    race: player.race,
                })
                .collect(),
            commands: Vec::new(),
            kills: Vec::new(),
            timeline: Vec::new(),
        });
    }

    fn finalize(&mut self, sim: &GameSim) {
        let Some(record) = self.record.as_mut() else {
            return;
        };
        record.duration_secs = sim.elapsed_secs();
        record.winner = sim.winner;
        record.timeline.push(sample(sim));
        self.finished = true;
    }

    /// Take the finished record for writing; returns None while a match is
    /// still being recorded.
    pub fn take_finished(&mut self) -> Option<MatchRecord> {
        if !self.finished {
            return None;
        }
        self.record.take()
    }
}

fn sample(sim: &GameSim) -> TimelineSample {
    let mut unit_count = [0usize; 2];
    let mut army_health = [0i32; 2];
    for unit in &sim.units {
        let slot = unit.owner.slot();
        unit_count[slot] += 1;
        army_health[slot] += unit.health.max(0);
    }
    TimelineSample {
        elapsed_secs: sim.elapsed_secs(),
        castle_health: [sim.castles[0].health, sim.castles[1].health],
        gold: [sim.economies[0].gold, sim.economies[1].gold],
        income: [sim.economies[0].income, sim.economies[1].income],
        unit_count,
        army_health,
    }
}

/// Write a finished record as JSON under `recordings/`; the server treats
/// write failures as non-fatal.
pub fn write_record(record: &MatchRecord) -> std::io::Result<PathBuf> {
    let dir = PathBuf::from("recordings");
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!(
        "game{}_{}.json",
        record.game_id, record.started_unix
    ));
    let json = serde_json::to_vec_pretty(record)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
    fs::write(&path, json)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{BalanceConfig, BuildZone, BuildingKind, GridCell};

    #[test]
    fn recording_captures_header_commands_timeline_and_result() {
        let mut sim = GameSim::with_seed(BalanceConfig::default(), 4242);
        let mut recorder = MatchRecorder::new(7);
        let left = sim.join_or_update_player("Alice".to_string()).unwrap().id;
        let right = sim.join_or_update_player("Bryn".to_string()).unwrap().id;
        sim.set_race(left, RaceKind::Vanguard).unwrap();
        sim.set_race(right, RaceKind::Grove).unwrap();
        recorder.observe(&sim);
        sim.set_ready(left, true).unwrap();
        sim.set_ready(right, true).unwrap();
        recorder.observe(&sim);

        assert!(sim.phase == MatchPhase::Playing);
        sim.place_building(
            left,
            BuildingKind::VanguardBarracks,
            Lane::Top,
            BuildZone::Front,
            GridCell { x: 0, y: 0 },
        )
        .unwrap();
        recorder.record_command(
            &sim,
            1,
            RecordedIntent::PlaceBuilding {
                kind: BuildingKind::VanguardBarracks,
                lane: Lane::Top,
                zone: BuildZone::Front,
                cell: GridCell { x: 0, y: 0 },
            },
        );
        sim.place_building(
            right,
            BuildingKind::GroveRootDen,
            Lane::Top,
            BuildZone::Front,
            GridCell { x: 0, y: 0 },
        )
        .unwrap();

        let mut ticks = 0;
        while sim.phase == MatchPhase::Playing && ticks < 30 * 60 * 25 {
            sim.tick(1.0 / 30.0);
            recorder.observe(&sim);
            ticks += 1;
        }
        assert_eq!(sim.phase, MatchPhase::GameOver);
        recorder.observe(&sim);

        let record = recorder.take_finished().expect("record finalized");
        assert_eq!(record.game_id, 7);
        assert_eq!(record.seed, 4242);
        assert_eq!(record.players.len(), 2);
        assert_eq!(record.commands.len(), 1);
        assert!(record.timeline.len() > 10, "1 Hz timeline expected");
        assert!(
            record
                .timeline
                .last()
                .is_some_and(|sample| sample.castle_health.iter().any(|hp| *hp <= 0)),
            "final sample should show a fallen castle"
        );
        assert!(record.winner.is_some());
        assert!(!record.kills.is_empty(), "kills feed expected in a fight");
        assert!(record.duration_secs > 10.0);

        // In-memory serialization round-trip keeps replays portable.
        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("\"seed\":4242"));
    }

    #[test]
    fn recording_finalizes_without_winner_when_match_is_abandoned() {
        let mut sim = GameSim::with_seed(BalanceConfig::default(), 1);
        let left = sim.join_or_update_player("Alice".to_string()).unwrap().id;
        let right = sim.join_or_update_player("Bryn".to_string()).unwrap().id;
        sim.set_race(left, RaceKind::Vanguard).unwrap();
        sim.set_race(right, RaceKind::Ember).unwrap();
        sim.set_ready(left, true).unwrap();
        sim.set_ready(right, true).unwrap();
        let mut recorder = MatchRecorder::new(3);
        recorder.observe(&sim);
        assert!(recorder.is_active());

        // Both players leave without a GameOver; the server's grace expiry
        // resets to lobby and the recorder finalizes with no winner instead
        // of leaking a half-open record.
        sim.disconnect_player(left);
        sim.disconnect_player(right);
        sim.reset_to_lobby();
        recorder.observe(&sim);
        let record = recorder.take_finished().unwrap();
        assert_eq!(record.winner, None);
    }
}

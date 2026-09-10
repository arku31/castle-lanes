//! net systems split out of the monolithic client (plan.md Phase 1 item 8).
#![allow(unused_imports)]
pub(crate) use super::audio::*;
pub(crate) use super::input::*;
pub(crate) use super::scene::*;
pub(crate) use super::ui::*;
pub(crate) use super::vfx::*;
use super::*;

pub(crate) fn receive_packets(
    mut net: ResMut<ClientNet>,
    mut state: ResMut<SnapshotState>,
    mut interp: ResMut<RenderInterp>,
) {
    if net.is_replay {
        return;
    }
    let mut buf = [0_u8; 65_535];
    loop {
        match net.socket.recv_from(&mut buf) {
            Ok((len, _)) => match decode_server(&buf[..len]) {
                Ok(ServerPacket::Connected { name }) => {
                    net.connected = true;
                    net.player_name = name;
                    net.status = "Connected. Create a game or join one from the list.".to_string();
                    send_client(&net, &ClientPacket::ListGames);
                }
                Ok(ServerPacket::Welcome { game_id, player }) => {
                    net.game_id = Some(game_id);
                    net.player_id = Some(player.id);
                    net.team = Some(player.team);
                    net.status = format!(
                        "Joined game #{game_id} as {} ({:?}).",
                        player.name, player.team
                    );
                    state.snapshot = None;
                }
                Ok(ServerPacket::GameList { games }) => {
                    state.games = games;
                }
                Ok(ServerPacket::BalanceRaces {
                    starting_gold,
                    base_income,
                    income_interval,
                    interest_rate,
                    castle_regen_per_second,
                    sudden_death_start,
                    races,
                }) => {
                    state.balance.starting_gold = starting_gold;
                    state.balance.base_income = base_income;
                    state.balance.income_interval = income_interval;
                    state.balance.interest_rate = interest_rate;
                    state.balance.castle_regen_per_second = castle_regen_per_second;
                    state.balance.sudden_death_start = sudden_death_start;
                    state.balance.races = races;
                }
                Ok(ServerPacket::BalanceBuildings { buildings }) => {
                    state.balance.buildings = buildings;
                }
                Ok(ServerPacket::BalanceUnits { units }) => {
                    state.balance.units = units;
                }
                Ok(ServerPacket::Snapshot(snapshot)) => {
                    state.snapshot = Some(snapshot);
                }
                Ok(ServerPacket::SnapshotDelta(delta)) => {
                    if let Some(snapshot) = &mut state.snapshot {
                        apply_snapshot_delta(snapshot, delta);
                    } else {
                        net.status = "Waiting for baseline snapshot.".to_string();
                    }
                }
                Ok(ServerPacket::Ack { seq: Some(seq) }) => {
                    if let Some(pending) = &net.pending_placement {
                        if pending.seq == seq {
                            net.pending_placement = None;
                        }
                    }
                }
                Ok(ServerPacket::Ack { seq: None }) => {}
                Ok(ServerPacket::ProfileData { wins, losses }) => {
                    net.profile_wins = wins;
                    net.profile_losses = losses;
                }
                Ok(ServerPacket::QueueStatus {
                    queued,
                    needed,
                    team_size,
                }) => {
                    net.status = format!(
                        "Queued {}v{}: {}/{} players. Waiting...",
                        team_size, team_size, queued, needed
                    );
                }
                Ok(ServerPacket::Error { message }) => {
                    net.pending_placement = None;
                    net.status = message;
                }
                Err(err) => {
                    net.status = format!("Bad server packet: {err}");
                }
            },
            Err(err) if err.kind() == ErrorKind::WouldBlock => break,
            Err(err) => {
                net.status = format!("Network error: {err}");
                break;
            }
        }
    }

    note_snapshot_for_interp(&mut interp, &state);

    if !net.connected && net.last_join.elapsed() > Duration::from_secs(2) {
        send_join(&mut net);
    } else if net.connected && net.last_keepalive.elapsed() > Duration::from_secs(2) {
        send_join(&mut net);
        net.last_keepalive = Instant::now();
    }
}

pub(crate) fn note_snapshot_for_interp(interp: &mut RenderInterp, state: &ResMut<SnapshotState>) {
    let Some(snapshot) = &state.snapshot else {
        *interp = RenderInterp::default();
        return;
    };
    if !state.is_changed() {
        return;
    }
    let latest_tick = interp.latest.as_ref().map(|latest| latest.tick);
    if latest_tick == Some(snapshot.tick) {
        return;
    }
    let unit_pos = snapshot
        .units
        .iter()
        .map(|unit| (unit.id, unit.pos))
        .collect();
    if let Some(latest) = &mut interp.latest {
        interp.prev = Some(InterpSnapshot {
            at: latest.at,
            tick: latest.tick,
            unit_pos: std::mem::replace(&mut latest.unit_pos, unit_pos),
        });
        latest.at = Instant::now();
        latest.tick = snapshot.tick;
    } else {
        interp.latest = Some(InterpSnapshot {
            at: Instant::now(),
            tick: snapshot.tick,
            unit_pos,
        });
    }
}

pub(crate) fn demo_automation(
    mut net: ResMut<ClientNet>,
    state: Res<SnapshotState>,
    mut selection: ResMut<BuildSelection>,
    mut hints: ResMut<MatchHints>,
    time: Res<Time>,
) {
    if net.connected && net.player_id.is_none() && (net.auto_ready || net.auto_build_demo) {
        if !net.sent_auto_game {
            send_client(
                &net,
                &ClientPacket::CreateGame {
                    name: format!("{}'s Game", net.player_name),
                    team_size: None,
                    random_factions: false,
                },
            );
            net.sent_auto_game = true;
            return;
        }
        // A stale connection or lost packet can leave us seatless; retry the
        // join periodically until the server frees the name (zombie timeout).
        if net.last_join.elapsed() >= Duration::from_secs(3) {
            send_join(&mut net);
            return;
        }
    }
    let Some(player_id) = net.player_id else {
        return;
    };
    // Auto intents are one-shot UDP sends; a lost packet would stall the demo
    // in the race popup forever. Re-send while the lobby state shows they
    // have not taken effect yet.
    let me = state
        .snapshot
        .as_ref()
        .and_then(|snapshot| current_player(snapshot, player_id));
    let race_set = me.and_then(|player| player.race).is_some();
    if !net.sent_auto_race || !race_set {
        send_client(
            &net,
            &ClientPacket::SetRace {
                player_id,
                race: net.auto_race,
            },
        );
        net.sent_auto_race = true;
    }
    if net.auto_ready {
        let ready = me.map(|player| player.ready).unwrap_or(false);
        if (!net.sent_auto_ready || !ready) && race_set && me.is_some() {
            send_client(
                &net,
                &ClientPacket::SetReady {
                    player_id,
                    ready: true,
                },
            );
            net.sent_auto_ready = true;
        }
    }

    if !net.auto_build_demo {
        return;
    }
    let Some(snapshot) = &state.snapshot else {
        eprintln!("DEMO gate: no snapshot");
        return;
    };
    if snapshot.phase != MatchPhase::Playing {
        eprintln!("DEMO gate: phase={:?}", snapshot.phase);
        return;
    }
    hints.tooltip_suppressed = true;
    // Showcase the placement UI: footprint tiles + ghost while auto-building.
    if hints.step == 0 {
        let race = current_player(snapshot, player_id)
            .and_then(|player| player.race)
            .unwrap_or(net.auto_race);
        let balance = active_balance(&state);
        let kind = balance.race(race).buildings[0];
        selection.kind = Some(kind);
        hints.step = 1;
    }
    // Place a spread of buildings so battles field real armies. The clock
    // starts when the battle phase is first seen (last_join doubles as the
    // keepalive and resets constantly).
    if hints.battle_at == 0.0 {
        hints.battle_at = time.elapsed_secs();
    }
    let since_battle = time.elapsed_secs() - hints.battle_at;
    let race = current_player(snapshot, player_id)
        .and_then(|player| player.race)
        .unwrap_or(net.auto_race);
    let balance = active_balance(&state);
    let kinds = balance.race(race).buildings.clone();
    let spots: [(i32, i32); 8] = [
        (0, 0),
        (4, 1),
        (8, 0),
        (2, 3),
        (6, 3),
        (9, 1),
        (4, 4),
        (7, 4),
    ];
    let idx = net.demo_build_count;
    let spots_n = spots.len() as u32;
    let kinds_n = kinds.len() as u32;
    if idx >= kinds_n.min(spots_n) {
        net.sent_auto_build = true;
        return;
    }
    if since_battle < 8.0 + idx as f32 * 9.0 {
        return;
    }
    let (cx, cy) = spots[(idx % spots_n) as usize];
    let kind = kinds[(idx % kinds_n) as usize];
    send_client(
        &net,
        &ClientPacket::PlaceBuilding {
            player_id,
            kind,
            lane: default_lane_for_team(net.team.unwrap_or(Team::Left)),
            zone: BuildZone::Front,
            cell: GridCell { x: cx, y: cy },
            seq: None,
        },
    );
    net.demo_build_count += 1;
}

pub(crate) fn local_gold(state: &SnapshotState, net: &ClientNet) -> Option<i32> {
    let snapshot = state.snapshot.as_ref()?;
    let player_id = net.player_id?;
    let player = current_player(snapshot, player_id)?;
    Some(snapshot.economies[player_index_of(snapshot, player_id)].gold)
}

pub(crate) fn active_balance(state: &SnapshotState) -> BalanceConfig {
    state.balance.clone()
}

pub(crate) fn current_player(
    snapshot: &MatchSnapshot,
    player_id: PlayerId,
) -> Option<&castle_lanes::sim::PlayerInfo> {
    snapshot
        .players
        .iter()
        .find(|player| player.id == player_id)
}

pub(crate) fn selected_race(state: &SnapshotState, net: &ClientNet) -> Option<RaceKind> {
    let snapshot = state.snapshot.as_ref()?;
    let player_id = net.player_id?;
    current_player(snapshot, player_id).and_then(|player| player.race)
}

pub(crate) fn race_selection_popup_open(state: &SnapshotState, net: &ClientNet) -> bool {
    let Some(snapshot) = &state.snapshot else {
        return false;
    };
    if snapshot.phase != MatchPhase::Lobby {
        return false;
    }
    let Some(player_id) = net.player_id else {
        return false;
    };
    current_player(snapshot, player_id).is_some_and(|player| !player.ready)
}

pub(crate) fn team_race(snapshot: &MatchSnapshot, team: Team) -> Option<RaceKind> {
    snapshot
        .players
        .iter()
        .find(|player| player.team == team)
        .and_then(|player| player.race)
}

pub(crate) fn send_join(net: &mut ClientNet) {
    send_client(
        net,
        &ClientPacket::Join {
            version: PROTOCOL_VERSION,
            name: net.player_name.clone(),
        },
    );
    net.last_join = Instant::now();
}

pub(crate) fn send_client(net: &ClientNet, packet: &ClientPacket) {
    if let Ok(bytes) = encode(packet) {
        let _ = net.socket.send_to(&bytes, net.server_addr);
    }
}

//! Bisects which ServerPacket variant fails bincode round-trip.

use castle_lanes::net::{encode_as, decode_server, PacketFormat, ServerPacket};
use castle_lanes::sim::*;

fn main() {
    let mut sim = GameSim::with_seed(BalanceConfig::load_or_default(DEFAULT_BALANCE_PATH), 1);
    let l = sim.join_or_update_player("A".into()).unwrap().id;
    let r = sim.join_or_update_player("B".into()).unwrap().id;
    sim.set_race(l, RaceKind::Vanguard).unwrap();
    sim.set_race(r, RaceKind::Ember).unwrap();
    let snapshot = sim.snapshot();

    let packets: Vec<(&str, ServerPacket)> = vec![
        ("Connected", ServerPacket::Connected { name: "A".into() }),
        ("GameList", ServerPacket::GameList { games: vec![castle_lanes::net::GameInfo {
            id: 1, name: "g".into(), phase: MatchPhase::Playing, players: 2, max_players: 2,
        }]}),
        ("Welcome", ServerPacket::Welcome { game_id: 1, player: sim.player(l).unwrap() }),
        ("BalanceRaces", ServerPacket::BalanceRaces {
            starting_gold: 150, base_income: 10, income_interval: 10.0, interest_rate: 0.04,
            castle_regen_per_second: 10.0, sudden_death_start: 480.0,
            races: sim.balance.races.clone(),
        }),
        ("BalanceBuildings", ServerPacket::BalanceBuildings { buildings: sim.balance.buildings.clone() }),
        ("BalanceUnits", ServerPacket::BalanceUnits { units: sim.balance.units.clone() }),
        ("Snapshot", ServerPacket::Snapshot(snapshot.clone())),
        ("Error", ServerPacket::Error { message: "e".into() }),
    ];

    for (name, packet) in packets {
        let bytes = encode_as(&packet, PacketFormat::Bincode).expect("encode");
        match decode_server(&bytes) {
            Ok(_) => println!("OK       {}", name),
            Err(err) => println!("FAIL     {}: {}", name, err),
        }
    }
}

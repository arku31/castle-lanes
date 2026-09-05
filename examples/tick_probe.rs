use castle_lanes::record::MatchRecorder;
use castle_lanes::sim::*;

fn main() {
    let mut sim = GameSim::with_seed(BalanceConfig::load_or_default("/tmp/balance_quick2.json"), 99);
    let l = sim.join_or_update_player("Alice".into()).unwrap().id;
    let r = sim.join_or_update_player("Bryn".into()).unwrap().id;
    sim.set_race(l, RaceKind::Vanguard).unwrap();
    sim.set_race(r, RaceKind::Ember).unwrap();
    sim.set_ready(l, true).unwrap();
    sim.set_ready(r, true).unwrap();
    let mut rec = MatchRecorder::new(1);
    for i in 0..3000i32 {
        sim.tick(1.0/30.0);
        rec.observe(&sim);
        if i % 300 == 0 {
            println!("i={i} phase={:?} elapsed={} castles={:?}", sim.phase, sim.elapsed_secs(), [sim.castles[0].health, sim.castles[1].health]);
        }
        if sim.phase == MatchPhase::GameOver {
            println!("GAMEOVER at i={i} elapsed={} winner={:?}", sim.elapsed_secs(), sim.winner);
            rec.observe(&sim);
            println!("record finalized: {}", rec.take_finished().is_some());
            return;
        }
    }
    println!("never ended: elapsed={} castles={:?}", sim.elapsed_secs(), [sim.castles[0].health, sim.castles[1].health]);
}

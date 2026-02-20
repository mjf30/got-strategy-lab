// ═══════════════════════════════════════════════════════════════════════
// Runner — CLI entry point for running games and tournaments
// ═══════════════════════════════════════════════════════════════════════

use got_engine::types::HouseName;
use got_agents::RandomAgent;
use got_agents::Agent;
use got_tournament::run_game;
use std::collections::HashMap;

fn main() {
    println!("=== Game of Thrones Strategy Lab ===\n");
    println!("Running test game: 6 Random Agents...\n");

    let seed = 42u64;
    let player_count = 6;

    // Create agents
    let mut agents: HashMap<HouseName, Box<dyn Agent>> = HashMap::new();
    for (i, &house) in HouseName::ALL.iter().enumerate() {
        agents.insert(house, Box::new(RandomAgent::new(house, seed + i as u64)));
    }

    match run_game(&mut agents, seed, player_count, 50_000) {
        Ok(result) => {
            println!("Game finished!");
            println!("  Winner: {}", result.winner);
            println!("  Rounds played: {}", result.rounds_played);
            println!();
            println!("  Final standings:");
            for pr in &result.player_results {
                println!("    {:10} — castles: {}, supply: {}, power: {}, IT:{} F:{} KC:{}",
                    pr.house.to_string(),
                    pr.final_castles,
                    pr.final_supply,
                    pr.final_power,
                    pr.final_iron_throne,
                    pr.final_fiefdoms,
                    pr.final_kings_court,
                );
            }
        }
        Err(e) => {
            eprintln!("Game error: {}", e);
        }
    }
}

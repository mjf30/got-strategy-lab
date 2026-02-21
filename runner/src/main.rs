// ═══════════════════════════════════════════════════════════════════════
// Runner — CLI entry point for running games and tournaments
// ═══════════════════════════════════════════════════════════════════════

use got_engine::types::HouseName;
use got_agents::{RandomAgent, HeuristicAgent};
use got_agents::Agent;
use got_tournament::{run_game, database::Database};
use std::collections::HashMap;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "got-runner", about = "Game of Thrones Strategy Lab")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a single game with random agents
    Play {
        #[arg(short, long, default_value_t = 42)]
        seed: u64,
        #[arg(short, long, default_value_t = 6)]
        players: u8,
        /// Agent type: "random" or "heuristic"
        #[arg(short, long, default_value = "random")]
        agent: String,
    },
    /// Run a tournament of N games
    Tournament {
        #[arg(short, long, default_value_t = 100)]
        games: u32,
        #[arg(short, long, default_value_t = 6)]
        players: u8,
        #[arg(short, long, default_value = "results.db")]
        db: String,
        /// Agent type: "random", "heuristic", or "mixed" (3 random + 3 heuristic)
        #[arg(short, long, default_value = "random")]
        agent: String,
    },
    /// Show leaderboard from database
    Leaderboard {
        #[arg(short, long, default_value = "results.db")]
        db: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Play { seed, players, agent } => cmd_play(seed, players, &agent),
        Commands::Tournament { games, players, db, agent } => cmd_tournament(games, players, &db, &agent),
        Commands::Leaderboard { db } => cmd_leaderboard(&db),
    }
}

fn cmd_play(seed: u64, player_count: u8, agent_type: &str) {
    println!("=== Game of Thrones Strategy Lab ===\n");
    println!("Running single game: seed={}, players={}, agent={}\n", seed, player_count, agent_type);

    let mut agents = make_agents(seed, player_count, agent_type);
    match run_game(&mut agents, seed, player_count, 50_000) {
        Ok(result) => {
            println!("Game finished!");
            println!("  Winner: {}", result.winner);
            println!("  Rounds played: {}", result.rounds_played);
            println!();
            println!("  Final standings:");
            for pr in &result.player_results {
                println!("    {:10} -- castles: {}, supply: {}, power: {}, IT:{} F:{} KC:{}",
                    pr.house.to_string(),
                    pr.final_castles, pr.final_supply, pr.final_power,
                    pr.final_iron_throne, pr.final_fiefdoms, pr.final_kings_court,
                );
            }
        }
        Err(e) => eprintln!("Game error: {}", e),
    }
}

fn cmd_tournament(num_games: u32, player_count: u8, db_path: &str, agent_type: &str) {
    println!("=== Tournament: {} games, {} players, agent={} ===\n", num_games, player_count, agent_type);

    let db = Database::new(db_path);

    // Register one agent per house (all random for now)
    let agent_name = "RandomAgent";
    let agent_id = db.register_agent(agent_name);

    let mut wins: HashMap<HouseName, u32> = HashMap::new();
    let mut errors = 0u32;

    for g in 0..num_games {
        let seed = 42u64 + g as u64 * 1000;
        let mut agents = make_agents(seed, player_count, agent_type);
        match run_game(&mut agents, seed, player_count, 50_000) {
            Ok(result) => {
                *wins.entry(result.winner).or_insert(0) += 1;

                // Store result
                let agent_ids: Vec<(String, i64)> = result.player_results.iter()
                    .map(|pr| (pr.house.to_string(), agent_id))
                    .collect();
                db.store_game(&result, &agent_ids);

                if (g + 1) % 10 == 0 || g + 1 == num_games {
                    print!("\rGame {}/{}...", g + 1, num_games);
                }
            }
            Err(e) => {
                errors += 1;
                eprintln!("Game {}: ERROR -- {}", g + 1, e);
            }
        }
    }

    println!("\n\n--- Summary ({} games, {} errors) ---", num_games, errors);
    for &house in &HouseName::ALL {
        let w = wins.get(&house).copied().unwrap_or(0);
        let pct = if num_games > 0 { w as f64 / num_games as f64 * 100.0 } else { 0.0 };
        println!("  {:10}: {:>4} wins ({:.1}%)", house.to_string(), w, pct);
    }
    println!("\nResults saved to: {}", db_path);
    println!("Total games in DB: {}", db.game_count());
}

fn cmd_leaderboard(db_path: &str) {
    let db = Database::new(db_path);
    let board = db.leaderboard();
    if board.is_empty() {
        println!("No agents found. Run some tournaments first.");
        return;
    }
    println!("=== Leaderboard ===\n");
    println!("{:<20} {:>8} {:>8} {:>8}", "Agent", "ELO", "Games", "Wins");
    println!("{}", "-".repeat(48));
    for (name, elo, games, wins_count) in &board {
        println!("{:<20} {:>8.1} {:>8} {:>8}", name, elo, games, wins_count);
    }
}

fn make_agents(seed: u64, player_count: u8, agent_type: &str) -> HashMap<HouseName, Box<dyn Agent>> {
    let mut agents: HashMap<HouseName, Box<dyn Agent>> = HashMap::new();
    for (i, &house) in HouseName::ALL.iter().take(player_count as usize).enumerate() {
        let agent: Box<dyn Agent> = match agent_type {
            "heuristic" => Box::new(HeuristicAgent::new(house, seed + i as u64)),
            "mixed" => {
                if i % 2 == 0 {
                    Box::new(HeuristicAgent::new(house, seed + i as u64))
                } else {
                    Box::new(RandomAgent::new(house, seed + i as u64))
                }
            }
            _ => Box::new(RandomAgent::new(house, seed + i as u64)),
        };
        agents.insert(house, agent);
    }
    agents
}

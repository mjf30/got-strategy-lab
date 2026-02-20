// ═══════════════════════════════════════════════════════════════════════
// Game Runner — runs a complete headless game with agents
// ═══════════════════════════════════════════════════════════════════════

use got_engine::types::*;
use got_engine::engine;
use got_engine::visibility::player_view;
use got_agents::Agent;
use std::collections::HashMap;

/// Result of a completed game.
#[derive(Debug, Clone)]
pub struct GameResult {
    pub seed: u64,
    pub winner: HouseName,
    pub rounds_played: u8,
    pub player_results: Vec<PlayerResult>,
}

#[derive(Debug, Clone)]
pub struct PlayerResult {
    pub house: HouseName,
    pub agent_name: String,
    pub final_castles: u8,
    pub final_supply: u8,
    pub final_power: u8,
    pub final_iron_throne: u8,
    pub final_fiefdoms: u8,
    pub final_kings_court: u8,
}

/// Run a complete game with the given agents.
/// Each agent in the map controls one house.
/// Returns the result when the game ends.
pub fn run_game(
    agents: &mut HashMap<HouseName, Box<dyn Agent>>,
    seed: u64,
    player_count: u8,
    max_decisions: usize, // safety limit to prevent infinite loops
) -> Result<GameResult, String> {
    let mut state = got_engine::setup::create_initial_state(player_count, seed);
    let mut decision_count = 0;

    // Main game loop
    loop {
        // Advance engine until it needs a decision or game ends
        engine::advance(&mut state);

        // Check game over
        if let Some(winner) = state.winner {
            return Ok(build_result(&state, seed, winner));
        }

        // If there's a pending decision, ask the appropriate agent
        if let Some(ref pending) = state.pending {
            let house = pending_house(pending);
            if let Some(agent) = agents.get_mut(&house) {
                let view = player_view(&state, house);
                let action = agent.decide(&view);
                engine::apply_action(&mut state, action);
                decision_count += 1;

                if decision_count > max_decisions {
                    return Err(format!(
                        "Game exceeded {} decisions without finishing (round {})",
                        max_decisions, state.round
                    ));
                }
            } else {
                return Err(format!("No agent for house {:?}", house));
            }
        } else if state.winner.is_none() {
            // No pending and no winner — shouldn't happen
            // Engine should always either set pending or advance
            return Err(format!(
                "Game stuck: phase={:?}, round={}, step={}",
                state.phase, state.round, state.westeros_step
            ));
        }
    }
}

fn pending_house(pending: &PendingDecision) -> HouseName {
    match pending {
        PendingDecision::PlaceOrders { house } => *house,
        PendingDecision::ChooseRaid { house, .. } => *house,
        PendingDecision::ChooseMarch { house, .. } => *house,
        PendingDecision::SupportDeclaration { house, .. } => *house,
        PendingDecision::SelectHouseCard { house, .. } => *house,
        PendingDecision::TyrionReplace { opponent } => *opponent,
        PendingDecision::AeronSwap { house } => *house,
        PendingDecision::PatchfaceDiscard { opponent, .. } => *opponent,
        PendingDecision::Retreat { house, .. } => *house,
        PendingDecision::Reconcile { house, .. } => *house,
        PendingDecision::Muster { house, .. } => *house,
        PendingDecision::MessengerRaven { house } => *house,
        PendingDecision::WildlingPenaltyChoice { house, .. } => *house,
        PendingDecision::CerseiRemoveOrder { opponent } => *opponent,
        PendingDecision::DoranChooseTrack { opponent } => *opponent,
        PendingDecision::QueenOfThornsRemoveOrder { opponent } => *opponent,
        PendingDecision::LeavePowerToken { house, .. } => *house,
        PendingDecision::UseValyrianBlade { house } => *house,
        PendingDecision::Bidding { house, .. } => *house,
        PendingDecision::WesterosChoice { chooser, .. } => *chooser,
        PendingDecision::RobbRetreat { house, .. } => *house,
    }
}

fn build_result(state: &GameState, seed: u64, winner: HouseName) -> GameResult {
    use got_engine::map::AREAS;

    let player_results: Vec<PlayerResult> = state.playing_houses.iter()
        .map(|&h| {
            let profile = state.house(h);
            let castles = state.areas.iter().enumerate()
                .filter(|(i, a)| a.house == Some(h) && AREAS[*i].has_castle_or_stronghold())
                .count() as u8;
            PlayerResult {
                house: h,
                agent_name: String::new(), // Filled by caller
                final_castles: castles,
                final_supply: profile.supply,
                final_power: profile.power,
                final_iron_throne: profile.iron_throne,
                final_fiefdoms: profile.fiefdoms,
                final_kings_court: profile.kings_court,
            }
        })
        .collect();

    GameResult {
        seed,
        winner,
        rounds_played: state.round,
        player_results,
    }
}

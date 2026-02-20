// ═══════════════════════════════════════════════════════════════════════
// Navigation — movement validation, ship transport
// Ported from TypeScript navigation.ts
// ═══════════════════════════════════════════════════════════════════════

use crate::types::*;
use crate::map::AREAS;
use std::collections::VecDeque;

/// Check if a move from one area to another is valid for a given house.
/// Considers direct adjacency and ship transport chains.
pub fn is_move_valid(state: &GameState, from: AreaId, to: AreaId, house: HouseName) -> bool {
    let from_def = &AREAS[from.0 as usize];
    let to_def = &AREAS[to.0 as usize];
    let to_state = &state.areas[to.0 as usize];

    // Blocked regions are impassable (3-player game)
    if to_state.blocked {
        return false;
    }

    // Direct adjacency
    if from_def.adjacent.contains(&to) {
        return true;
    }

    // Ship Transport: Land → (chain of friendly-ship seas) → Land
    // Units starting in a Port or Sea cannot use ship transport
    if !from_def.is_land() {
        return false;
    }
    if to_def.is_sea() {
        return false;
    }

    // BFS through seas with friendly ships
    let mut queue: VecDeque<AreaId> = VecDeque::new();
    let mut visited = vec![false; AREAS.len()];
    
    queue.push_back(from);
    visited[from.0 as usize] = true;

    while let Some(current) = queue.pop_front() {
        let current_def = &AREAS[current.0 as usize];

        for &adj_id in current_def.adjacent {
            if visited[adj_id.0 as usize] {
                continue;
            }

            let adj_def = &AREAS[adj_id.0 as usize];

            // Did we reach the destination?
            if adj_id == to {
                // Only valid if we came through a sea with friendly ship
                if current_def.is_sea() && has_friendly_ship(state, current, house) {
                    return true;
                }
                continue;
            }

            // Can we traverse through this area?
            if adj_def.is_sea() && has_friendly_ship(state, adj_id, house) {
                visited[adj_id.0 as usize] = true;
                queue.push_back(adj_id);
            }
        }
    }

    false
}

/// Check if an area has at least one friendly ship.
fn has_friendly_ship(state: &GameState, area: AreaId, house: HouseName) -> bool {
    state.areas[area.0 as usize].units.iter()
        .any(|u| u.unit_type == UnitType::Ship && u.house == house)
}

/// Get all valid move destinations for a house from a given area.
pub fn valid_destinations(state: &GameState, from: AreaId, house: HouseName) -> Vec<AreaId> {
    let mut destinations = Vec::new();
    for i in 0..AREAS.len() {
        let to = AreaId(i as u8);
        if to != from && is_move_valid(state, from, to, house) {
            destinations.push(to);
        }
    }
    destinations
}

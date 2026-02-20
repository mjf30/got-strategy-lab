// ═══════════════════════════════════════════════════════════════════════
// Supply mechanics — ported from TypeScript supply.ts
// ═══════════════════════════════════════════════════════════════════════

use crate::types::*;
use crate::map::AREAS;

/// Check if a house violates supply limits.
/// Returns true if supply is violated.
pub fn check_supply_violation(state: &GameState, house: HouseName) -> bool {
    let supply = state.house(house).supply.min(6);
    let limits = supply_limits(supply);

    // Collect armies: groups of 2+ units in same area
    let mut armies: Vec<u8> = Vec::new();
    for (_i, area_state) in state.areas.iter().enumerate() {
        if area_state.house == Some(house) && area_state.units.len() >= 2 {
            armies.push(area_state.units.len() as u8);
        }
    }

    // Sort descending to match biggest army to biggest slot
    armies.sort_unstable_by(|a, b| b.cmp(a));

    // More armies than slots? Violation.
    if armies.len() > limits.len() {
        return true;
    }

    // Each army must fit in its corresponding slot
    for (i, &army_size) in armies.iter().enumerate() {
        if army_size > limits[i] {
            return true;
        }
    }

    false
}

/// Check supply limits for all playing houses.
/// Returns a map of house → whether they are in violation.
pub fn check_all_supply_violations(state: &GameState) -> Vec<(HouseName, bool)> {
    state.playing_houses.iter()
        .map(|&h| (h, check_supply_violation(state, h)))
        .collect()
}

/// Calculate supply level for a house based on controlled supply icons.
pub fn calculate_supply(state: &GameState, house: HouseName) -> u8 {
    let mut total: u8 = 0;
    for (i, area_state) in state.areas.iter().enumerate() {
        if area_state.house == Some(house) {
            total += AREAS[i].supply_icons;
        }
    }
    total.min(6)
}

/// Find which armies violate supply limits and by how much.
/// Returns list of (area_id, current_size, max_allowed).
pub fn find_violations(state: &GameState, house: HouseName) -> Vec<(AreaId, u8, u8)> {
    let supply = state.house(house).supply.min(6);
    let limits = supply_limits(supply);

    // Collect armies sorted descending
    let mut armies: Vec<(AreaId, u8)> = Vec::new();
    for (i, area_state) in state.areas.iter().enumerate() {
        if area_state.house == Some(house) && area_state.units.len() >= 2 {
            armies.push((AreaId(i as u8), area_state.units.len() as u8));
        }
    }
    armies.sort_unstable_by(|a, b| b.1.cmp(&a.1));

    let mut violations = Vec::new();
    for (idx, &(area_id, size)) in armies.iter().enumerate() {
        let max = if idx < limits.len() { limits[idx] } else { 1 }; // max 1 if no slot
        if size > max {
            violations.push((area_id, size, max));
        }
    }
    violations
}

// ═══════════════════════════════════════════════════════════════════════
// Game Engine — core game loop and action resolution
// This is the main orchestrator that advances game state.
//
// Architecture:
//   The engine is a pure state machine. It never does I/O or calls agents.
//   Instead it sets `state.pending` to describe what decision is needed,
//   and the runner/tournament code reads that, asks the appropriate agent,
//   and feeds the answer back via `apply_action()`.
//
// Flow:
//   1. Runner calls `advance()` to push the game forward
//   2. Engine processes until it needs a player decision → sets pending
//   3. Runner reads pending, asks agent, calls `apply_action(state, action)`
//   4. Repeat until state.winner is Some
// ═══════════════════════════════════════════════════════════════════════

use crate::types::*;
use crate::map::*;
use crate::supply;
use crate::navigation;

/// Actions that agents can submit to resolve pending decisions.
#[derive(Debug, Clone)]
pub enum Action {
    /// Place orders: map of area_id → order token index (into ORDER_TOKENS)
    PlaceOrders(Vec<(AreaId, u8)>),

    /// Choose raid target (or None to skip)
    Raid(Option<AreaId>),

    /// March: destination area + which unit indices to move
    March { to: AreaId, unit_indices: Vec<usize> },

    /// Skip march (resolve order without moving)
    MarchSkip,

    /// Leave power token when vacating area
    LeavePowerToken(bool),

    /// Support declaration during combat
    DeclareSupport(SupportChoice),

    /// Select house card for combat
    SelectCard(HouseCardId),

    /// Use Valyrian Steel Blade (yes/no)
    UseValyrianBlade(bool),

    /// Submit bid (power tokens)
    Bid(u8),

    /// Westeros decision: index into options
    WesterosChoice(usize),

    /// Muster actions: list of (area_id, unit_type_to_build)
    Muster(Vec<(AreaId, MusterAction2)>),

    /// Choose retreat destination
    Retreat(AreaId),

    /// Reconcile: remove unit at (area_id, unit_index)
    Reconcile(AreaId, usize),

    /// Messenger Raven: swap order at area_id to new token index (or None to skip)
    MessengerRaven(Option<(AreaId, u8)>),

    /// Aeron Damphair: swap card or decline
    AeronSwap(Option<HouseCardId>),

    /// Tyrion replacement card
    TyrionReplace(HouseCardId),

    /// Patchface: choose card to discard from opponent
    PatchfaceDiscard(HouseCardId),

    /// Robb Stark: choose retreat area for defender
    RobbRetreat(AreaId),

    /// Cersei: choose area whose order to remove
    CerseiRemoveOrder(AreaId),

    /// Doran: choose track to move opponent to bottom
    DoranChooseTrack(Track),

    /// Queen of Thorns: choose area whose order to remove
    QueenOfThorns(AreaId),

    /// Wildling penalty choice (index into options)
    WildlingPenalty(usize),
}

#[derive(Debug, Clone)]
pub enum MusterAction2 {
    Build(UnitType),
    Upgrade, // Footman → Knight
}

/// Advance the game state. Processes automatic transitions until a
/// player decision is needed (sets state.pending) or the game ends.
pub fn advance(state: &mut GameState) {
    // If there's already a pending decision, do nothing — waiting for input
    if state.pending.is_some() || state.winner.is_some() {
        return;
    }

    match state.phase {
        Phase::Planning => {
            // Check if all houses have placed orders
            let all_placed = state.playing_houses.iter().all(|h| {
                // A house has placed orders if all areas it controls with units have orders
                state.areas.iter().enumerate().all(|(_i, area)| {
                    if area.house == Some(*h) && !area.units.is_empty() {
                        area.order.is_some()
                    } else {
                        true
                    }
                })
            });

            if !all_placed {
                // Find next house that needs to place orders
                for &h in &state.playing_houses {
                    let needs_orders = state.areas.iter().enumerate().any(|(_i, area)| {
                        area.house == Some(h) && !area.units.is_empty() && area.order.is_none()
                    });
                    if needs_orders {
                        state.pending = Some(PendingDecision::PlaceOrders { house: h });
                        return;
                    }
                }
            }

            // All orders placed → check messenger raven
            if !state.messenger_raven_used {
                let raven_holder = state.playing_houses.iter()
                    .find(|&&h| state.house(h).kings_court == 1);
                if let Some(&holder) = raven_holder {
                    state.messenger_raven_used = true;
                    state.pending = Some(PendingDecision::MessengerRaven { house: holder });
                    return;
                }
            }

            // Transition to Action phase
            state.phase = Phase::Action;
            state.action_sub_phase = ActionSubPhase::Raid;
            state.action_player_index = 0;
            advance(state); // recurse to process action phase
        }

        Phase::Action => {
            advance_action_phase(state);
        }

        Phase::Westeros => {
            // TODO: draw and resolve Westeros cards
            // For now, skip to Planning
            state.phase = Phase::Planning;
            advance(state);
        }

        Phase::Combat => {
            // Combat is handled through pending decisions
            // If we got here with no pending, combat resolution is done
            state.phase = Phase::Action;
            advance(state);
        }
    }
}

fn advance_action_phase(state: &mut GameState) {
    if state.pending.is_some() || state.winner.is_some() {
        return;
    }

    let player_count = state.playing_houses.len();

    loop {
        if state.action_sub_phase == ActionSubPhase::Done {
            // Clean up: remove all orders, unroute units, advance round
            cleanup_round(state);
            return;
        }

        if state.action_player_index as usize >= player_count {
            // All players done with this sub-phase, advance
            state.action_player_index = 0;
            state.action_sub_phase = match state.action_sub_phase {
                ActionSubPhase::Raid => ActionSubPhase::March,
                ActionSubPhase::March => ActionSubPhase::ConsolidatePower,
                ActionSubPhase::ConsolidatePower => ActionSubPhase::Done,
                ActionSubPhase::Done => unreachable!(),
            };
            continue;
        }

        let current_house = state.turn_order[state.action_player_index as usize];

        match state.action_sub_phase {
            ActionSubPhase::Raid => {
                // Find areas with raid orders for current house
                let raid_areas: Vec<AreaId> = state.areas.iter().enumerate()
                    .filter(|(_, a)| {
                        a.house == Some(current_house) &&
                        a.order.map_or(false, |o| o.order_type == OrderType::Raid)
                    })
                    .map(|(i, _)| AreaId(i as u8))
                    .collect();

                if raid_areas.is_empty() {
                    state.action_player_index += 1;
                    continue;
                }

                // For now, ask player for each raid area (simplified: first one)
                let from = raid_areas[0];
                let valid_targets = find_raid_targets(state, from, current_house);
                state.pending = Some(PendingDecision::ChooseRaid {
                    house: current_house,
                    from_area: from,
                    valid_targets,
                });
                return;
            }

            ActionSubPhase::March => {
                let march_areas: Vec<AreaId> = state.areas.iter().enumerate()
                    .filter(|(_, a)| {
                        a.house == Some(current_house) &&
                        a.order.map_or(false, |o| o.order_type == OrderType::March)
                    })
                    .map(|(i, _)| AreaId(i as u8))
                    .collect();

                if march_areas.is_empty() {
                    state.action_player_index += 1;
                    continue;
                }

                let from = march_areas[0];
                let valid_dests = navigation::valid_destinations(state, from, current_house);
                state.pending = Some(PendingDecision::ChooseMarch {
                    house: current_house,
                    from_area: from,
                    valid_destinations: valid_dests,
                });
                return;
            }

            ActionSubPhase::ConsolidatePower => {
                // Auto-resolve: collect power for each CP order
                resolve_consolidate_power(state, current_house);
                state.action_player_index += 1;
                continue;
            }

            ActionSubPhase::Done => unreachable!(),
        }
    }
}

/// Apply a player's action to resolve a pending decision.
pub fn apply_action(state: &mut GameState, action: Action) {
    let pending = state.pending.take();
    if pending.is_none() {
        return; // No pending decision
    }

    match (pending.unwrap(), action) {
        (PendingDecision::PlaceOrders { house }, Action::PlaceOrders(orders)) => {
            for (area_id, token_idx) in orders {
                let token = ORDER_TOKENS[token_idx as usize];
                state.area_mut(area_id).order = Some(Order {
                    order_type: token.order_type,
                    strength: token.strength,
                    star: token.star,
                    house,
                    token_index: token_idx,
                });
            }
            state.house_mut(house).used_order_tokens = Vec::new();
        }

        (PendingDecision::ChooseRaid { house: _, from_area, .. }, Action::Raid(target)) => {
            if let Some(target_id) = target {
                // Remove target's order
                state.area_mut(target_id).order = None;
                // If raiding a CP order, steal a power token
                // (simplified — full implementation in later commit)
            }
            // Remove own raid order
            state.area_mut(from_area).order = None;
            state.action_player_index += 1;
        }

        (PendingDecision::ChooseMarch { house, from_area, .. }, Action::March { to, unit_indices }) => {
            // Move units
            let moving_units: Vec<Unit> = unit_indices.iter()
                .map(|&i| state.area(from_area).units[i])
                .collect();

            // Remove units from source (reverse order to keep indices valid)
            let mut sorted_indices = unit_indices.clone();
            sorted_indices.sort_unstable_by(|a, b| b.cmp(a));
            for &i in &sorted_indices {
                state.area_mut(from_area).units.remove(i);
            }

            // Check for combat
            let target_house = state.area(to).house;
            if target_house.is_some() && target_house != Some(house) && !state.area(to).units.is_empty() {
                // Combat! Set up combat state
                let defending_units = state.area(to).units.clone();
                state.combat = Some(CombatState {
                    attacker: house,
                    defender: target_house.unwrap(),
                    area_id: to,
                    attacking_units: moving_units,
                    defending_units,
                    attacker_card: None,
                    defender_card: None,
                    attacker_strength: 0,
                    defender_strength: 0,
                    march_from_area: Some(from_area),
                    attacker_used_blade: false,
                    defender_used_blade: false,
                    support_decisions: std::collections::HashMap::new(),
                    phase: CombatPhase::Cards,
                    aeron_resolved: false,
                    tyrion_resolved: false,
                });
                state.phase = Phase::Combat;

                // Ask attacker for card
                let available = state.house(house).hand.clone();
                state.pending = Some(PendingDecision::SelectHouseCard {
                    house,
                    available_cards: available,
                });
            } else {
                // No combat — just place units
                for unit in moving_units {
                    state.area_mut(to).units.push(unit);
                }
                state.area_mut(to).house = Some(house);

                // Update source area control
                if state.area(from_area).units.is_empty() {
                    // May want to leave power token
                    if AREAS[from_area.0 as usize].is_land() {
                        state.pending = Some(PendingDecision::LeavePowerToken { area_id: from_area });
                        return;
                    }
                    state.area_mut(from_area).house = None;
                }

                // Remove march order
                state.area_mut(from_area).order = None;
                state.action_player_index += 1;

                // Check victory
                check_victory(state);
            }
        }

        (PendingDecision::ChooseMarch { from_area, .. }, Action::MarchSkip) => {
            state.area_mut(from_area).order = None;
            state.action_player_index += 1;
        }

        (PendingDecision::LeavePowerToken { area_id }, Action::LeavePowerToken(leave)) => {
            let house = state.area(area_id).house;
            if leave {
                if let Some(h) = house {
                    if state.house(h).power > 0 {
                        state.house_mut(h).power -= 1;
                        // Keep control
                    } else {
                        state.area_mut(area_id).house = None;
                    }
                }
            } else {
                state.area_mut(area_id).house = None;
            }
            state.action_player_index += 1;
        }

        (PendingDecision::SelectHouseCard { house, .. }, Action::SelectCard(card_id)) => {
            if let Some(combat) = &mut state.combat {
                if house == combat.attacker {
                    combat.attacker_card = Some(card_id);
                    // Now ask defender
                    let defender = combat.defender;
                    let available = state.house(defender).hand.clone();
                    state.pending = Some(PendingDecision::SelectHouseCard {
                        house: defender,
                        available_cards: available,
                    });
                } else {
                    combat.defender_card = Some(card_id);
                    // Both cards selected → resolve combat
                    resolve_combat(state);
                }
            }
            // Remove card from hand, add to discards
            let hand = &mut state.house_mut(house).hand;
            if let Some(pos) = hand.iter().position(|&c| c == card_id) {
                hand.remove(pos);
            }
            state.house_mut(house).discards.push(card_id);
        }

        (PendingDecision::Bidding { bidding_type: _, .. }, Action::Bid(_amount)) => {
            // Store bid — handled in full bidding resolution
            // TODO: full bidding implementation
        }

        (PendingDecision::MessengerRaven { house }, Action::MessengerRaven(swap)) => {
            if let Some((area_id, new_token_idx)) = swap {
                let token = ORDER_TOKENS[new_token_idx as usize];
                state.area_mut(area_id).order = Some(Order {
                    order_type: token.order_type,
                    strength: token.strength,
                    star: token.star,
                    house,
                    token_index: new_token_idx,
                });
            }
            // Continue to Action phase
        }

        (PendingDecision::Retreat { house, from_area, .. }, Action::Retreat(to)) => {
            // Move retreating units
            let units: Vec<Unit> = state.area(from_area).units.iter()
                .filter(|u| u.house == house)
                .cloned()
                .collect();
            state.area_mut(from_area).units.retain(|u| u.house != house);
            for mut unit in units {
                unit.routed = true;
                state.area_mut(to).units.push(unit);
            }
            if state.area(to).house.is_none() {
                state.area_mut(to).house = Some(house);
            }
            // Return to action phase
            state.combat = None;
            state.phase = Phase::Action;
            state.action_player_index += 1;
        }

        (PendingDecision::Reconcile { house, area_id: _, max_allowed: _, .. }, Action::Reconcile(aid, unit_idx)) => {
            let unit = state.area_mut(aid).units.remove(unit_idx);
            // Return unit to pool
            *state.house_mut(house).available_units.get_mut(unit.unit_type) += 1;
            // Check if still in violation
            if supply::check_supply_violation(state, house) {
                let violations = supply::find_violations(state, house);
                if let Some(&(vid, curr, max)) = violations.first() {
                    state.pending = Some(PendingDecision::Reconcile {
                        house, area_id: vid, current_size: curr, max_allowed: max,
                    });
                }
            }
        }

        _ => {
            // Unhandled combination — for now, just clear pending
            // Full implementation will handle all cases
        }
    }

    // After applying action, try to advance
    if state.pending.is_none() {
        advance(state);
    }
}

// ── Helper functions ───────────────────────────────────────────────────

fn find_raid_targets(state: &GameState, from: AreaId, house: HouseName) -> Vec<AreaId> {
    let from_def = &AREAS[from.0 as usize];
    let is_star = state.area(from).order.map_or(false, |o| o.star);

    from_def.adjacent.iter()
        .filter(|&&adj| {
            let area = state.area(adj);
            if area.house == Some(house) || area.house.is_none() {
                return false;
            }
            if let Some(order) = &area.order {
                match order.order_type {
                    OrderType::Raid | OrderType::Support | OrderType::ConsolidatePower => true,
                    OrderType::Defense if is_star => true, // star raid can remove defense (Raid★ only)
                    _ => false,
                }
            } else {
                false
            }
        })
        .copied()
        .collect()
}

fn resolve_consolidate_power(state: &mut GameState, house: HouseName) {
    let cp_areas: Vec<(AreaId, bool)> = state.areas.iter().enumerate()
        .filter(|(_, a)| {
            a.house == Some(house) &&
            a.order.map_or(false, |o| o.order_type == OrderType::ConsolidatePower)
        })
        .map(|(i, a)| (AreaId(i as u8), a.order.unwrap().star))
        .collect();

    for (area_id, _is_star) in cp_areas {
        let area_def = &AREAS[area_id.0 as usize];
        // Gain 1 power + 1 per power icon
        let power_gain = 1 + area_def.power_icons;
        state.house_mut(house).power += power_gain;

        // CP★ on castle/stronghold triggers mustering (TODO: full muster flow)

        // Remove order
        state.area_mut(area_id).order = None;
    }
}

fn resolve_combat(state: &mut GameState) {
    // Simplified combat resolution
    // Full implementation will handle all house card abilities

    // Extract all needed combat data upfront to avoid borrow conflicts
    let combat_data = if let Some(combat) = &state.combat {
        Some((
            combat.attacker,
            combat.defender,
            combat.area_id,
            combat.attacker_card,
            combat.defender_card,
            combat.attacking_units.clone(),
            combat.defending_units.clone(),
            combat.march_from_area,
        ))
    } else {
        None
    };

    let Some((attacker, defender, area_id, atk_card_id, def_card_id,
              attacking_units, defending_units, march_from_area)) = combat_data
    else {
        return;
    };

    let atk_card = atk_card_id.map(|id| crate::cards::get_house_card(id));
    let def_card = def_card_id.map(|id| crate::cards::get_house_card(id));

    // Base unit strength
    let atk_units: i16 = attacking_units.iter()
        .map(|u| u.unit_type.combat_strength() as i16)
        .sum();
    let def_units: i16 = defending_units.iter()
        .map(|u| u.unit_type.combat_strength() as i16)
        .sum();

    // Card strength
    let atk_card_str = atk_card.map_or(0, |c| c.strength as i16);
    let def_card_str = def_card.map_or(0, |c| c.strength as i16);

    // March order bonus
    let march_bonus = march_from_area
        .and_then(|from| state.area(from).order.map(|o| o.strength as i16))
        .unwrap_or(0);

    let atk_strength = atk_units + atk_card_str + march_bonus;
    let def_strength = def_units + def_card_str;

    // Write back computed strengths
    if let Some(combat) = &mut state.combat {
        combat.attacker_strength = atk_strength;
        combat.defender_strength = def_strength;
    }

    // Determine winner — fiefdoms tiebreaker (lower = better)
    let atk_fiefdoms = state.house(attacker).fiefdoms;
    let def_fiefdoms = state.house(defender).fiefdoms;
    let attacker_wins = atk_strength > def_strength
        || (atk_strength == def_strength && atk_fiefdoms < def_fiefdoms);

    if attacker_wins {
        // Place attacking units in conquered area
        state.area_mut(area_id).units.retain(|u| u.house != defender);
        for unit in &attacking_units {
            state.area_mut(area_id).units.push(*unit);
        }
        state.area_mut(area_id).house = Some(attacker);

        // Defender must retreat
        let retreat_options = find_retreat_areas(state, area_id, defender);
        if retreat_options.is_empty() {
            // Units destroyed — return to pool
            for unit in &defending_units {
                *state.house_mut(defender).available_units.get_mut(unit.unit_type) += 1;
            }
        } else {
            state.pending = Some(PendingDecision::Retreat {
                house: defender,
                units: defending_units,
                from_area: area_id,
                possible_areas: retreat_options,
            });
            check_victory(state);
            return;
        }
    } else {
        // Defender wins — attacker retreats
        let from = march_from_area.unwrap_or(area_id);

        // Return attacking units to origin
        for mut unit in attacking_units {
            unit.routed = true;
            state.area_mut(from).units.push(unit);
        }
    }

    state.combat = None;
    state.phase = Phase::Action;
    state.action_player_index += 1;
    check_victory(state);
}

fn find_retreat_areas(state: &GameState, from: AreaId, house: HouseName) -> Vec<AreaId> {
    let from_def = &AREAS[from.0 as usize];
    from_def.adjacent.iter()
        .filter(|&&adj| {
            let def = &AREAS[adj.0 as usize];
            let area = state.area(adj);
            def.is_land()
                && !area.blocked
                && (area.house.is_none() || area.house == Some(house))
                && area.units.iter().all(|u| u.house == house)
        })
        .copied()
        .collect()
}

fn cleanup_round(state: &mut GameState) {
    // Remove all remaining orders
    for area in &mut state.areas {
        area.order = None;
    }

    // Unroute all units
    for area in &mut state.areas {
        for unit in &mut area.units {
            unit.routed = false;
        }
    }

    // Reset token tracking
    let houses = state.playing_houses.clone();
    for h in &houses {
        state.house_mut(*h).used_order_tokens.clear();
    }
    state.valyrian_steel_blade_used = false;
    state.messenger_raven_used = false;
    state.order_restrictions.clear();
    state.star_order_restrictions.clear();

    // Advance round
    state.round += 1;

    if state.round > 10 {
        // Game over — determine winner by tiebreaker
        resolve_tiebreaker(state);
        return;
    }

    // Next round: Westeros phase (or Planning if round 1, but round 1 is already handled)
    state.phase = Phase::Westeros;
}

fn check_victory(state: &mut GameState) {
    for &h in &state.playing_houses {
        let castles = state.areas.iter().enumerate()
            .filter(|(i, a)| {
                a.house == Some(h) && AREAS[*i].has_castle_or_stronghold()
            })
            .count();
        if castles >= 7 {
            state.winner = Some(h);
            return;
        }
    }
}

fn resolve_tiebreaker(state: &mut GameState) {
    // Most castles/strongholds → highest supply → most power → best iron throne
    let mut rankings: Vec<(HouseName, usize, u8, u8, u8)> = state.playing_houses.iter()
        .map(|&h| {
            let castles = state.areas.iter().enumerate()
                .filter(|(i, a)| a.house == Some(h) && AREAS[*i].has_castle_or_stronghold())
                .count();
            let profile = state.house(h);
            (h, castles, profile.supply, profile.power, profile.iron_throne)
        })
        .collect();

    // Sort: most castles, then highest supply, then most power, then best (lowest) iron throne
    rankings.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then(b.2.cmp(&a.2))
            .then(b.3.cmp(&a.3))
            .then(a.4.cmp(&b.4))
    });

    state.winner = Some(rankings[0].0);
}

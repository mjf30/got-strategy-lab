// ═══════════════════════════════════════════════════════════════════════
// Visibility / Information Model
//
// In the physical board game, information is split between:
//   PUBLIC  — visible to all players at all times
//   PRIVATE — known only to the owning player  
//   HIDDEN  — unknown to all players (deck order)
//
// This module produces a "player view" of the game state that only
// contains information that player is legally allowed to know.
// Agents MUST only receive PlayerView, never the raw GameState.
// ═══════════════════════════════════════════════════════════════════════

use crate::types::*;
use crate::map::NUM_AREAS;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

// ── What is PUBLIC ─────────────────────────────────────────────────────
//
// • Board state: which house controls each area, what units are where
// • Influence track positions (Iron Throne, Fiefdoms, King's Court)
// • Supply levels for all houses
// • Power token COUNT for all houses (you can count them on the table)
// • Garrisons (static and placed)
// • Wildling threat level
// • Current round and phase
// • Westeros cards AFTER drawn (and which deck they came from)
// • House cards in DISCARD piles (played cards are face-up)
// • Orders AFTER reveal (orders are placed face-down, then all flipped)
// • Combat results (attacker, defender, cards played, strengths)
// • Number of cards remaining in each house's hand (but NOT which cards)
// • Turn order
//
// ── What is PRIVATE (per player) ───────────────────────────────────────
//
// • Your own hand of house cards (remaining unplayed cards)
// • Your orders BEFORE they are revealed (during Planning phase)
// • Your bid amount BEFORE all bids are revealed
//
// ── What is HIDDEN (unknown to everyone) ───────────────────────────────
//
// • Order of cards in Westeros decks (future draws)
// • Order of cards in Wildling deck
// • Other players' unrevealed orders (during Planning)
// • Other players' bids (during bidding, before reveal)
//

/// The view of the game state that a specific player is allowed to see.
/// This is what gets passed to an Agent's decision functions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerView {
    // ── Public info ────────────────────────────────────────
    pub viewer: HouseName,
    pub round: u8,
    pub phase: Phase,
    pub action_sub_phase: ActionSubPhase,
    pub wildling_threat: u8,
    pub turn_order: Vec<HouseName>,
    pub playing_houses: Vec<HouseName>,

    /// Public info about each house (no hand details for opponents).
    pub house_info: HashMap<HouseName, PublicHouseInfo>,

    /// Board state: units, orders (if revealed), control.
    pub areas: Vec<AreaView>,

    /// Garrisons on the board.
    pub garrisons: HashMap<AreaId, Garrison>,

    /// Active combat (if any) — all combat info is public once initiated.
    pub combat: Option<CombatState>,

    /// Current pending decision (if it involves this player).
    pub pending: Option<PendingDecision>,

    /// Dominance token status.
    pub valyrian_steel_blade_used: bool,
    pub messenger_raven_used: bool,

    /// Current order restrictions from Westeros cards.
    pub order_restrictions: Vec<OrderType>,
    pub star_order_restrictions: Vec<OrderType>,

    /// Winner (if game is over).
    pub winner: Option<HouseName>,

    // ── Private info (only for the viewer) ─────────────────
    /// Your own hand of house cards.
    pub my_hand: Vec<HouseCardId>,

    /// Your own unrevealed orders (during Planning phase, before reveal).
    /// Maps area_id → order. Empty if orders have been revealed.
    pub my_orders: HashMap<AreaId, Order>,
}

/// Public information about a house (visible to all players).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicHouseInfo {
    pub name: HouseName,
    pub iron_throne: u8,
    pub fiefdoms: u8,
    pub kings_court: u8,
    pub supply: u8,
    pub power: u8,
    /// Number of house cards remaining in hand (public knowledge).
    pub cards_in_hand: u8,
    /// Discarded (played) house cards — face-up, visible to everyone.
    pub discards: Vec<HouseCardId>,
    /// Available units in the pool (public — you can see the plastic pieces).
    pub available_units: UnitPool,
}

/// View of a single area on the board.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AreaView {
    pub id: AreaId,
    pub units: Vec<Unit>,
    /// Controlling house (public).
    pub house: Option<HouseName>,
    /// Order on this area. None if:
    /// - No order placed
    /// - Orders not yet revealed AND this isn't the viewer's area
    /// During Planning before reveal: only viewer's own orders are visible.
    /// After reveal: all orders are visible.
    pub order: Option<Order>,
    /// Whether an order exists but is hidden (face-down, not yet revealed).
    pub has_hidden_order: bool,
    pub blocked: bool,
}

/// Are orders currently revealed (visible to all)?
/// Orders are revealed after all players have placed them in the Planning phase.
/// During the Action phase and Combat, all orders are visible.
fn orders_are_revealed(state: &GameState) -> bool {
    matches!(state.phase, Phase::Action | Phase::Combat)
        || (state.phase == Phase::Westeros)
}

/// Build the PlayerView for a specific house.
pub fn player_view(state: &GameState, viewer: HouseName) -> PlayerView {
    let revealed = orders_are_revealed(state);

    // Build area views
    let mut area_views = Vec::with_capacity(NUM_AREAS);
    for (i, area_state) in state.areas.iter().enumerate() {
        let area_id = AreaId(i as u8);
        let is_mine = area_state.house == Some(viewer);

        let (order_visible, has_hidden) = if revealed {
            // After reveal: all orders visible
            (area_state.order, false)
        } else if is_mine {
            // Before reveal: only own orders visible
            (area_state.order, false)
        } else {
            // Before reveal: opponent orders hidden
            (None, area_state.order.is_some())
        };

        area_views.push(AreaView {
            id: area_id,
            units: area_state.units.clone(),
            house: area_state.house,
            order: order_visible,
            has_hidden_order: has_hidden,
            blocked: area_state.blocked,
        });
    }

    // Build public house info
    let mut house_info = HashMap::new();
    for (&h, profile) in &state.houses {
        house_info.insert(h, PublicHouseInfo {
            name: h,
            iron_throne: profile.iron_throne,
            fiefdoms: profile.fiefdoms,
            kings_court: profile.kings_court,
            supply: profile.supply,
            power: profile.power,
            cards_in_hand: profile.hand.len() as u8,
            discards: profile.discards.clone(),
            available_units: profile.available_units,
        });
    }

    // Viewer's own private info
    let my_hand = state.houses[&viewer].hand.clone();

    // Viewer's own orders (during Planning, before reveal)
    let mut my_orders = HashMap::new();
    if !revealed {
        for (i, area_state) in state.areas.iter().enumerate() {
            if area_state.house == Some(viewer) {
                if let Some(order) = &area_state.order {
                    my_orders.insert(AreaId(i as u8), *order);
                }
            }
        }
    }

    // Pending decision: only pass it if it involves the viewer
    let pending = state.pending.as_ref().and_then(|p| {
        if pending_involves(p, viewer) { Some(p.clone()) } else { None }
    });

    PlayerView {
        viewer,
        round: state.round,
        phase: state.phase,
        action_sub_phase: state.action_sub_phase,
        wildling_threat: state.wildling_threat,
        turn_order: state.turn_order.clone(),
        playing_houses: state.playing_houses.clone(),
        house_info,
        areas: area_views,
        garrisons: state.garrisons.clone(),
        combat: state.combat.clone(),
        pending,
        valyrian_steel_blade_used: state.valyrian_steel_blade_used,
        messenger_raven_used: state.messenger_raven_used,
        order_restrictions: state.order_restrictions.clone(),
        star_order_restrictions: state.star_order_restrictions.clone(),
        winner: state.winner,
        my_hand,
        my_orders,
    }
}

/// Check if a pending decision involves a specific house.
fn pending_involves(pending: &PendingDecision, house: HouseName) -> bool {
    match pending {
        PendingDecision::WesterosChoice { chooser, .. } => *chooser == house,
        PendingDecision::SupportDeclaration { house: h, .. } => *h == house,
        PendingDecision::TyrionReplace { opponent } => *opponent == house,
        PendingDecision::AeronSwap { house: h } => *h == house,
        PendingDecision::PatchfaceDiscard { .. } => {
            // Patchface is decided by the Baratheon player (winner)
            // But the cards shown are the opponent's — still the decision-maker sees them
            // The actual decider is the Patchface player
            true // Simplified: let the agent framework handle filtering
        }
        PendingDecision::RobbRetreat { .. } => true,
        PendingDecision::Retreat { house: h, .. } => *h == house,
        PendingDecision::Reconcile { house: h, .. } => *h == house,
        PendingDecision::Muster { house: h, .. } => *h == house,
        PendingDecision::Bidding { .. } => true, // All houses bid simultaneously
        PendingDecision::LeavePowerToken { .. } => true,
        PendingDecision::UseValyrianBlade => true,
        PendingDecision::PlaceOrders { house: h } => *h == house,
        PendingDecision::ChooseRaid { house: h, .. } => *h == house,
        PendingDecision::ChooseMarch { house: h, .. } => *h == house,
        PendingDecision::SelectHouseCard { house: h, .. } => *h == house,
        PendingDecision::MessengerRaven { house: h } => *h == house,
        PendingDecision::WildlingPenaltyChoice { house: h, .. } => *h == house,
        PendingDecision::CerseiRemoveOrder { .. } => true,
        PendingDecision::DoranChooseTrack { .. } => true,
        PendingDecision::QueenOfThornsRemoveOrder { .. } => true,
    }
}

/// Derive what cards an opponent MIGHT have in hand (public deduction).
/// Since all 7 cards per house are known, and discards are public,
/// the remaining cards in hand = initial set - discards.
/// This is legal public information (any player can deduce it).
pub fn possible_hand(state: &GameState, house: HouseName) -> Vec<HouseCardId> {
    let all_cards = crate::cards::all_house_card_ids(house);
    let discards = &state.houses[&house].discards;
    all_cards.into_iter().filter(|c| !discards.contains(c)).collect()
}

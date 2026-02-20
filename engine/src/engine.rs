// ═══════════════════════════════════════════════════════════════════════
// Game Engine — core game loop and action resolution
//
// Architecture:
//   Pure state machine. Never does I/O or calls agents.
//   Sets `state.pending` to describe what decision is needed,
//   and the runner feeds answers back via `apply_action()`.
//
// Flow: advance() → pending set → agent decide → apply_action() → repeat
// ═══════════════════════════════════════════════════════════════════════

use crate::types::*;
use crate::map::*;
use crate::supply;
use crate::navigation;
use crate::cards;
use std::collections::HashMap;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

// ── Action enum ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Action {
    PlaceOrders(Vec<(AreaId, u8)>),
    Raid(Option<AreaId>),
    March { to: AreaId, unit_indices: Vec<usize> },
    MarchSkip,
    LeavePowerToken(bool),
    DeclareSupport(SupportChoice),
    SelectCard(HouseCardId),
    UseValyrianBlade(bool),
    Bid(u8),
    WesterosChoice(usize),
    Muster(Vec<(AreaId, MusterAction2)>),
    Retreat(AreaId),
    Reconcile(AreaId, usize),
    MessengerRaven(Option<(AreaId, u8)>),
    AeronSwap(Option<HouseCardId>),
    TyrionReplace(HouseCardId),
    PatchfaceDiscard(HouseCardId),
    RobbRetreat(AreaId),
    CerseiRemoveOrder(AreaId),
    DoranChooseTrack(Track),
    QueenOfThorns(AreaId),
    WildlingPenalty(usize),
}

#[derive(Debug, Clone)]
pub enum MusterAction2 {
    Build(UnitType),
    Upgrade, // Footman → Knight
}

// ── Helpers ────────────────────────────────────────────────────────────

fn next_rng(state: &mut GameState) -> ChaCha8Rng {
    state.rng_counter += 1;
    ChaCha8Rng::seed_from_u64(state.seed.wrapping_add(state.rng_counter.wrapping_mul(6364136223846793005)))
}

fn find_track_holder(state: &GameState, track: Track) -> HouseName {
    state.playing_houses.iter()
        .find(|&&h| match track {
            Track::IronThrone => state.house(h).iron_throne == 1,
            Track::Fiefdoms => state.house(h).fiefdoms == 1,
            Track::KingsCourt => state.house(h).kings_court == 1,
        })
        .copied()
        .unwrap_or(state.turn_order[0])
}

fn get_muster_areas(state: &GameState, house: HouseName) -> Vec<MusterArea> {
    state.areas.iter().enumerate()
        .filter(|(i, area)| {
            area.house == Some(house) && AREAS[*i].has_castle_or_stronghold() && !area.blocked
        })
        .map(|(i, _)| MusterArea {
            area_id: AreaId(i as u8),
            points: AREAS[i].muster_points(),
        })
        .collect()
}

// ═══════════════════════════════════════════════════════════════════════
// MAIN ADVANCE — push state forward until a decision is needed
// ═══════════════════════════════════════════════════════════════════════

pub fn advance(state: &mut GameState) {
    if state.pending.is_some() || state.winner.is_some() {
        return;
    }
    match state.phase {
        Phase::Westeros => advance_westeros(state),
        Phase::Planning => advance_planning(state),
        Phase::Action   => advance_action_phase(state),
        Phase::Combat   => advance_combat(state),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// WESTEROS PHASE — draw cards, resolve effects
// ═══════════════════════════════════════════════════════════════════════

fn advance_westeros(state: &mut GameState) {
    // Handle ongoing bidding
    if state.bidding.is_some() {
        advance_bidding(state);
        if state.pending.is_some() || state.bidding.is_some() {
            return;
        }
        // Bidding fully complete → fall through to continue cards
    }

    // Handle ongoing mustering
    if state.muster_house_idx > 0 {
        advance_mustering_step(state);
        if state.pending.is_some() || state.muster_house_idx > 0 {
            return;
        }
        // Mustering fully complete → fall through
    }

    // Step 0: Draw cards
    if state.westeros_step == 0 {
        draw_westeros_cards(state);
        state.westeros_step = 1;
    }

    // Steps 1–3: Resolve each deck's card
    // Increment step BEFORE resolving so re-entry skips processed cards
    while state.westeros_step >= 1 && state.westeros_step <= 3 {
        let idx = (state.westeros_step - 1) as usize;
        if idx >= state.westeros_cards_drawn.len() {
            state.westeros_step += 1;
            continue;
        }
        let card = state.westeros_cards_drawn[idx];
        state.westeros_step += 1; // advance BEFORE resolving

        resolve_westeros_card(state, card);

        if state.pending.is_some() {
            return;
        }
        if state.bidding.is_some() {
            advance_bidding(state);
            if state.pending.is_some() || state.bidding.is_some() {
                return;
            }
        }
        if state.muster_house_idx > 0 {
            advance_mustering_step(state);
            if state.pending.is_some() || state.muster_house_idx > 0 {
                return;
            }
        }
    }

    // All cards resolved → Planning phase
    state.westeros_cards_drawn.clear();
    state.westeros_step = 0;
    state.phase = Phase::Planning;
    advance(state);
}

/// Advance mustering: ask next house that has muster areas, or finish.
fn advance_mustering_step(state: &mut GameState) {
    let houses = state.playing_houses.clone();
    loop {
        let idx = (state.muster_house_idx - 1) as usize;
        if idx >= houses.len() {
            state.muster_house_idx = 0;
            return;
        }
        let house = houses[idx];
        let muster_areas = get_muster_areas(state, house);
        if muster_areas.is_empty() {
            state.muster_house_idx += 1;
            continue;
        }
        state.pending = Some(PendingDecision::Muster { house, areas: muster_areas });
        return;
    }
}

fn draw_westeros_cards(state: &mut GameState) {
    state.westeros_cards_drawn.clear();
    let mut wildling_icons = 0u8;

    if let Some(c) = state.westeros_deck_1.pop() {
        if c.wildling_icon { wildling_icons += 1; }
        state.westeros_cards_drawn.push(c);
    }
    if let Some(c) = state.westeros_deck_2.pop() {
        if c.wildling_icon { wildling_icons += 1; }
        state.westeros_cards_drawn.push(c);
    }
    if let Some(c) = state.westeros_deck_3.pop() {
        if c.wildling_icon { wildling_icons += 1; }
        state.westeros_cards_drawn.push(c);
    }

    state.wildling_threat = (state.wildling_threat + wildling_icons * 2).min(12);
}

fn resolve_westeros_card(state: &mut GameState, card: WesterosCard) {
    use WesterosCardType::*;
    match card.card_type {
        Supply => resolve_supply_update(state),

        Mustering => {
            state.muster_house_idx = 1; // triggers mustering flow in advance_westeros
        }

        AThroneOfBlades => {
            let holder = find_track_holder(state, Track::IronThrone);
            state.pending = Some(PendingDecision::WesterosChoice {
                card_name: "A Throne of Blades".into(),
                chooser: holder,
                options: vec!["Mustering".into(), "Supply".into()],
            });
        }

        ClashOfKings => begin_clash_of_kings(state),

        GameOfThrones => resolve_game_of_thrones(state),

        DarkWingsDarkWords => {
            let holder = find_track_holder(state, Track::KingsCourt);
            state.pending = Some(PendingDecision::WesterosChoice {
                card_name: "Dark Wings, Dark Words".into(),
                chooser: holder,
                options: vec!["Clash of Kings".into(), "Game of Thrones".into()],
            });
        }

        WildlingAttack => begin_wildling_attack(state),

        PutToTheSword => {
            let holder = find_track_holder(state, Track::Fiefdoms);
            state.pending = Some(PendingDecision::WesterosChoice {
                card_name: "Put to the Sword".into(),
                chooser: holder,
                options: vec![
                    "Ban March★".into(),
                    "Ban Defense★".into(),
                    "Ban Support★".into(),
                    "Ban Raid★".into(),
                    "Ban CP★".into(),
                ],
            });
        }

        SeaOfStorms    => { state.order_restrictions.push(OrderType::Raid); }
        FeastForCrows  => { state.order_restrictions.push(OrderType::ConsolidatePower); }
        WebOfLies      => { state.order_restrictions.push(OrderType::Support); }
        StormOfSwords  => { state.order_restrictions.push(OrderType::Defense); }
        RainsOfAutumn  => { state.star_order_restrictions.push(OrderType::March); }

        WinterIsComing => {
            let mut rng = next_rng(state);
            let new_card = match card.deck {
                1 => { state.westeros_deck_1.shuffle(&mut rng); state.westeros_deck_1.pop() }
                2 => { state.westeros_deck_2.shuffle(&mut rng); state.westeros_deck_2.pop() }
                3 => { state.westeros_deck_3.shuffle(&mut rng); state.westeros_deck_3.pop() }
                _ => None,
            };
            if let Some(nc) = new_card {
                // westeros_step was already incremented before resolving, so current index = step - 2
                let s = (state.westeros_step.saturating_sub(2)) as usize;
                if s < state.westeros_cards_drawn.len() {
                    state.westeros_cards_drawn[s] = nc;
                }
                if nc.wildling_icon {
                    state.wildling_threat = (state.wildling_threat + 2).min(12);
                }
                resolve_westeros_card(state, nc);
            }
        }

        LastDaysOfSummer => {} // No effect
    }
}

fn resolve_supply_update(state: &mut GameState) {
    let houses = state.playing_houses.clone();
    for &h in &houses {
        let new_supply = supply::calculate_supply(state, h);
        state.house_mut(h).supply = new_supply;
    }
    // Check for supply violations → reconcile
    for &h in &houses {
        if supply::check_supply_violation(state, h) {
            let violations = supply::find_violations(state, h);
            if let Some(&(vid, curr, max)) = violations.first() {
                state.pending = Some(PendingDecision::Reconcile {
                    house: h, area_id: vid, current_size: curr, max_allowed: max,
                });
                return;
            }
        }
    }
}

fn resolve_game_of_thrones(state: &mut GameState) {
    let houses = state.playing_houses.clone();
    for &h in &houses {
        let mut power_gain: u8 = 0;
        for (i, area) in state.areas.iter().enumerate() {
            if area.house == Some(h) {
                power_gain += AREAS[i].power_icons;
            }
        }
        state.house_mut(h).power += power_gain;
    }
}

// ═══════════════════════════════════════════════════════════════════════
// BIDDING — Clash of Kings and Wildling attacks
// ═══════════════════════════════════════════════════════════════════════

fn begin_clash_of_kings(state: &mut GameState) {
    state.bidding = Some(BiddingState {
        bidding_type: BiddingType::IronThrone,
        bids: HashMap::new(),
        current_track: Some(Track::IronThrone),
        remaining_tracks: vec![Track::Fiefdoms, Track::KingsCourt],
        bid_order: state.turn_order.clone(),
        next_bidder_idx: 0,
    });
}

fn begin_wildling_attack(state: &mut GameState) {
    // If wildling threat is 0, nothing happens
    if state.wildling_threat == 0 {
        return;
    }
    state.bidding = Some(BiddingState {
        bidding_type: BiddingType::Wildling,
        bids: HashMap::new(),
        current_track: None,
        remaining_tracks: Vec::new(),
        bid_order: state.turn_order.clone(),
        next_bidder_idx: 0,
    });
}

fn advance_bidding(state: &mut GameState) {
    let bidding = state.bidding.as_ref().unwrap();
    let bt = bidding.bidding_type;
    let track = bidding.current_track;

    if bidding.next_bidder_idx < bidding.bid_order.len() {
        let house = bidding.bid_order[bidding.next_bidder_idx];
        state.pending = Some(PendingDecision::Bidding {
            house,
            bidding_type: bt,
            track,
        });
        return;
    }

    // All bids collected → resolve
    match bt {
        BiddingType::Wildling => {
            resolve_wildling_bidding(state);
        }
        _ => {
            resolve_track_bidding(state);
        }
    }

    // If resolve created new bidding (next track), immediately set pending
    if state.bidding.is_some() {
        advance_bidding(state);
    }
}

fn resolve_track_bidding(state: &mut GameState) {
    let bidding = state.bidding.take().unwrap();
    let track = bidding.current_track.unwrap();

    // Sort houses by bid (descending), tiebreak by current position (ascending = better)
    let mut sorted: Vec<(HouseName, u8, u8)> = bidding.bid_order.iter()
        .map(|&h| {
            let bid = *bidding.bids.get(&h).unwrap_or(&0);
            let current_pos = match track {
                Track::IronThrone => state.house(h).iron_throne,
                Track::Fiefdoms => state.house(h).fiefdoms,
                Track::KingsCourt => state.house(h).kings_court,
            };
            (h, bid, current_pos)
        })
        .collect();

    sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.2.cmp(&b.2)));

    // Assign new positions (1-based)
    let _houses_clone = state.playing_houses.clone();
    for (pos, &(h, bid, _)) in sorted.iter().enumerate() {
        let position = (pos + 1) as u8;
        match track {
            Track::IronThrone => state.house_mut(h).iron_throne = position,
            Track::Fiefdoms => state.house_mut(h).fiefdoms = position,
            Track::KingsCourt => state.house_mut(h).kings_court = position,
        }
        // Deduct bid tokens
        state.house_mut(h).power = state.house(h).power.saturating_sub(bid);
    }

    // Update turn order if Iron Throne changed
    if track == Track::IronThrone {
        let mut new_order = state.playing_houses.clone();
        new_order.sort_by_key(|&h| state.house(h).iron_throne);
        state.turn_order = new_order;
    }

    // Continue to next track or finish
    if !bidding.remaining_tracks.is_empty() {
        let mut remaining = bidding.remaining_tracks;
        let next_track = remaining.remove(0);
        let bt = match next_track {
            Track::IronThrone => BiddingType::IronThrone,
            Track::Fiefdoms => BiddingType::Fiefdoms,
            Track::KingsCourt => BiddingType::KingsCourt,
        };
        state.bidding = Some(BiddingState {
            bidding_type: bt,
            bids: HashMap::new(),
            current_track: Some(next_track),
            remaining_tracks: remaining,
            bid_order: state.turn_order.clone(),
            next_bidder_idx: 0,
        });
    }
    // else: bidding is done (already set to None by take())
}

fn resolve_wildling_bidding(state: &mut GameState) {
    let bidding = state.bidding.take().unwrap();
    let total_bid: u8 = bidding.bids.values().sum();
    let threat = state.wildling_threat;

    // Find highest and lowest bidders
    let mut sorted: Vec<(HouseName, u8)> = bidding.bid_order.iter()
        .map(|&h| (h, *bidding.bids.get(&h).unwrap_or(&0)))
        .collect();

    // Deduct all bids
    for &(h, bid) in &sorted {
        state.house_mut(h).power = state.house(h).power.saturating_sub(bid);
    }

    if total_bid >= threat {
        // Night's Watch wins!
        // Highest bidder (tiebreak: first in turn order) gets a reward
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        let highest = sorted[0].0;

        // Draw wildling card for specific effect
        let wildling_card = state.wildling_deck.pop();
        if let Some(_wc) = wildling_card {
            // Simplified reward: highest bidder gains 2 power back
            state.house_mut(highest).power += 2;
        }

        state.wildling_threat = 0;
    } else {
        // Wildlings win!
        // Lowest bidder (tiebreak: worst Iron Throne position) gets worst penalty
        sorted.sort_by(|a, b| a.1.cmp(&b.1).then(
            state.house(b.0).iron_throne.cmp(&state.house(a.0).iron_throne)
        ));
        let lowest = sorted[0].0;

        // Draw wildling card
        let _wildling_card = state.wildling_deck.pop();

        // Simplified penalty: lowest bidder loses 2 power and all others lose 1
        state.house_mut(lowest).power = state.house(lowest).power.saturating_sub(2);
        for &(h, _) in &sorted[1..] {
            state.house_mut(h).power = state.house(h).power.saturating_sub(1);
        }

        state.wildling_threat = 2;
    }
}

// ═══════════════════════════════════════════════════════════════════════
// PLANNING PHASE — place orders, messenger raven
// ═══════════════════════════════════════════════════════════════════════

fn advance_planning(state: &mut GameState) {
    if state.pending.is_some() || state.winner.is_some() {
        return;
    }

    // Find next house that needs to place orders (in turn order)
    for &h in &state.turn_order.clone() {
        let needs_orders = state.areas.iter().enumerate().any(|(_, area)| {
            area.house == Some(h) && !area.units.is_empty() && area.order.is_none()
        });
        if needs_orders {
            state.pending = Some(PendingDecision::PlaceOrders { house: h });
            return;
        }
    }

    // All orders placed → messenger raven
    if !state.messenger_raven_used {
        let raven_holder = find_track_holder(state, Track::KingsCourt);
        state.messenger_raven_used = true;
        state.pending = Some(PendingDecision::MessengerRaven { house: raven_holder });
        return;
    }

    // Transition to Action phase
    state.phase = Phase::Action;
    state.action_sub_phase = ActionSubPhase::Raid;
    state.action_player_index = 0;
    advance(state);
}

// ═══════════════════════════════════════════════════════════════════════
// ACTION PHASE — Raid, March, Consolidate Power (cycling turn order)
// ═══════════════════════════════════════════════════════════════════════

fn advance_action_phase(state: &mut GameState) {
    if state.pending.is_some() || state.winner.is_some() {
        return;
    }

    let player_count = state.playing_houses.len();

    loop {
        if state.action_sub_phase == ActionSubPhase::Done {
            cleanup_round(state);
            if state.winner.is_none() {
                advance(state);
            }
            return;
        }

        // Cycle through players looking for one with orders of current type
        let start_idx = state.action_player_index;
        let mut found = false;
        let mut checked = 0;

        loop {
            if checked >= player_count {
                break; // Full cycle, nobody has orders
            }
            let idx = ((start_idx as usize + checked) % player_count) as u8;
            let house = state.turn_order[idx as usize];
            checked += 1;

            match state.action_sub_phase {
                ActionSubPhase::Raid => {
                    let raid_area = find_first_order_area(state, house, OrderType::Raid);
                    if let Some(from) = raid_area {
                        state.action_player_index = idx;
                        let valid_targets = find_raid_targets(state, from, house);
                        state.pending = Some(PendingDecision::ChooseRaid {
                            house,
                            from_area: from,
                            valid_targets,
                        });
                        return;
                    }
                }
                ActionSubPhase::March => {
                    let march_area = find_first_order_area(state, house, OrderType::March);
                    if let Some(from) = march_area {
                        state.action_player_index = idx;
                        let valid_dests = navigation::valid_destinations(state, from, house);
                        state.pending = Some(PendingDecision::ChooseMarch {
                            house,
                            from_area: from,
                            valid_destinations: valid_dests,
                        });
                        return;
                    }
                }
                ActionSubPhase::ConsolidatePower => {
                    let cp_area = find_first_order_area(state, house, OrderType::ConsolidatePower);
                    if let Some(area_id) = cp_area {
                        state.action_player_index = idx;
                        resolve_single_consolidate_power(state, house, area_id);
                        // After resolving, advance to next player
                        state.action_player_index = (idx + 1) % player_count as u8;
                        found = true;
                        break;
                    }
                }
                ActionSubPhase::Done => unreachable!(),
            }
        }

        // If ConsolidatePower just resolved one, loop again to find next
        if found && state.action_sub_phase == ActionSubPhase::ConsolidatePower {
            continue;
        }

        // No one had orders → advance sub-phase
        state.action_player_index = 0;
        state.action_sub_phase = match state.action_sub_phase {
            ActionSubPhase::Raid => ActionSubPhase::March,
            ActionSubPhase::March => ActionSubPhase::ConsolidatePower,
            ActionSubPhase::ConsolidatePower => ActionSubPhase::Done,
            ActionSubPhase::Done => unreachable!(),
        };
    }
}

fn find_first_order_area(state: &GameState, house: HouseName, order_type: OrderType) -> Option<AreaId> {
    state.areas.iter().enumerate()
        .find(|(_, a)| {
            a.house == Some(house) &&
            a.order.map_or(false, |o| o.order_type == order_type)
        })
        .map(|(i, _)| AreaId(i as u8))
}

fn resolve_single_consolidate_power(state: &mut GameState, house: HouseName, area_id: AreaId) {
    let area_def = &AREAS[area_id.0 as usize];
    let is_star = state.area(area_id).order.map_or(false, |o| o.star);

    if is_star && area_def.has_castle_or_stronghold() {
        // CP★ on castle/stronghold → mustering at this area
        let muster_area = MusterArea {
            area_id,
            points: area_def.muster_points(),
        };
        state.area_mut(area_id).order = None;
        state.pending = Some(PendingDecision::Muster {
            house,
            areas: vec![muster_area],
        });
    } else {
        // Regular CP: gain 1 power + power icons
        let power_gain = 1 + area_def.power_icons;
        state.house_mut(house).power += power_gain;
        state.area_mut(area_id).order = None;
    }
}

// ═══════════════════════════════════════════════════════════════════════
// COMBAT — support, cards, blade, resolution, casualties, retreat
// ═══════════════════════════════════════════════════════════════════════

fn advance_combat(state: &mut GameState) {
    if state.pending.is_some() || state.winner.is_some() {
        return;
    }

    let combat_data = match &state.combat {
        Some(c) => (c.phase, c.attacker, c.defender, c.area_id),
        None => {
            state.phase = Phase::Action;
            advance(state);
            return;
        }
    };
    let (combat_phase, attacker, defender, _area_id) = combat_data;

    match combat_phase {
        CombatPhase::Support => {
            // Find next house to ask about support
            let pending_support = state.combat.as_ref().unwrap().pending_support_houses.clone();
            if let Some(&(sup_area, sup_house)) = pending_support.first() {
                state.pending = Some(PendingDecision::SupportDeclaration {
                    house: sup_house,
                    area_id: sup_area,
                    attacker,
                    defender,
                });
                return;
            }
            // All support declared → Cards phase
            if let Some(c) = &mut state.combat {
                c.phase = CombatPhase::Cards;
            }
            advance_combat(state);
        }

        CombatPhase::Cards => {
            let atk_card = state.combat.as_ref().unwrap().attacker_card;
            let def_card = state.combat.as_ref().unwrap().defender_card;

            if atk_card.is_none() {
                let available = state.house(attacker).hand.clone();
                if available.is_empty() {
                    // No cards → skip
                    if let Some(c) = &mut state.combat { c.attacker_card = Some(HouseCardId::EddardStark); } // placeholder
                    advance_combat(state);
                } else {
                    state.pending = Some(PendingDecision::SelectHouseCard {
                        house: attacker,
                        available_cards: available,
                    });
                }
                return;
            }

            if def_card.is_none() {
                let available = state.house(defender).hand.clone();
                if available.is_empty() {
                    if let Some(c) = &mut state.combat { c.defender_card = Some(HouseCardId::EddardStark); }
                    advance_combat(state);
                } else {
                    state.pending = Some(PendingDecision::SelectHouseCard {
                        house: defender,
                        available_cards: available,
                    });
                }
                return;
            }

            // Both cards selected → PreCombat (Tyrion, Aeron)
            if let Some(c) = &mut state.combat {
                c.phase = CombatPhase::PreCombat;
            }
            advance_combat(state);
        }

        CombatPhase::PreCombat => {
            // Check Tyrion Lannister (cancel opponent's card)
            let combat = state.combat.as_ref().unwrap();
            if !combat.tyrion_resolved {
                let atk_card = combat.attacker_card.unwrap();
                let def_card = combat.defender_card.unwrap();

                // Check if attacker played Tyrion
                if atk_card == HouseCardId::TyrionLannister {
                    if let Some(c) = &mut state.combat { c.tyrion_resolved = true; }
                    // Cancel defender's card — ask them to choose a replacement
                    let old_card = def_card;
                    // Return cancelled card to hand
                    state.house_mut(defender).hand.push(old_card);
                    if let Some(pos) = state.house(defender).discards.iter().position(|&c| c == old_card) {
                        state.house_mut(defender).discards.remove(pos);
                    }
                    if let Some(c) = &mut state.combat { c.defender_card = None; }
                    let _available = state.house(defender).hand.clone();
                    state.pending = Some(PendingDecision::TyrionReplace { opponent: defender });
                    return;
                }

                // Check if defender played Tyrion
                if def_card == HouseCardId::TyrionLannister {
                    if let Some(c) = &mut state.combat { c.tyrion_resolved = true; }
                    let old_card = atk_card;
                    state.house_mut(attacker).hand.push(old_card);
                    if let Some(pos) = state.house(attacker).discards.iter().position(|&c| c == old_card) {
                        state.house_mut(attacker).discards.remove(pos);
                    }
                    if let Some(c) = &mut state.combat { c.attacker_card = None; }
                    state.pending = Some(PendingDecision::TyrionReplace { opponent: attacker });
                    return;
                }

                if let Some(c) = &mut state.combat { c.tyrion_resolved = true; }
            }

            // Check Aeron Damphair (pay 2 power to swap card)
            let combat = state.combat.as_ref().unwrap();
            if !combat.aeron_resolved {
                let atk_card = combat.attacker_card.unwrap();
                let def_card = combat.defender_card.unwrap();

                if atk_card == HouseCardId::AeronDamphair && state.house(attacker).power >= 2 {
                    if let Some(c) = &mut state.combat { c.aeron_resolved = true; }
                    state.pending = Some(PendingDecision::AeronSwap { house: attacker });
                    return;
                }
                if def_card == HouseCardId::AeronDamphair && state.house(defender).power >= 2 {
                    if let Some(c) = &mut state.combat { c.aeron_resolved = true; }
                    state.pending = Some(PendingDecision::AeronSwap { house: defender });
                    return;
                }

                if let Some(c) = &mut state.combat { c.aeron_resolved = true; }
            }

            // Move to Resolution
            if let Some(c) = &mut state.combat {
                c.phase = CombatPhase::Resolution;
            }
            advance_combat(state);
        }

        CombatPhase::Resolution => {
            // Ask blade holder if they want to use it (+1 strength)
            let blade_holder = find_track_holder(state, Track::Fiefdoms);
            if !state.valyrian_steel_blade_used {
                let in_combat = blade_holder == attacker || blade_holder == defender;
                if in_combat {
                    state.pending = Some(PendingDecision::UseValyrianBlade { house: blade_holder });
                    // Move to PostCombat after blade decision
                    if let Some(c) = &mut state.combat {
                        c.phase = CombatPhase::PostCombat;
                    }
                    return;
                }
            }

            if let Some(c) = &mut state.combat {
                c.phase = CombatPhase::PostCombat;
            }
            advance_combat(state);
        }

        CombatPhase::PostCombat => {
            resolve_combat_final(state);
        }
    }
}

fn begin_combat(state: &mut GameState, attacker: HouseName, defender: HouseName,
                area_id: AreaId, attacking_units: Vec<Unit>, march_from: AreaId) {
    let defending_units = state.area(area_id).units.clone();

    // Find adjacent support areas (non-combatant houses with Support orders)
    let mut support_houses: Vec<(AreaId, HouseName)> = Vec::new();
    let area_def = &AREAS[area_id.0 as usize];
    for &adj in area_def.adjacent {
        let adj_area = state.area(adj);
        if let Some(order) = adj_area.order {
            if order.order_type == OrderType::Support {
                if let Some(h) = adj_area.house {
                    if h != attacker && h != defender {
                        support_houses.push((adj, h));
                    }
                }
            }
        }
    }

    state.combat = Some(CombatState {
        attacker,
        defender,
        area_id,
        attacking_units,
        defending_units,
        attacker_card: None,
        defender_card: None,
        attacker_strength: 0,
        defender_strength: 0,
        march_from_area: Some(march_from),
        attacker_used_blade: false,
        defender_used_blade: false,
        support_decisions: HashMap::new(),
        phase: if support_houses.is_empty() { CombatPhase::Cards } else { CombatPhase::Support },
        aeron_resolved: false,
        tyrion_resolved: false,
        pending_support_houses: support_houses,
    });

    // Auto-add combatants' own support (adjacent support orders from attacker/defender)
    for &adj in area_def.adjacent {
        let adj_area = state.area(adj);
        if let Some(order) = adj_area.order {
            if order.order_type == OrderType::Support {
                if adj_area.house == Some(attacker) {
                    state.combat.as_mut().unwrap().support_decisions.insert(adj, SupportChoice::Attacker);
                } else if adj_area.house == Some(defender) {
                    state.combat.as_mut().unwrap().support_decisions.insert(adj, SupportChoice::Defender);
                }
            }
        }
    }

    state.phase = Phase::Combat;
}

fn resolve_combat_final(state: &mut GameState) {
    // Extract all combat data
    let combat = match &state.combat {
        Some(c) => c,
        None => { state.phase = Phase::Action; return; }
    };

    let attacker = combat.attacker;
    let defender = combat.defender;
    let area_id = combat.area_id;
    let atk_card_id = combat.attacker_card;
    let def_card_id = combat.defender_card;
    let attacking_units = combat.attacking_units.clone();
    let defending_units = combat.defending_units.clone();
    let march_from_area = combat.march_from_area;
    let support_decisions = combat.support_decisions.clone();

    let atk_card = atk_card_id.and_then(|id| {
        if state.house(attacker).hand.contains(&id) || state.house(attacker).discards.contains(&id) || true {
            Some(cards::get_house_card(id))
        } else { None }
    });
    let def_card = def_card_id.and_then(|id| Some(cards::get_house_card(id)));

    // ── Compute strengths ──

    // Unit strength
    let area_def = &AREAS[area_id.0 as usize];
    let atk_unit_str: i16 = attacking_units.iter()
        .map(|u| {
            if u.unit_type == UnitType::SiegeEngine && area_def.has_castle_or_stronghold() {
                4i16 // Siege engines get 4 when attacking castle/stronghold
            } else {
                u.unit_type.combat_strength() as i16
            }
        })
        .sum();
    let def_unit_str: i16 = defending_units.iter()
        .map(|u| u.unit_type.combat_strength() as i16)
        .sum();

    // Card strength
    let atk_card_str = atk_card.map_or(0i16, |c| c.strength as i16);
    let def_card_str = def_card.map_or(0i16, |c| c.strength as i16);

    // March order bonus
    let march_bonus = march_from_area
        .and_then(|from| state.area(from).order.map(|o| o.strength as i16))
        .unwrap_or(0);

    // Defense order bonus
    let defense_bonus: i16 = state.area(area_id).order
        .filter(|o| o.order_type == OrderType::Defense)
        .map_or(0, |o| o.strength as i16);

    // Garrison defense
    let garrison_str: i16 = state.garrisons.get(&area_id)
        .filter(|g| g.house == defender)
        .map_or(0, |g| g.strength as i16);

    // Support strength
    let mut atk_support: i16 = 0;
    let mut def_support: i16 = 0;
    for (&sup_area, &choice) in &support_decisions {
        if choice == SupportChoice::None { continue; }
        let sup = state.area(sup_area);
        let sup_order = sup.order.map_or(0i16, |o| o.strength as i16);
        let sup_units: i16 = sup.units.iter()
            .map(|u| u.unit_type.combat_strength() as i16)
            .sum();
        let total = sup_order + sup_units;
        match choice {
            SupportChoice::Attacker => atk_support += total,
            SupportChoice::Defender => def_support += total,
            SupportChoice::None => {}
        }
    }

    // Valyrian Steel Blade
    let atk_blade: i16 = if state.combat.as_ref().unwrap().attacker_used_blade { 1 } else { 0 };
    let def_blade: i16 = if state.combat.as_ref().unwrap().defender_used_blade { 1 } else { 0 };

    // Card abilities: Catelyn Stark bonus
    let mut atk_ability_bonus: i16 = 0;
    let mut def_ability_bonus: i16 = 0;
    if atk_card_id == Some(HouseCardId::CatelynStark) {
        // +1 per Stark card in discard
        atk_ability_bonus += state.house(attacker).discards.len() as i16;
    }
    if def_card_id == Some(HouseCardId::CatelynStark) {
        def_ability_bonus += state.house(defender).discards.len() as i16;
    }

    let atk_total = atk_unit_str + atk_card_str + march_bonus + atk_support + atk_blade + atk_ability_bonus;
    let def_total = def_unit_str + def_card_str + defense_bonus + garrison_str + def_support + def_blade + def_ability_bonus;

    // Write back strengths
    if let Some(c) = &mut state.combat {
        c.attacker_strength = atk_total;
        c.defender_strength = def_total;
    }

    // Tiebreaker: holder of Valyrian Steel Blade (Fiefdoms track position 1)
    let atk_fiefdoms = state.house(attacker).fiefdoms;
    let def_fiefdoms = state.house(defender).fiefdoms;
    let attacker_wins = atk_total > def_total
        || (atk_total == def_total && atk_fiefdoms < def_fiefdoms);

    // ── Casualties ──
    let atk_swords = atk_card.map_or(0u8, |c| c.swords);
    let def_swords = def_card.map_or(0u8, |c| c.swords);
    let atk_forts = atk_card.map_or(0u8, |c| c.fortifications);
    let def_forts = def_card.map_or(0u8, |c| c.fortifications);

    let (winner_swords, loser_forts) = if attacker_wins {
        (atk_swords, def_forts)
    } else {
        (def_swords, atk_forts)
    };
    let casualties = winner_swords.saturating_sub(loser_forts) as usize;

    // ── Apply result ──

    if attacker_wins {
        // Kill defender casualties
        let mut killed = 0;
        let mut remaining_defenders: Vec<Unit> = defending_units.clone();
        while killed < casualties && !remaining_defenders.is_empty() {
            let unit = remaining_defenders.remove(0);
            *state.house_mut(defender).available_units.get_mut(unit.unit_type) += 1;
            killed += 1;
        }

        // Place attacking units in conquered area
        state.area_mut(area_id).units.retain(|u| u.house != defender);
        for unit in &attacking_units {
            state.area_mut(area_id).units.push(*unit);
        }
        state.area_mut(area_id).house = Some(attacker);

        // Roose Bolton: returns to hand instead of discard
        if def_card_id == Some(HouseCardId::RooseBolton) {
            if let Some(pos) = state.house(defender).discards.iter().position(|&c| c == HouseCardId::RooseBolton) {
                state.house_mut(defender).discards.remove(pos);
                state.house_mut(defender).hand.push(HouseCardId::RooseBolton);
            }
        }

        // Post-combat: Tywin Lannister (winner takes 2 power from loser)
        if atk_card_id == Some(HouseCardId::TywinLannister) {
            let steal = state.house(defender).power.min(2);
            state.house_mut(defender).power -= steal;
            state.house_mut(attacker).power += steal;
        }

        // Post-combat: Cersei Lannister (remove one enemy order)
        if atk_card_id == Some(HouseCardId::CerseiLannister) {
            state.pending = Some(PendingDecision::CerseiRemoveOrder { opponent: defender });
            check_victory(state);
            // Don't clear combat yet — will be cleared after Cersei decision
            return;
        }

        // Robb Stark: attacker chooses retreat area for defender
        if atk_card_id == Some(HouseCardId::RobbStark) {
            let retreat_options = find_retreat_areas(state, area_id, defender);
            if !retreat_options.is_empty() && !remaining_defenders.is_empty() {
                state.pending = Some(PendingDecision::RobbRetreat {
                    house: attacker,
                    possible_areas: retreat_options,
                });
                check_victory(state);
                return;
            }
        }

        // Defender must retreat surviving units
        if !remaining_defenders.is_empty() {
            let retreat_options = find_retreat_areas(state, area_id, defender);
            if retreat_options.is_empty() {
                // Units destroyed
                for unit in remaining_defenders {
                    *state.house_mut(defender).available_units.get_mut(unit.unit_type) += 1;
                }
            } else {
                state.pending = Some(PendingDecision::Retreat {
                    house: defender,
                    units: remaining_defenders,
                    from_area: area_id,
                    possible_areas: retreat_options,
                });
                check_victory(state);
                return;
            }
        }

        // Patchface: look at opponent's hand, discard one
        if atk_card_id == Some(HouseCardId::Patchface) && !state.house(defender).hand.is_empty() {
            let visible = state.house(defender).hand.clone();
            state.pending = Some(PendingDecision::PatchfaceDiscard {
                opponent: defender,
                visible_cards: visible,
            });
            check_victory(state);
            return;
        }

        // Doran Martell: if defender played Doran and lost, move attacker down one track
        if def_card_id == Some(HouseCardId::DoranMartell) {
            state.pending = Some(PendingDecision::DoranChooseTrack { opponent: attacker });
            check_victory(state);
            return;
        }
    } else {
        // Defender wins — attacker retreats

        // Kill attacker casualties
        let mut killed = 0;
        let mut remaining_attackers: Vec<Unit> = attacking_units.clone();
        while killed < casualties && !remaining_attackers.is_empty() {
            let unit = remaining_attackers.remove(0);
            *state.house_mut(attacker).available_units.get_mut(unit.unit_type) += 1;
            killed += 1;
        }

        // Roose Bolton for attacker
        if atk_card_id == Some(HouseCardId::RooseBolton) {
            if let Some(pos) = state.house(attacker).discards.iter().position(|&c| c == HouseCardId::RooseBolton) {
                state.house_mut(attacker).discards.remove(pos);
                state.house_mut(attacker).hand.push(HouseCardId::RooseBolton);
            }
        }

        // Tywin for defender
        if def_card_id == Some(HouseCardId::TywinLannister) {
            let steal = state.house(attacker).power.min(2);
            state.house_mut(attacker).power -= steal;
            state.house_mut(defender).power += steal;
        }

        // Cersei for defender
        if def_card_id == Some(HouseCardId::CerseiLannister) {
            state.pending = Some(PendingDecision::CerseiRemoveOrder { opponent: attacker });
            check_victory(state);
            return;
        }

        // Return surviving attackers to origin (routed)
        let from = march_from_area.unwrap_or(area_id);
        for mut unit in remaining_attackers {
            unit.routed = true;
            state.area_mut(from).units.push(unit);
        }

        // Patchface for defender
        if def_card_id == Some(HouseCardId::Patchface) && !state.house(attacker).hand.is_empty() {
            let visible = state.house(attacker).hand.clone();
            state.pending = Some(PendingDecision::PatchfaceDiscard {
                opponent: attacker,
                visible_cards: visible,
            });
            check_victory(state);
            return;
        }

        // Doran for attacker losing
        if atk_card_id == Some(HouseCardId::DoranMartell) {
            state.pending = Some(PendingDecision::DoranChooseTrack { opponent: defender });
            check_victory(state);
            return;
        }
    }

    // Combat done
    finalize_combat(state);
}

fn finalize_combat(state: &mut GameState) {
    // Remove march order from origin
    if let Some(combat) = &state.combat {
        if let Some(from) = combat.march_from_area {
            state.area_mut(from).order = None;
        }
    }

    state.combat = None;
    state.phase = Phase::Action;
    // Advance to next player in action cycle
    let pc = state.playing_houses.len() as u8;
    state.action_player_index = (state.action_player_index + 1) % pc;
    check_victory(state);
}

// ═══════════════════════════════════════════════════════════════════════
// APPLY ACTION — resolve player decisions
// ═══════════════════════════════════════════════════════════════════════

pub fn apply_action(state: &mut GameState, action: Action) {
    let pending = state.pending.take();
    if pending.is_none() { return; }

    match (pending.unwrap(), action) {
        // ── Planning ──
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
        }

        // ── Westeros choices ──
        (PendingDecision::WesterosChoice { card_name, options, .. }, Action::WesterosChoice(idx)) => {
            let choice = options.get(idx).cloned().unwrap_or_default();
            match card_name.as_str() {
                "A Throne of Blades" => {
                    if choice == "Mustering" {
                        state.muster_house_idx = 1;
                    } else {
                        resolve_supply_update(state);
                    }
                }
                "Dark Wings, Dark Words" => {
                    if choice == "Clash of Kings" {
                        begin_clash_of_kings(state);
                    } else {
                        resolve_game_of_thrones(state);
                    }
                }
                "Put to the Sword" => {
                    let order_type = match idx {
                        0 => OrderType::March,
                        1 => OrderType::Defense,
                        2 => OrderType::Support,
                        3 => OrderType::Raid,
                        4 => OrderType::ConsolidatePower,
                        _ => OrderType::March,
                    };
                    state.star_order_restrictions.push(order_type);
                }
                _ => {}
            }
        }

        // ── Bidding ──
        (PendingDecision::Bidding { house, .. }, Action::Bid(amount)) => {
            let clamped = amount.min(state.house(house).power);
            if let Some(bidding) = &mut state.bidding {
                bidding.bids.insert(house, clamped);
                bidding.next_bidder_idx += 1;
            }
        }

        // ── Mustering ──
        (PendingDecision::Muster { house, areas: _ }, Action::Muster(actions)) => {
            for (area_id, muster_action) in actions {
                match muster_action {
                    MusterAction2::Build(unit_type) => {
                        let pool = state.house(house).available_units.get(unit_type);
                        if pool > 0 {
                            *state.house_mut(house).available_units.get_mut(unit_type) -= 1;
                            state.area_mut(area_id).units.push(Unit {
                                unit_type,
                                house,
                                routed: false,
                            });
                        }
                    }
                    MusterAction2::Upgrade => {
                        // Find a footman and upgrade to knight
                        let has_knight = state.house(house).available_units.knights > 0;
                        let area = &mut state.areas[area_id.0 as usize];
                        if let Some(pos) = area.units.iter().position(|u| {
                            u.house == house && u.unit_type == UnitType::Footman
                        }) {
                            if has_knight {
                                area.units[pos].unit_type = UnitType::Knight;
                                state.house_mut(house).available_units.knights -= 1;
                                state.house_mut(house).available_units.footmen += 1;
                            }
                        }
                    }
                }
            }
            // Advance muster_house_idx (for Westeros mustering)
            if state.muster_house_idx > 0 {
                state.muster_house_idx += 1;
            }
        }

        // ── Raids ──
        (PendingDecision::ChooseRaid { house: _, from_area, .. }, Action::Raid(target)) => {
            if let Some(target_id) = target {
                // If raiding a CP order, steal a power token
                let target_order = state.area(target_id).order;
                if let Some(order) = target_order {
                    if order.order_type == OrderType::ConsolidatePower {
                        if let Some(target_house) = state.area(target_id).house {
                            if state.house(target_house).power > 0 {
                                let raider = state.area(from_area).house.unwrap_or(HouseName::Stark);
                                state.house_mut(target_house).power -= 1;
                                state.house_mut(raider).power += 1;
                            }
                        }
                    }
                }
                // Remove target's order
                state.area_mut(target_id).order = None;
            }
            // Remove own raid order
            state.area_mut(from_area).order = None;
            let pc = state.playing_houses.len() as u8;
            state.action_player_index = (state.action_player_index + 1) % pc;
        }

        // ── Marches ──
        (PendingDecision::ChooseMarch { house, from_area, .. }, Action::March { to, unit_indices }) => {
            // Collect units to move
            let moving_units: Vec<Unit> = unit_indices.iter()
                .filter_map(|&i| state.area(from_area).units.get(i).copied())
                .collect();

            // Remove from source (reverse order)
            let mut sorted_indices = unit_indices.clone();
            sorted_indices.sort_unstable_by(|a, b| b.cmp(a));
            for &i in &sorted_indices {
                if i < state.area(from_area).units.len() {
                    state.area_mut(from_area).units.remove(i);
                }
            }

            // Check for combat
            let target_house = state.area(to).house;
            let has_enemy_units = !state.area(to).units.is_empty()
                && target_house.is_some()
                && target_house != Some(house);

            // Also check garrison
            let has_garrison = state.garrisons.get(&to)
                .map_or(false, |g| g.house != house);

            if has_enemy_units || (target_house.is_some() && target_house != Some(house) && has_garrison) {
                // Combat!
                begin_combat(state, house, target_house.unwrap(), to, moving_units, from_area);
            } else {
                // No combat — place units
                for unit in moving_units {
                    state.area_mut(to).units.push(unit);
                }
                if state.area(to).house.is_none() || state.area(to).house == Some(house) {
                    state.area_mut(to).house = Some(house);
                }

                // Update source area
                if state.area(from_area).units.is_empty() {
                    if AREAS[from_area.0 as usize].is_land() {
                        state.pending = Some(PendingDecision::LeavePowerToken {
                            house,
                            area_id: from_area,
                        });
                        // Don't advance yet — wait for leave token decision
                        return;
                    }
                    state.area_mut(from_area).house = None;
                }

                state.area_mut(from_area).order = None;
                let pc = state.playing_houses.len() as u8;
                state.action_player_index = (state.action_player_index + 1) % pc;
                check_victory(state);
            }
        }

        (PendingDecision::ChooseMarch { from_area, .. }, Action::MarchSkip) => {
            state.area_mut(from_area).order = None;
            let pc = state.playing_houses.len() as u8;
            state.action_player_index = (state.action_player_index + 1) % pc;
        }

        // ── Leave Power Token ──
        (PendingDecision::LeavePowerToken { house, area_id }, Action::LeavePowerToken(leave)) => {
            if leave && state.house(house).power > 0 {
                state.house_mut(house).power -= 1;
                // Keep control via power token
            } else {
                state.area_mut(area_id).house = None;
            }
            // Remove march order and advance
            state.area_mut(area_id).order = None;
            let pc = state.playing_houses.len() as u8;
            state.action_player_index = (state.action_player_index + 1) % pc;
        }

        // ── Combat: Card Selection ──
        (PendingDecision::SelectHouseCard { house, .. }, Action::SelectCard(card_id)) => {
            if let Some(combat) = &mut state.combat {
                if house == combat.attacker {
                    combat.attacker_card = Some(card_id);
                } else {
                    combat.defender_card = Some(card_id);
                }
            }
            // Remove card from hand, add to discards
            let hand = &mut state.house_mut(house).hand;
            if let Some(pos) = hand.iter().position(|&c| c == card_id) {
                hand.remove(pos);
            }
            state.house_mut(house).discards.push(card_id);
        }

        // ── Combat: Support ──
        (PendingDecision::SupportDeclaration { area_id, .. }, Action::DeclareSupport(choice)) => {
            if let Some(combat) = &mut state.combat {
                combat.support_decisions.insert(area_id, choice);
                combat.pending_support_houses.retain(|&(a, _)| a != area_id);
            }
        }

        // ── Combat: Valyrian Steel Blade ──
        (PendingDecision::UseValyrianBlade { house }, Action::UseValyrianBlade(use_it)) => {
            if use_it {
                state.valyrian_steel_blade_used = true;
                if let Some(combat) = &mut state.combat {
                    if house == combat.attacker {
                        combat.attacker_used_blade = true;
                    } else {
                        combat.defender_used_blade = true;
                    }
                }
            }
        }

        // ── Combat: Retreat ──
        (PendingDecision::Retreat { house, from_area, .. }, Action::Retreat(to)) => {
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
            finalize_combat(state);
        }

        // ── Combat: Tyrion replacement ──
        (PendingDecision::TyrionReplace { opponent }, Action::TyrionReplace(new_card)) => {
            // Opponent picks a new card
            if let Some(combat) = &mut state.combat {
                if opponent == combat.attacker {
                    combat.attacker_card = Some(new_card);
                } else {
                    combat.defender_card = Some(new_card);
                }
            }
            let hand = &mut state.house_mut(opponent).hand;
            if let Some(pos) = hand.iter().position(|&c| c == new_card) {
                hand.remove(pos);
            }
            state.house_mut(opponent).discards.push(new_card);
        }

        // ── Combat: Aeron swap ──
        (PendingDecision::AeronSwap { house }, Action::AeronSwap(new_card)) => {
            if let Some(new_id) = new_card {
                // Pay 2 power
                state.house_mut(house).power = state.house(house).power.saturating_sub(2);
                // Return old card to hand, play new one
                let old_card = if let Some(combat) = &state.combat {
                    if house == combat.attacker { combat.attacker_card } else { combat.defender_card }
                } else { None };

                if let Some(old) = old_card {
                    // Return old to hand
                    if let Some(pos) = state.house(house).discards.iter().position(|&c| c == old) {
                        state.house_mut(house).discards.remove(pos);
                    }
                    state.house_mut(house).hand.push(old);
                }

                // Play new card
                if let Some(combat) = &mut state.combat {
                    if house == combat.attacker {
                        combat.attacker_card = Some(new_id);
                    } else {
                        combat.defender_card = Some(new_id);
                    }
                }
                let hand = &mut state.house_mut(house).hand;
                if let Some(pos) = hand.iter().position(|&c| c == new_id) {
                    hand.remove(pos);
                }
                state.house_mut(house).discards.push(new_id);
            }
        }

        // ── Combat: Robb retreat choice ──
        (PendingDecision::RobbRetreat { .. }, Action::RobbRetreat(to)) => {
            // Move defender's units to chosen area
            if let Some(combat) = &state.combat {
                let defender = combat.defender;
                let area_id = combat.area_id;
                let units: Vec<Unit> = state.area(area_id).units.iter()
                    .filter(|u| u.house == defender)
                    .cloned()
                    .collect();
                state.area_mut(area_id).units.retain(|u| u.house != defender);
                for mut unit in units {
                    unit.routed = true;
                    state.area_mut(to).units.push(unit);
                }
                if state.area(to).house.is_none() {
                    state.area_mut(to).house = Some(defender);
                }
            }
            finalize_combat(state);
        }

        // ── Post-combat: Cersei ──
        (PendingDecision::CerseiRemoveOrder { .. }, Action::CerseiRemoveOrder(area_id)) => {
            state.area_mut(area_id).order = None;
            finalize_combat(state);
        }

        // ── Post-combat: Patchface ──
        (PendingDecision::PatchfaceDiscard { opponent, .. }, Action::PatchfaceDiscard(card_id)) => {
            let hand = &mut state.house_mut(opponent).hand;
            if let Some(pos) = hand.iter().position(|&c| c == card_id) {
                hand.remove(pos);
            }
            state.house_mut(opponent).discards.push(card_id);
            finalize_combat(state);
        }

        // ── Post-combat: Doran ──
        (PendingDecision::DoranChooseTrack { opponent }, Action::DoranChooseTrack(track)) => {
            // Move opponent to last position on chosen track
            let pc = state.playing_houses.len() as u8;
            match track {
                Track::IronThrone => {
                    let old_pos = state.house(opponent).iron_throne;
                    // Everyone below moves up
                    let houses = state.playing_houses.clone();
                    for &h in &houses {
                        if state.house(h).iron_throne > old_pos {
                            state.house_mut(h).iron_throne -= 1;
                        }
                    }
                    state.house_mut(opponent).iron_throne = pc;
                    // Update turn order
                    let mut new_order = state.playing_houses.clone();
                    new_order.sort_by_key(|&h| state.house(h).iron_throne);
                    state.turn_order = new_order;
                }
                Track::Fiefdoms => {
                    let old_pos = state.house(opponent).fiefdoms;
                    let houses = state.playing_houses.clone();
                    for &h in &houses {
                        if state.house(h).fiefdoms > old_pos {
                            state.house_mut(h).fiefdoms -= 1;
                        }
                    }
                    state.house_mut(opponent).fiefdoms = pc;
                }
                Track::KingsCourt => {
                    let old_pos = state.house(opponent).kings_court;
                    let houses = state.playing_houses.clone();
                    for &h in &houses {
                        if state.house(h).kings_court > old_pos {
                            state.house_mut(h).kings_court -= 1;
                        }
                    }
                    state.house_mut(opponent).kings_court = pc;
                }
            }
            finalize_combat(state);
        }

        // ── Queen of Thorns ──
        (PendingDecision::QueenOfThornsRemoveOrder { .. }, Action::QueenOfThorns(area_id)) => {
            state.area_mut(area_id).order = None;
        }

        // ── Reconcile ──
        (PendingDecision::Reconcile { house, .. }, Action::Reconcile(aid, unit_idx)) => {
            if unit_idx < state.area(aid).units.len() {
                let unit = state.area_mut(aid).units.remove(unit_idx);
                *state.house_mut(house).available_units.get_mut(unit.unit_type) += 1;
            }
            // Check ALL houses for remaining violations (current first, then others)
            for &h in &state.playing_houses.clone() {
                if supply::check_supply_violation(state, h) {
                    let violations = supply::find_violations(state, h);
                    if let Some(&(vid, curr, max)) = violations.first() {
                        state.pending = Some(PendingDecision::Reconcile {
                            house: h, area_id: vid, current_size: curr, max_allowed: max,
                        });
                        break;
                    }
                }
            }
        }

        // ── Wildling penalty ──
        (PendingDecision::WildlingPenaltyChoice { .. }, Action::WildlingPenalty(_idx)) => {
            // Simplified: penalty already applied in resolve_wildling_bidding
        }

        _ => {
            // Unhandled — clear pending and continue
        }
    }

    // After applying action, try to advance
    if state.pending.is_none() {
        advance(state);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════

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
                    OrderType::Defense if is_star => true,
                    _ => false,
                }
            } else {
                false
            }
        })
        .copied()
        .collect()
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

    // Reset per-round tracking
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
        resolve_tiebreaker(state);
        return;
    }

    // Rounds 2+: Westeros phase
    state.phase = Phase::Westeros;
    state.westeros_step = 0;
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
    // Strongholds (2 pts) > Castles (1 pt), then supply, then power, then Iron Throne
    let mut rankings: Vec<(HouseName, usize, u8, u8, u8)> = state.playing_houses.iter()
        .map(|&h| {
            let score: usize = state.areas.iter().enumerate()
                .filter(|(_, a)| a.house == Some(h))
                .map(|(i, _)| {
                    if AREAS[i].stronghold { 2 }
                    else if AREAS[i].castle { 1 }
                    else { 0 }
                })
                .sum();
            let profile = state.house(h);
            (h, score, profile.supply, profile.power, profile.iron_throne)
        })
        .collect();

    rankings.sort_by(|a, b| {
        b.1.cmp(&a.1)                       // Most castle/stronghold points
            .then(b.2.cmp(&a.2))             // Highest supply
            .then(b.3.cmp(&a.3))             // Most power
            .then(a.4.cmp(&b.4))             // Best Iron Throne (lower = better)
    });

    state.winner = Some(rankings[0].0);
}

// ═══════════════════════════════════════════════════════════════════════
// Comprehensive test suite for the Game of Thrones engine
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use crate::types::*;
    use crate::map::*;
    use crate::cards;
    use crate::supply;
    use crate::engine::{advance, apply_action, Action, MusterAction2};
    use crate::navigation;
    use crate::setup::create_initial_state;

    // ── Helper: create a minimal state for unit testing ──────────────────

    fn make_6p_state(seed: u64) -> GameState {
        create_initial_state(6, seed)
    }

    /// Run a full game with a simple random agent (seed-deterministic).
    fn play_full_game_random(seed: u64, player_count: u8) -> GameState {
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let mut state = create_initial_state(player_count, seed);
        advance(&mut state);

        let mut step = 0u64;
        while state.winner.is_none() && step < 100_000 {
            step += 1;
            let pending = match &state.pending {
                Some(p) => p.clone(),
                None => break,
            };
            let mut rng = ChaCha8Rng::seed_from_u64(seed.wrapping_add(step.wrapping_mul(999961)));
            let action = random_action(&pending, &state, &mut rng);
            apply_action(&mut state, action);
            advance(&mut state);
        }
        state
    }

    /// Produce a random action for a given pending decision.
    fn random_action(
        pending: &PendingDecision,
        state: &GameState,
        rng: &mut impl rand::Rng,
    ) -> Action {
        use rand::seq::SliceRandom;
        match pending {
            PendingDecision::PlaceOrders { house } => {
                let my_areas: Vec<AreaId> = state.areas.iter().enumerate()
                    .filter(|(_, a)| a.house == Some(*house) && !a.units.is_empty())
                    .map(|(i, _)| AreaId(i as u8))
                    .collect();
                let mut orders = Vec::new();
                let mut used: Vec<u8> = Vec::new();
                for area in &my_areas {
                    let available: Vec<u8> = (0..15u8)
                        .filter(|t| !used.contains(t))
                        .collect();
                    if let Some(&t) = available.choose(rng) {
                        orders.push((*area, t));
                        used.push(t);
                    }
                }
                Action::PlaceOrders(orders)
            }
            PendingDecision::ChooseRaid { valid_targets, .. } => {
                if valid_targets.is_empty() || rng.gen_bool(0.3) {
                    Action::Raid(None)
                } else {
                    Action::Raid(Some(*valid_targets.choose(rng).unwrap()))
                }
            }
            PendingDecision::ChooseMarch { from_area, valid_destinations, .. } => {
                if valid_destinations.is_empty() {
                    Action::MarchSkip
                } else {
                    let to = *valid_destinations.choose(rng).unwrap();
                    let unit_count = state.area(*from_area).units.len();
                    let indices: Vec<usize> = (0..unit_count).collect();
                    Action::March { to, unit_indices: indices }
                }
            }
            PendingDecision::LeavePowerToken { .. } => {
                Action::LeavePowerToken(rng.gen_bool(0.5))
            }
            PendingDecision::SupportDeclaration { .. } => {
                let choices = [SupportChoice::Attacker, SupportChoice::Defender, SupportChoice::None];
                Action::DeclareSupport(*choices.choose(rng).unwrap())
            }
            PendingDecision::SelectHouseCard { available_cards, .. } => {
                Action::SelectCard(*available_cards.choose(rng).unwrap())
            }
            PendingDecision::UseValyrianBlade { .. } => {
                Action::UseValyrianBlade(rng.gen_bool(0.5))
            }
            PendingDecision::Bidding { house, .. } => {
                let max = state.house(*house).power;
                Action::Bid(rng.gen_range(0..=max))
            }
            PendingDecision::WesterosChoice { options, .. } => {
                Action::WesterosChoice(rng.gen_range(0..options.len()))
            }
            PendingDecision::Muster { areas, .. } => {
                // Simple: build one footman per area if possible
                let mut actions = Vec::new();
                for area in areas {
                    if area.points >= 1 {
                        actions.push((area.area_id, MusterAction2::Build(UnitType::Footman)));
                    }
                }
                Action::Muster(actions)
            }
            PendingDecision::Retreat { possible_areas, .. } => {
                if possible_areas.is_empty() {
                    // Units are destroyed
                    Action::Retreat(AreaId(255))
                } else {
                    Action::Retreat(*possible_areas.choose(rng).unwrap())
                }
            }
            PendingDecision::Reconcile { area_id, .. } => {
                Action::Reconcile(*area_id, 0)
            }
            PendingDecision::MessengerRaven { .. } => {
                Action::MessengerRaven(None)
            }
            PendingDecision::AeronSwap { .. } => {
                Action::AeronSwap(None)
            }
            PendingDecision::TyrionReplace { opponent } => {
                let hand = &state.house(*opponent).hand;
                Action::TyrionReplace(*hand.first().unwrap_or(&HouseCardId::EddardStark))
            }
            PendingDecision::PatchfaceDiscard { visible_cards, .. } => {
                Action::PatchfaceDiscard(*visible_cards.first().unwrap())
            }
            PendingDecision::RobbRetreat { possible_areas, .. } => {
                Action::RobbRetreat(*possible_areas.first().unwrap())
            }
            PendingDecision::WildlingPenaltyChoice { options, .. } => {
                Action::WildlingPenalty(rng.gen_range(0..options.len()))
            }
            PendingDecision::CerseiRemoveOrder { .. } => {
                // find any area with an order
                let area = state.areas.iter().enumerate()
                    .find(|(_, a)| a.order.is_some())
                    .map(|(i, _)| AreaId(i as u8))
                    .unwrap_or(AreaId(0));
                Action::CerseiRemoveOrder(area)
            }
            PendingDecision::DoranChooseTrack { .. } => {
                Action::DoranChooseTrack(Track::IronThrone)
            }
            PendingDecision::QueenOfThornsRemoveOrder { .. } => {
                let area = state.areas.iter().enumerate()
                    .find(|(_, a)| a.order.is_some())
                    .map(|(i, _)| AreaId(i as u8))
                    .unwrap_or(AreaId(0));
                Action::QueenOfThorns(area)
            }
        }
    }

    // ═════════════════════════════════════════════════════════════════════
    // SETUP TESTS
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_initial_state_6_players() {
        let state = make_6p_state(42);
        assert_eq!(state.playing_houses.len(), 6);
        assert_eq!(state.round, 1);
        assert_eq!(state.phase, Phase::Planning); // Round 1 skips Westeros
        assert_eq!(state.wildling_threat, 2);
        assert!(state.winner.is_none());
        assert!(state.combat.is_none());
    }

    #[test]
    fn test_initial_state_3_players() {
        let state = create_initial_state(3, 42);
        assert_eq!(state.playing_houses.len(), 3);
        assert!(state.playing_houses.contains(&HouseName::Stark));
        assert!(state.playing_houses.contains(&HouseName::Lannister));
        assert!(state.playing_houses.contains(&HouseName::Baratheon));
        // Southern areas should be blocked
        assert!(state.areas[SUNSPEAR.0 as usize].blocked);
        assert!(state.areas[HIGHGARDEN.0 as usize].blocked);
        // Pyke gets neutral garrison in 4p but is not blocked in 3p
        assert!(!state.areas[PYKE.0 as usize].blocked);
    }

    #[test]
    fn test_initial_tracks() {
        let state = make_6p_state(42);
        // Baratheon starts at Iron Throne 1
        assert_eq!(state.house(HouseName::Baratheon).iron_throne, 1);
        // Lannister at Iron Throne 2
        assert_eq!(state.house(HouseName::Lannister).iron_throne, 2);
        // Greyjoy at Fiefdoms 1
        assert_eq!(state.house(HouseName::Greyjoy).fiefdoms, 1);
        // Lannister at King's Court 1
        assert_eq!(state.house(HouseName::Lannister).kings_court, 1);
    }

    #[test]
    fn test_initial_units() {
        let state = make_6p_state(42);
        // Stark: Winterfell (K+F), White Harbor (F), Shivering Sea (S)
        let wf = &state.areas[WINTERFELL.0 as usize];
        assert_eq!(wf.units.len(), 2);
        assert_eq!(wf.house, Some(HouseName::Stark));
        assert!(wf.units.iter().any(|u| u.unit_type == UnitType::Knight));
        assert!(wf.units.iter().any(|u| u.unit_type == UnitType::Footman));

        // Baratheon: Dragonstone (K+F), Kingswood (F), Shipbreaker Bay (S)
        let ds = &state.areas[DRAGONSTONE.0 as usize];
        assert_eq!(ds.units.len(), 2);
        assert_eq!(ds.house, Some(HouseName::Baratheon));
    }

    #[test]
    fn test_initial_cards() {
        let state = make_6p_state(42);
        for h in &state.playing_houses {
            assert_eq!(state.house(*h).hand.len(), 7, "{:?} should have 7 cards", h);
            assert!(state.house(*h).discards.is_empty());
        }
    }

    #[test]
    fn test_initial_power() {
        let state = make_6p_state(42);
        for h in &state.playing_houses {
            assert_eq!(state.house(*h).power, 5);
        }
    }

    #[test]
    fn test_neutral_garrisons_6p() {
        let state = make_6p_state(42);
        // King's Landing and The Eyrie should have neutral garrisons
        let kl = state.garrisons.get(&KINGS_LANDING);
        assert!(kl.is_some());
        assert_eq!(kl.unwrap().house, None);
        assert_eq!(kl.unwrap().strength, 5);

        let eyrie = state.garrisons.get(&THE_EYRIE);
        assert!(eyrie.is_some());
        assert_eq!(eyrie.unwrap().house, None);
        assert_eq!(eyrie.unwrap().strength, 6);
    }

    #[test]
    fn test_neutral_garrisons_5p() {
        let state = create_initial_state(5, 42);
        // 5-player: Martell excluded, Dornish areas get neutral garrisons
        assert!(state.garrisons.get(&SUNSPEAR).is_some());
        assert_eq!(state.garrisons.get(&SUNSPEAR).unwrap().strength, 5);
        assert_eq!(state.garrisons.get(&SUNSPEAR).unwrap().house, None);
    }

    // ═════════════════════════════════════════════════════════════════════
    // SUPPLY TESTS
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_supply_limits() {
        assert_eq!(supply_limits(0), &[2, 2]);
        assert_eq!(supply_limits(1), &[3, 2]);
        assert_eq!(supply_limits(2), &[3, 2, 2]);
        assert_eq!(supply_limits(3), &[3, 2, 2, 2]);
        assert_eq!(supply_limits(4), &[3, 3, 2, 2]);
        assert_eq!(supply_limits(5), &[4, 3, 2, 2]);
        assert_eq!(supply_limits(6), &[4, 3, 2, 2, 2]);
    }

    #[test]
    fn test_supply_violation_empty() {
        let state = make_6p_state(42);
        // At start, no house should violate supply
        for h in &state.playing_houses {
            assert!(!supply::check_supply_violation(&state, *h),
                "{:?} should not violate supply at start", h);
        }
    }

    #[test]
    fn test_supply_violation_detected() {
        let mut state = make_6p_state(42);
        // Give Stark supply 0 (max armies: [2, 2]) but put 3 units in Winterfell
        state.house_mut(HouseName::Stark).supply = 0;
        state.areas[WINTERFELL.0 as usize].units.push(Unit {
            unit_type: UnitType::Footman,
            house: HouseName::Stark,
            routed: false,
        });
        // Winterfell now has 3 units, supply 0 max is 2 — violation
        assert!(supply::check_supply_violation(&state, HouseName::Stark));
    }

    #[test]
    fn test_supply_calculation() {
        let state = make_6p_state(42);
        // Stark controls Winterfell, White Harbor — check supply icons
        let calc = supply::calculate_supply(&state, HouseName::Stark);
        assert!(calc >= 1, "Stark should have at least 1 supply icon");
    }

    // ═════════════════════════════════════════════════════════════════════
    // UNIT TYPE TESTS
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_combat_strength() {
        assert_eq!(UnitType::Footman.combat_strength(), 1);
        assert_eq!(UnitType::Knight.combat_strength(), 2);
        assert_eq!(UnitType::Ship.combat_strength(), 1);
        assert_eq!(UnitType::SiegeEngine.combat_strength(), 0);
    }

    #[test]
    fn test_muster_cost() {
        assert_eq!(UnitType::Footman.muster_cost(), 1);
        assert_eq!(UnitType::Knight.muster_cost(), 2);
        assert_eq!(UnitType::Ship.muster_cost(), 1);
        assert_eq!(UnitType::SiegeEngine.muster_cost(), 2);
    }

    // ═════════════════════════════════════════════════════════════════════
    // CARD TESTS
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_house_cards_count() {
        for h in HouseName::ALL {
            let hand = cards::house_cards(h);
            assert_eq!(hand.len(), 7, "{:?} should have 7 house cards", h);
        }
    }

    #[test]
    fn test_house_card_lookup() {
        let card = cards::get_house_card(HouseCardId::TywinLannister);
        assert_eq!(card.house, HouseName::Lannister);
        assert_eq!(card.strength, 4);

        let card = cards::get_house_card(HouseCardId::EddardStark);
        assert_eq!(card.house, HouseName::Stark);
        assert_eq!(card.strength, 4);
    }

    #[test]
    fn test_house_card_strength_range() {
        // All cards should have strength 0-4
        for h in HouseName::ALL {
            for card in cards::house_cards(h) {
                assert!(card.strength <= 4, "{:?} has strength > 4", card.id);
            }
        }
    }

    #[test]
    fn test_westeros_deck_sizes() {
        let d1 = cards::westeros_deck_1();
        let d2 = cards::westeros_deck_2();
        let d3 = cards::westeros_deck_3();
        assert!(!d1.is_empty());
        assert!(!d2.is_empty());
        assert!(!d3.is_empty());
    }

    #[test]
    fn test_wildling_deck_size() {
        let wl = cards::wildling_deck();
        assert_eq!(wl.len(), 9, "Should have 9 wildling cards");
    }

    // ═════════════════════════════════════════════════════════════════════
    // ORDER TOKEN TESTS
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_order_tokens_count() {
        assert_eq!(ORDER_TOKENS.len(), 15);
    }

    #[test]
    fn test_order_tokens_distribution() {
        let march_count = ORDER_TOKENS.iter().filter(|t| t.order_type == OrderType::March).count();
        let defense_count = ORDER_TOKENS.iter().filter(|t| t.order_type == OrderType::Defense).count();
        let support_count = ORDER_TOKENS.iter().filter(|t| t.order_type == OrderType::Support).count();
        let raid_count = ORDER_TOKENS.iter().filter(|t| t.order_type == OrderType::Raid).count();
        let cp_count = ORDER_TOKENS.iter().filter(|t| t.order_type == OrderType::ConsolidatePower).count();
        assert_eq!(march_count, 3);
        assert_eq!(defense_count, 3);
        assert_eq!(support_count, 3);
        assert_eq!(raid_count, 3);
        assert_eq!(cp_count, 3);
    }

    #[test]
    fn test_star_orders() {
        // Each order type has exactly 1 star token
        for ot in [OrderType::March, OrderType::Defense, OrderType::Support, OrderType::Raid, OrderType::ConsolidatePower] {
            let stars = ORDER_TOKENS.iter()
                .filter(|t| t.order_type == ot && t.star)
                .count();
            assert_eq!(stars, 1, "{:?} should have exactly 1 star token", ot);
        }
    }

    #[test]
    fn test_star_order_limits() {
        assert_eq!(star_order_limit(6, 1), 3);
        assert_eq!(star_order_limit(6, 4), 1);
        assert_eq!(star_order_limit(6, 5), 0);
        assert_eq!(star_order_limit(3, 1), 3);
        assert_eq!(star_order_limit(3, 3), 1);
    }

    // ═════════════════════════════════════════════════════════════════════
    // MAP TESTS
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_area_count() {
        assert!(AREAS.len() >= 50, "Should have at least 50 areas");
    }

    #[test]
    fn test_winterfell_is_stronghold() {
        let winterfell = &AREAS[WINTERFELL.0 as usize];
        assert!(winterfell.stronghold);
        assert_eq!(winterfell.area_type, AreaType::Land);
    }

    #[test]
    fn test_sea_areas() {
        let bay_of_ice = &AREAS[BAY_OF_ICE.0 as usize];
        assert_eq!(bay_of_ice.area_type, AreaType::Sea);
    }

    #[test]
    fn test_area_neighbors() {
        let winterfell = &AREAS[WINTERFELL.0 as usize];
        assert!(!winterfell.adjacent.is_empty(), "Winterfell should have neighbors");
    }

    // ═════════════════════════════════════════════════════════════════════
    // NAVIGATION TESTS
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_valid_destinations_from_winterfell() {
        let state = make_6p_state(42);
        let dests = navigation::valid_destinations(&state, WINTERFELL, HouseName::Stark);
        assert!(!dests.is_empty(), "Winterfell should have valid march destinations");
    }

    #[test]
    fn test_sea_movement() {
        let state = make_6p_state(42);
        let dests = navigation::valid_destinations(&state, THE_SHIVERING_SEA, HouseName::Stark);
        assert!(!dests.is_empty(), "Ships should be able to move from Shivering Sea");
    }

    // ═════════════════════════════════════════════════════════════════════
    // GAME STATE METHODS TESTS
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_player_count() {
        let state = make_6p_state(42);
        assert_eq!(state.player_count(), 6);

        let state3 = create_initial_state(3, 42);
        assert_eq!(state3.player_count(), 3);
    }

    #[test]
    fn test_current_action_player() {
        let state = make_6p_state(42);
        let player = state.current_action_player();
        // First action player should be the first in turn_order
        assert_eq!(player, state.turn_order[0]);
    }

    #[test]
    fn test_house_mut() {
        let mut state = make_6p_state(42);
        state.house_mut(HouseName::Stark).power = 10;
        assert_eq!(state.house(HouseName::Stark).power, 10);
    }

    #[test]
    fn test_area_mut() {
        let mut state = make_6p_state(42);
        state.area_mut(WINTERFELL).house = Some(HouseName::Lannister);
        assert_eq!(state.area(WINTERFELL).house, Some(HouseName::Lannister));
    }

    // ═════════════════════════════════════════════════════════════════════
    // ENGINE ADVANCE TESTS
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_advance_creates_pending() {
        let mut state = make_6p_state(42);
        advance(&mut state);
        assert!(state.pending.is_some(), "Should have a pending decision after advance");
    }

    #[test]
    fn test_first_pending_is_place_orders() {
        let mut state = make_6p_state(42);
        advance(&mut state);
        match &state.pending {
            Some(PendingDecision::PlaceOrders { house }) => {
                // First player to place orders should be the first in turn_order
                assert_eq!(*house, state.turn_order[0]);
            }
            other => panic!("Expected PlaceOrders, got {:?}", other),
        }
    }

    #[test]
    fn test_apply_place_orders() {
        let mut state = make_6p_state(42);
        advance(&mut state);

        let house = match &state.pending {
            Some(PendingDecision::PlaceOrders { house }) => *house,
            _ => panic!("Expected PlaceOrders"),
        };

        // Find areas with units
        let my_areas: Vec<AreaId> = state.areas.iter().enumerate()
            .filter(|(_, a)| a.house == Some(house) && !a.units.is_empty())
            .map(|(i, _)| AreaId(i as u8))
            .collect();

        // Place a defense order on first area
        let orders: Vec<(AreaId, u8)> = my_areas.iter()
            .enumerate()
            .map(|(i, &a)| (a, i as u8))
            .collect();

        apply_action(&mut state, Action::PlaceOrders(orders.clone()));
        advance(&mut state);

        // Orders should be placed
        for (area_id, _token_idx) in &orders {
            let area_state = state.area(*area_id);
            assert!(area_state.order.is_some(),
                "Area {:?} should have an order placed", area_id);
            assert_eq!(area_state.order.unwrap().house, house);
        }
    }

    // ═════════════════════════════════════════════════════════════════════
    // DETERMINISM TESTS
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_deterministic_game() {
        // Same seed → same winner
        let g1 = play_full_game_random(12345, 6);
        let g2 = play_full_game_random(12345, 6);
        assert_eq!(g1.winner, g2.winner);
        assert_eq!(g1.round, g2.round);
    }

    #[test]
    fn test_different_seeds_different_outcomes() {
        // Over several seeds, we should see at least 2 different winners
        let mut winners = Vec::new();
        for seed in 0..20 {
            let g = play_full_game_random(seed * 1000, 6);
            if let Some(w) = g.winner {
                winners.push(w);
            }
        }
        // At least 2 different winners across 20 games
        winners.sort_by_key(|w| *w as u8);
        winners.dedup();
        assert!(winners.len() >= 2, "Random games should produce different winners");
    }

    // ═════════════════════════════════════════════════════════════════════
    // FULL GAME SIMULATION TESTS
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_game_completes_6p() {
        let state = play_full_game_random(42, 6);
        assert!(state.winner.is_some() || state.round > 10,
            "6-player game should complete (winner or round limit)");
    }

    #[test]
    fn test_game_completes_5p() {
        let state = play_full_game_random(42, 5);
        assert!(state.winner.is_some() || state.round > 10,
            "5-player game should complete");
    }

    #[test]
    fn test_game_completes_4p() {
        let state = play_full_game_random(42, 4);
        assert!(state.winner.is_some() || state.round > 10,
            "4-player game should complete");
    }

    #[test]
    fn test_game_completes_3p() {
        let state = play_full_game_random(42, 3);
        assert!(state.winner.is_some() || state.round > 10,
            "3-player game should complete");
    }

    #[test]
    fn test_stress_multiple_games() {
        // Run 20 games with different seeds to catch edge cases
        for seed in 0..20u64 {
            let state = play_full_game_random(seed * 7919, 6);
            assert!(state.winner.is_some() || state.round > 10,
                "Game with seed {} should complete", seed * 7919);
        }
    }

    #[test]
    fn test_stress_different_player_counts() {
        for pc in 3..=6u8 {
            for seed in 0..5u64 {
                let state = play_full_game_random(seed * 6703 + pc as u64, pc);
                assert!(state.winner.is_some() || state.round > 10,
                    "Game with {} players, seed {} should complete", pc, seed);
            }
        }
    }

    // ═════════════════════════════════════════════════════════════════════
    // COMBAT STATE TESTS
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_combat_state_default() {
        let state = make_6p_state(42);
        assert!(state.combat.is_none());
    }

    #[test]
    fn test_garrison_helps_defender() {
        let state = make_6p_state(42);
        // King's Landing has neutral garrison (strength 5)
        let g = state.garrisons.get(&KINGS_LANDING).unwrap();
        assert_eq!(g.house, None); // Neutral
        assert_eq!(g.strength, 5);
    }

    // ═════════════════════════════════════════════════════════════════════
    // HOUSE CARD ABILITY TESTS (structural)
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_all_houses_have_correct_cards() {
        for h in HouseName::ALL {
            let ids = cards::all_house_card_ids(h);
            assert_eq!(ids.len(), 7, "{:?} should have 7 card IDs", h);
            for id in ids {
                let card = cards::get_house_card(id);
                assert_eq!(card.house, h, "{:?} should belong to {:?}", id, h);
            }
        }
    }

    #[test]
    fn test_specific_card_abilities() {
        // Verify key cards have correct stats
        let tywin = cards::get_house_card(HouseCardId::TywinLannister);
        assert_eq!(tywin.strength, 4);
        assert_eq!(tywin.swords, 0);        // Tywin's ability is special, not from swords
        assert_eq!(tywin.fortifications, 0);

        let gregor = cards::get_house_card(HouseCardId::SerGregorClegane);
        assert_eq!(gregor.strength, 3);
        assert_eq!(gregor.swords, 3); // Gregor has 3 swords

        let stannis = cards::get_house_card(HouseCardId::StannisBaratheon);
        assert_eq!(stannis.strength, 4);

        let euron = cards::get_house_card(HouseCardId::EuronCrowsEye);
        assert_eq!(euron.strength, 4);
        assert_eq!(euron.swords, 1);
    }

    // ═════════════════════════════════════════════════════════════════════
    // ROUND / PHASE PROGRESSION TESTS
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_round1_skips_westeros() {
        let mut state = make_6p_state(42);
        assert_eq!(state.phase, Phase::Planning); // Round 1 starts in Planning
        advance(&mut state);
        // Should get a PlaceOrders pending, not a Westeros decision
        match &state.pending {
            Some(PendingDecision::PlaceOrders { .. }) => {}
            other => panic!("Round 1 should go straight to Planning, got {:?}", other),
        }
    }

    // ═════════════════════════════════════════════════════════════════════
    // BIDDING TYPE TESTS
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_bidding_types_exist() {
        // Verify all bidding types can be constructed
        let _it = BiddingType::IronThrone;
        let _fief = BiddingType::Fiefdoms;
        let _kc = BiddingType::KingsCourt;
        let _w = BiddingType::Wildling;
    }

    // ═════════════════════════════════════════════════════════════════════
    // AREA DEFAULT STATE TESTS
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_area_state_default() {
        let area: AreaState = Default::default();
        assert!(area.units.is_empty());
        assert!(area.order.is_none());
        assert!(area.house.is_none());
        assert!(!area.blocked);
    }

    // ═════════════════════════════════════════════════════════════════════
    // UNIT POOL TESTS
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_unit_pool_get_set() {
        let mut pool = UnitPool { footmen: 5, knights: 3, ships: 2, siege_engines: 1 };
        assert_eq!(pool.get(UnitType::Footman), 5);
        assert_eq!(pool.get(UnitType::Knight), 3);
        *pool.get_mut(UnitType::Ship) = 4;
        assert_eq!(pool.get(UnitType::Ship), 4);
    }

    // ═════════════════════════════════════════════════════════════════════
    // HOUSE NAME TESTS
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_house_display() {
        assert_eq!(format!("{}", HouseName::Stark), "Stark");
        assert_eq!(format!("{}", HouseName::Lannister), "Lannister");
        assert_eq!(format!("{}", HouseName::Baratheon), "Baratheon");
    }

    #[test]
    fn test_house_all() {
        assert_eq!(HouseName::ALL.len(), 6);
    }

    // ═════════════════════════════════════════════════════════════════════
    // VISIBILITY / PLAYER VIEW TESTS
    // ═════════════════════════════════════════════════════════════════════

    #[test]
    fn test_player_view_hides_opponent_hand() {
        use crate::visibility::player_view;
        let mut state = make_6p_state(42);
        advance(&mut state);

        let view = player_view(&state, HouseName::Stark);
        // Stark should see own hand
        assert_eq!(view.my_hand.len(), 7);
        // Opponent hand sizes should be available via house_info
        for (h, info) in &view.house_info {
            if *h != HouseName::Stark {
                assert_eq!(info.cards_in_hand, 7, "{:?} should show 7 cards in hand", h);
            }
        }
    }
}

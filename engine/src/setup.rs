// ═══════════════════════════════════════════════════════════════════════
// Game setup — creates initial GameState for N players
// Ported from TypeScript setup.ts
// ═══════════════════════════════════════════════════════════════════════

use crate::types::*;
use crate::map::*;
use crate::cards;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;

/// Starting configuration for a house.
struct HouseSetup {
    home_area: AreaId,
    initial_supply: u8,
    minimum_players: u8,
    iron_throne: u8,
    fiefdoms: u8,
    kings_court: u8,
    starting_units: Vec<(AreaId, Vec<UnitType>)>,
}

fn house_setups() -> Vec<(HouseName, HouseSetup)> {
    vec![
        (HouseName::Stark, HouseSetup {
            home_area: WINTERFELL,
            initial_supply: 1,
            minimum_players: 3,
            iron_throne: 3, fiefdoms: 4, kings_court: 2,
            starting_units: vec![
                (WINTERFELL, vec![UnitType::Knight, UnitType::Footman]),
                (WHITE_HARBOR, vec![UnitType::Footman]),
                (THE_SHIVERING_SEA, vec![UnitType::Ship]),
            ],
        }),
        (HouseName::Lannister, HouseSetup {
            home_area: LANNISPORT,
            initial_supply: 2,
            minimum_players: 3,
            iron_throne: 2, fiefdoms: 6, kings_court: 1,
            starting_units: vec![
                (LANNISPORT, vec![UnitType::Knight, UnitType::Footman]),
                (STONEY_SEPT, vec![UnitType::Footman]),
                (THE_GOLDEN_SOUND, vec![UnitType::Ship]),
            ],
        }),
        (HouseName::Baratheon, HouseSetup {
            home_area: DRAGONSTONE,
            initial_supply: 2,
            minimum_players: 3,
            iron_throne: 1, fiefdoms: 5, kings_court: 4,
            starting_units: vec![
                (DRAGONSTONE, vec![UnitType::Knight, UnitType::Footman]),
                (KINGSWOOD, vec![UnitType::Footman]),
                (SHIPBREAKER_BAY, vec![UnitType::Ship, UnitType::Ship]),
            ],
        }),
        (HouseName::Greyjoy, HouseSetup {
            home_area: PYKE,
            initial_supply: 2,
            minimum_players: 4,
            iron_throne: 5, fiefdoms: 1, kings_court: 6,
            starting_units: vec![
                (PYKE, vec![UnitType::Knight, UnitType::Footman]),
                (PYKE_PORT, vec![UnitType::Ship]),
                (GREYWATER_WATCH, vec![UnitType::Footman]),
                (IRONMANS_BAY, vec![UnitType::Ship]),
            ],
        }),
        (HouseName::Tyrell, HouseSetup {
            home_area: HIGHGARDEN,
            initial_supply: 2,
            minimum_players: 5,
            iron_throne: 6, fiefdoms: 2, kings_court: 5,
            starting_units: vec![
                (HIGHGARDEN, vec![UnitType::Knight, UnitType::Footman]),
                (DORNISH_MARCHES, vec![UnitType::Footman]),
                (REDWYNE_STRAITS, vec![UnitType::Ship]),
            ],
        }),
        (HouseName::Martell, HouseSetup {
            home_area: SUNSPEAR,
            initial_supply: 2,
            minimum_players: 6,
            iron_throne: 4, fiefdoms: 3, kings_court: 3,
            starting_units: vec![
                (SUNSPEAR, vec![UnitType::Knight, UnitType::Footman]),
                (SALT_SHORE, vec![UnitType::Footman]),
                (SEA_OF_DORNE, vec![UnitType::Ship]),
            ],
        }),
    ]
}

/// Create the initial game state for a given number of players (3–6).
/// Seed controls deck shuffling for reproducibility.
pub fn create_initial_state(player_count: u8, seed: u64) -> GameState {
    assert!((3..=6).contains(&player_count), "Player count must be 3–6");

    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let setups = house_setups();
    let playing: Vec<(HouseName, HouseSetup)> = setups
        .into_iter()
        .filter(|(_, s)| s.minimum_players <= player_count)
        .collect();

    let playing_houses: Vec<HouseName> = playing.iter().map(|(h, _)| *h).collect();

    // Turn order = sorted by iron throne position
    let mut turn_order = playing_houses.clone();
    turn_order.sort_by_key(|h| {
        playing.iter().find(|(name, _)| name == h).unwrap().1.iron_throne
    });

    // Initialize area states
    let mut areas: Vec<AreaState> = (0..NUM_AREAS)
        .map(|_| AreaState::default())
        .collect();

    // Initialize house profiles
    let mut houses = HashMap::new();
    for (house_name, setup) in &playing {
        // Starting unit pool
        let mut pool = UnitPool {
            footmen: 10,
            knights: 5,
            ships: 6,
            siege_engines: 2,
        };

        // Place starting units
        for (area_id, unit_types) in &setup.starting_units {
            for &ut in unit_types {
                let unit = Unit {
                    unit_type: ut,
                    house: *house_name,
                    routed: false,
                };
                areas[area_id.0 as usize].units.push(unit);
                areas[area_id.0 as usize].house = Some(*house_name);
                *pool.get_mut(ut) -= 1;
            }
        }

        // Also set control for home area even if no units specifically there
        areas[setup.home_area.0 as usize].house = Some(*house_name);

        houses.insert(*house_name, HouseProfile {
            name: *house_name,
            iron_throne: setup.iron_throne,
            fiefdoms: setup.fiefdoms,
            kings_court: setup.kings_court,
            supply: setup.initial_supply,
            power: 5,
            available_units: pool,
            hand: cards::all_house_card_ids(*house_name),
            discards: Vec::new(),
            used_order_tokens: Vec::new(),
        });
    }

    // Garrisons
    let mut garrisons = HashMap::new();
    for (house_name, setup) in &playing {
        if let Some(strength) = initial_garrison_strength(setup.home_area) {
            garrisons.insert(setup.home_area, Garrison {
                house: *house_name,
                strength,
            });
        }
    }

    // Neutral garrisons for fewer players
    // 4-player: Tyrell + Martell zones get neutral garrisons
    // 5-player: Martell zones get neutral garrisons
    if player_count <= 4 {
        // Highgarden, Oldtown get neutral garrison if Tyrell not playing
        if !playing_houses.contains(&HouseName::Tyrell) {
            // These are "neutral" — no house, but have garrison strength
            // In the game, neutral garrisons don't have a house
        }
    }

    // Block areas for 3-player game (southern regions)
    if player_count == 3 {
        let blocked_areas = [
            SUNSPEAR, SALT_SHORE, STARFALL, YRONWOOD, PRINCES_PASS,
            THE_BONEWAY, THREE_TOWERS, DORNISH_MARCHES,
            HIGHGARDEN, OLDTOWN, THE_ARBOR,
            SEA_OF_DORNE, EAST_SUMMER_SEA, WEST_SUMMER_SEA, REDWYNE_STRAITS,
            SUNSPEAR_PORT, HIGHGARDEN_PORT, OLDTOWN_PORT,
        ];
        for &area in &blocked_areas {
            areas[area.0 as usize].blocked = true;
        }
    }

    // Neutral garrison at King's Landing and The Eyrie (always)
    garrisons.entry(KINGS_LANDING).or_insert(Garrison {
        house: HouseName::Baratheon, // placeholder — neutral in practice
        strength: 5,
    });
    garrisons.entry(THE_EYRIE).or_insert(Garrison {
        house: HouseName::Stark, // placeholder
        strength: 6,
    });

    // Shuffle decks
    let mut deck1 = cards::westeros_deck_1();
    let mut deck2 = cards::westeros_deck_2();
    let mut deck3 = cards::westeros_deck_3();
    let mut wildling = cards::wildling_deck();
    deck1.shuffle(&mut rng);
    deck2.shuffle(&mut rng);
    deck3.shuffle(&mut rng);
    wildling.shuffle(&mut rng);

    GameState {
        round: 1,
        phase: Phase::Planning, // Westeros skipped on round 1
        action_sub_phase: ActionSubPhase::Raid,
        action_player_index: 0,
        houses,
        areas,
        turn_order,
        wildling_threat: 2,
        garrisons,
        valyrian_steel_blade_used: false,
        messenger_raven_used: false,
        westeros_deck_1: deck1,
        westeros_deck_2: deck2,
        westeros_deck_3: deck3,
        wildling_deck: wildling,
        order_restrictions: Vec::new(),
        star_order_restrictions: Vec::new(),
        combat: None,
        pending: None,
        winner: None,
        playing_houses,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_6_player_game() {
        let state = create_initial_state(6, 42);
        assert_eq!(state.playing_houses.len(), 6);
        assert_eq!(state.round, 1);
        assert_eq!(state.phase, Phase::Planning);
        assert_eq!(state.wildling_threat, 2);

        // Baratheon should be first in turn order (iron throne = 1)
        assert_eq!(state.turn_order[0], HouseName::Baratheon);

        // Each house should have 7 cards in hand
        for h in &state.playing_houses {
            assert_eq!(state.house(*h).hand.len(), 7);
            assert_eq!(state.house(*h).power, 5);
        }

        // Stark should have units in Winterfell, White Harbor, Shivering Sea
        let winterfell_units = &state.areas[WINTERFELL.0 as usize].units;
        assert_eq!(winterfell_units.len(), 2);
        assert_eq!(state.areas[WINTERFELL.0 as usize].house, Some(HouseName::Stark));
    }

    #[test]
    fn test_create_3_player_game() {
        let state = create_initial_state(3, 42);
        assert_eq!(state.playing_houses.len(), 3);
        // Southern areas should be blocked
        assert!(state.areas[SUNSPEAR.0 as usize].blocked);
        assert!(state.areas[HIGHGARDEN.0 as usize].blocked);
    }

    #[test]
    fn test_deterministic_seed() {
        let s1 = create_initial_state(6, 123);
        let s2 = create_initial_state(6, 123);
        // Same seed → same deck order
        assert_eq!(s1.westeros_deck_1.len(), s2.westeros_deck_1.len());
        for i in 0..s1.westeros_deck_1.len() {
            assert_eq!(s1.westeros_deck_1[i].card_type, s2.westeros_deck_1[i].card_type);
        }
    }
}

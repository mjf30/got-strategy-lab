// ═══════════════════════════════════════════════════════════════════════
// Core types — ported from TypeScript types.ts
// ═══════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Enums ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HouseName {
    Stark,
    Lannister,
    Baratheon,
    Greyjoy,
    Tyrell,
    Martell,
}

impl HouseName {
    pub const ALL: [HouseName; 6] = [
        HouseName::Stark,
        HouseName::Lannister,
        HouseName::Baratheon,
        HouseName::Greyjoy,
        HouseName::Tyrell,
        HouseName::Martell,
    ];
}

impl std::fmt::Display for HouseName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HouseName::Stark => write!(f, "Stark"),
            HouseName::Lannister => write!(f, "Lannister"),
            HouseName::Baratheon => write!(f, "Baratheon"),
            HouseName::Greyjoy => write!(f, "Greyjoy"),
            HouseName::Tyrell => write!(f, "Tyrell"),
            HouseName::Martell => write!(f, "Martell"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnitType {
    Footman,
    Knight,
    Ship,
    SiegeEngine,
}

impl UnitType {
    /// Base combat strength of this unit type.
    /// SiegeEngine is 4 when attacking castle/stronghold, 0 otherwise — handled in combat.
    pub fn combat_strength(self) -> u8 {
        match self {
            UnitType::Footman => 1,
            UnitType::Knight => 2,
            UnitType::Ship => 1,
            UnitType::SiegeEngine => 0, // contextual
        }
    }

    /// Mustering cost in points (stronghold=2pts, castle=1pt).
    pub fn muster_cost(self) -> u8 {
        match self {
            UnitType::Footman => 1,
            UnitType::Knight => 2,
            UnitType::Ship => 1,
            UnitType::SiegeEngine => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderType {
    March,
    Raid,
    Support,
    Defense,
    ConsolidatePower,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AreaType {
    Land,
    Sea,
    Port,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Phase {
    Westeros,
    Planning,
    Action,
    Combat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionSubPhase {
    Raid,
    March,
    ConsolidatePower,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Track {
    IronThrone,
    Fiefdoms,
    KingsCourt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CombatPhase {
    Support,
    Cards,
    PreCombat,
    Resolution,
    PostCombat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SupportChoice {
    Attacker,
    Defender,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BiddingType {
    IronThrone,
    Fiefdoms,
    KingsCourt,
    Wildling,
}

// ── Area ID ────────────────────────────────────────────────────────────
// Compact, copyable area identifier. Index into the static AREAS array.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct AreaId(pub u8);

// ── Unit ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unit {
    pub unit_type: UnitType,
    pub house: HouseName,
    pub routed: bool,
}

// ── Order Token ────────────────────────────────────────────────────────

/// Definition of the 15 order tokens each house owns (static data).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderTokenDef {
    pub order_type: OrderType,
    pub strength: i8, // can be -1 for March-1
    pub star: bool,
}

/// An order placed on an area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Order {
    pub order_type: OrderType,
    pub strength: i8,
    pub star: bool,
    pub house: HouseName,
    pub token_index: u8, // index into ORDER_TOKENS
}

/// The 15 order tokens, same for every house.
pub const ORDER_TOKENS: [OrderTokenDef; 15] = [
    // March (3)
    OrderTokenDef { order_type: OrderType::March, strength: -1, star: false },
    OrderTokenDef { order_type: OrderType::March, strength: 0,  star: false },
    OrderTokenDef { order_type: OrderType::March, strength: 1,  star: true },
    // Defense (3)
    OrderTokenDef { order_type: OrderType::Defense, strength: 1, star: false },
    OrderTokenDef { order_type: OrderType::Defense, strength: 1, star: false },
    OrderTokenDef { order_type: OrderType::Defense, strength: 2, star: true },
    // Support (3)
    OrderTokenDef { order_type: OrderType::Support, strength: 0, star: false },
    OrderTokenDef { order_type: OrderType::Support, strength: 0, star: false },
    OrderTokenDef { order_type: OrderType::Support, strength: 1, star: true },
    // Raid (3)
    OrderTokenDef { order_type: OrderType::Raid, strength: 0, star: false },
    OrderTokenDef { order_type: OrderType::Raid, strength: 0, star: false },
    OrderTokenDef { order_type: OrderType::Raid, strength: 0, star: true },
    // Consolidate Power (3)
    OrderTokenDef { order_type: OrderType::ConsolidatePower, strength: 0, star: false },
    OrderTokenDef { order_type: OrderType::ConsolidatePower, strength: 0, star: false },
    OrderTokenDef { order_type: OrderType::ConsolidatePower, strength: 0, star: true },
];

// ── Star Order Limits ──────────────────────────────────────────────────

/// Returns max number of starred orders allowed for a given King's Court position (1-based).
pub fn star_order_limit(player_count: u8, position: u8) -> u8 {
    // Official 2nd Edition values
    match player_count {
        6 => match position { 1 => 3, 2 => 3, 3 => 2, 4 => 1, _ => 0 },
        5 => match position { 1 => 3, 2 => 3, 3 => 2, 4 => 1, _ => 0 },
        4 => match position { 1 => 3, 2 => 3, 3 => 1, _ => 0 },
        3 => match position { 1 => 3, 2 => 2, 3 => 1, _ => 0 },
        _ => 0,
    }
}

// ── House Card ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HouseCardId {
    // Stark
    EddardStark, RobbStark, GreatjonUmber, RooseBolton, TheBlackfish, SerRodrikCassel, CatelynStark,
    // Lannister
    TywinLannister, SerGregorClegane, SerJaimeLannister, TheHound, TyrionLannister, SerKevanLannister, CerseiLannister,
    // Baratheon
    StannisBaratheon, RenlyBaratheon, BrienneOfTarth, SerDavosSeaworth, Melisandre, SalladhorSaan, Patchface,
    // Greyjoy
    EuronCrowsEye, VictarionGreyjoy, BalonGreyjoy, TheonGreyjoy, AshaGreyjoy, DagmerCleftjaw, AeronDamphair,
    // Tyrell
    MaceTyrell, SerLorasTyrell, SerGarlanTyrell, RandyllTarly, MargaeryTyrell, AlesterFlorent, QueenOfThorns,
    // Martell
    TheRedViper, AreoHotah, ObaraSand, Darkstar, NymeriaSand, ArianneMartell, DoranMartell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HouseCard {
    pub id: HouseCardId,
    pub house: HouseName,
    pub strength: u8,
    pub swords: u8,
    pub fortifications: u8,
}

// ── Westeros Card ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WesterosCardType {
    // Deck 1
    Supply,
    Mustering,
    AThroneOfBlades,
    // Deck 2
    ClashOfKings,
    GameOfThrones,
    DarkWingsDarkWords,
    // Deck 3
    WildlingAttack,
    PutToTheSword,
    SeaOfStorms,
    RainsOfAutumn,
    FeastForCrows,
    WebOfLies,
    StormOfSwords,
    // Special
    WinterIsComing,
    LastDaysOfSummer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WesterosCard {
    pub deck: u8, // 1, 2, or 3
    pub card_type: WesterosCardType,
    pub wildling_icon: bool,
}

// ── Wildling Card ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WildlingCardType {
    AKingBeyondTheWall,
    CrowKillers,
    MammothRiders,
    MassingOnTheMilkwater,
    PreemptiveRaid,
    RattleshirtsRaiders,
    SilenceAtTheWall,
    SkinchangerScout,
    TheHordeDescends,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WildlingCard {
    pub card_type: WildlingCardType,
}

// ── Supply Table ───────────────────────────────────────────────────────

/// Returns max army sizes allowed for a given supply level (0–6).
/// Armies = groups of 2+ units in the same land area.
pub fn supply_limits(supply: u8) -> &'static [u8] {
    match supply.min(6) {
        0 => &[2, 2],
        1 => &[3, 2],
        2 => &[3, 2, 2],
        3 => &[3, 2, 2, 2],
        4 => &[3, 3, 2, 2],
        5 => &[4, 3, 2, 2],
        6 => &[4, 3, 2, 2, 2],
        _ => unreachable!(),
    }
}

// ── Garrison ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Garrison {
    pub house: Option<HouseName>, // None = neutral (King's Landing, The Eyrie)
    pub strength: u8,
}

// ── Bidding State ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiddingState {
    pub bidding_type: BiddingType,
    pub bids: HashMap<HouseName, u8>,
    pub current_track: Option<Track>,
    pub remaining_tracks: Vec<Track>,
    pub bid_order: Vec<HouseName>,
    pub next_bidder_idx: usize,
}

// ── Combat State ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatState {
    pub attacker: HouseName,
    pub defender: HouseName,
    pub area_id: AreaId,
    pub attacking_units: Vec<Unit>,
    pub defending_units: Vec<Unit>,
    pub attacker_card: Option<HouseCardId>,
    pub defender_card: Option<HouseCardId>,
    pub attacker_strength: i16,
    pub defender_strength: i16,
    pub march_from_area: Option<AreaId>,
    pub attacker_used_blade: bool,
    pub defender_used_blade: bool,
    pub support_decisions: HashMap<AreaId, SupportChoice>,
    pub phase: CombatPhase,
    pub aeron_resolved: bool,
    pub tyrion_resolved: bool,
    pub pending_support_houses: Vec<(AreaId, HouseName)>,
}

// ── House Profile (per-player state) ───────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HouseProfile {
    pub name: HouseName,
    pub iron_throne: u8,    // position 1–6 (1 = top)
    pub fiefdoms: u8,       // position 1–6
    pub kings_court: u8,    // position 1–6
    pub supply: u8,         // 0–6
    pub power: u8,          // available power tokens
    pub available_units: UnitPool,
    pub hand: Vec<HouseCardId>,       // cards in hand (PRIVATE)
    pub discards: Vec<HouseCardId>,   // played cards (PUBLIC)
    pub used_order_tokens: Vec<u8>,   // indices into ORDER_TOKENS used this round
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct UnitPool {
    pub footmen: u8,
    pub knights: u8,
    pub ships: u8,
    pub siege_engines: u8,
}

impl UnitPool {
    pub fn get(&self, ut: UnitType) -> u8 {
        match ut {
            UnitType::Footman => self.footmen,
            UnitType::Knight => self.knights,
            UnitType::Ship => self.ships,
            UnitType::SiegeEngine => self.siege_engines,
        }
    }
    pub fn get_mut(&mut self, ut: UnitType) -> &mut u8 {
        match ut {
            UnitType::Footman => &mut self.footmen,
            UnitType::Knight => &mut self.knights,
            UnitType::Ship => &mut self.ships,
            UnitType::SiegeEngine => &mut self.siege_engines,
        }
    }
}

// ── Area (board tile) ──────────────────────────────────────────────────

/// Dynamic per-area state during a game.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct AreaState {
    pub units: Vec<Unit>,
    pub order: Option<Order>,
    pub house: Option<HouseName>,  // controlling house
    pub blocked: bool,             // impassable in 3-player
}


// ── Pending Decision Types ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PendingDecision {
    /// Westeros card choice (e.g. Throne of Blades: Supply or Mustering)
    WesterosChoice {
        card_name: String,
        chooser: HouseName,
        options: Vec<String>,
    },
    /// Support declaration during combat
    SupportDeclaration {
        house: HouseName,
        area_id: AreaId,
        attacker: HouseName,
        defender: HouseName,
    },
    /// Tyrion cancelled opponent's card — must choose replacement
    TyrionReplace {
        opponent: HouseName,
    },
    /// Aeron Damphair: pay 2 power to swap card?
    AeronSwap {
        house: HouseName,
    },
    /// Patchface: choose card to discard from opponent's hand
    PatchfaceDiscard {
        opponent: HouseName,
        visible_cards: Vec<HouseCardId>,
    },
    /// Robb Stark: winner chooses defender retreat area
    RobbRetreat {
        house: HouseName,
        possible_areas: Vec<AreaId>,
    },
    /// Generic retreat: loser picks retreat destination
    Retreat {
        house: HouseName,
        units: Vec<Unit>,
        from_area: AreaId,
        possible_areas: Vec<AreaId>,
    },
    /// Reconcile armies to supply limits
    Reconcile {
        house: HouseName,
        area_id: AreaId,
        current_size: u8,
        max_allowed: u8,
    },
    /// Mustering: choose what to build
    Muster {
        house: HouseName,
        areas: Vec<MusterArea>,
    },
    /// Bidding (Clash of Kings / Wildling Attack)
    Bidding {
        house: HouseName,
        bidding_type: BiddingType,
        track: Option<Track>,
    },
    /// Choose whether to leave a power token when vacating land
    LeavePowerToken {
        house: HouseName,
        area_id: AreaId,
    },
    /// Use Valyrian Steel Blade in combat?
    UseValyrianBlade {
        house: HouseName,
    },
    /// Place orders (planning phase)
    PlaceOrders {
        house: HouseName,
    },
    /// Choose raid target
    ChooseRaid {
        house: HouseName,
        from_area: AreaId,
        valid_targets: Vec<AreaId>,
    },
    /// Choose march destination
    ChooseMarch {
        house: HouseName,
        from_area: AreaId,
        valid_destinations: Vec<AreaId>,
    },
    /// Select house card for combat
    SelectHouseCard {
        house: HouseName,
        available_cards: Vec<HouseCardId>,
    },
    /// Messenger Raven: swap an order after reveal
    MessengerRaven {
        house: HouseName,
    },
    /// Preemptive Raid wildling penalty choice: destroy 2 units or lose 2 track positions
    WildlingPenaltyChoice {
        house: HouseName,
        options: Vec<String>,
    },
    /// Cersei Lannister: choose opponent order to remove
    CerseiRemoveOrder {
        opponent: HouseName,
    },
    /// Doran Martell: choose influence track to move opponent to bottom
    DoranChooseTrack {
        opponent: HouseName,
    },
    /// Queen of Thorns: remove adjacent opponent order
    QueenOfThornsRemoveOrder {
        opponent: HouseName,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusterArea {
    pub area_id: AreaId,
    pub points: u8, // 2 for stronghold, 1 for castle
}

// ── Game State ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub round: u8, // 1–10
    pub phase: Phase,
    pub action_sub_phase: ActionSubPhase,
    pub action_player_index: u8,

    /// Per-house profiles. Order matches HouseName::ALL.
    pub houses: HashMap<HouseName, HouseProfile>,
    /// Dynamic state per area, indexed by AreaId.
    pub areas: Vec<AreaState>,
    /// Turn order (Iron Throne track order).
    pub turn_order: Vec<HouseName>,
    /// Wildling threat (0–12).
    pub wildling_threat: u8,
    /// Garrisons on areas.
    pub garrisons: HashMap<AreaId, Garrison>,

    // Dominance token tracking
    pub valyrian_steel_blade_used: bool,
    pub messenger_raven_used: bool,

    // Card decks (shuffled at game start, draw from front)
    pub westeros_deck_1: Vec<WesterosCard>,
    pub westeros_deck_2: Vec<WesterosCard>,
    pub westeros_deck_3: Vec<WesterosCard>,
    pub wildling_deck: Vec<WildlingCard>,

    // Order restrictions this round (from Westeros cards)
    pub order_restrictions: Vec<OrderType>,
    pub star_order_restrictions: Vec<OrderType>,

    // Active combat (if any)
    pub combat: Option<CombatState>,

    // Bidding state (Clash of Kings / Wildling Attack)
    pub bidding: Option<BiddingState>,

    // Westeros phase tracking
    pub westeros_cards_drawn: Vec<WesterosCard>,
    pub westeros_step: u8,
    pub muster_house_idx: u8,

    // Deterministic RNG
    pub seed: u64,
    pub rng_counter: u64,

    // Current pending decision the game is waiting on
    pub pending: Option<PendingDecision>,

    // Winner (if game over)
    pub winner: Option<HouseName>,

    // Which houses are playing (subset of HouseName::ALL based on player count)
    pub playing_houses: Vec<HouseName>,
}

impl GameState {
    /// How many players in this game.
    pub fn player_count(&self) -> u8 {
        self.playing_houses.len() as u8
    }

    /// Get the house profile for a house.
    pub fn house(&self, h: HouseName) -> &HouseProfile {
        &self.houses[&h]
    }

    /// Get mutable house profile.
    pub fn house_mut(&mut self, h: HouseName) -> &mut HouseProfile {
        self.houses.get_mut(&h).unwrap()
    }

    /// Get area state by AreaId.
    pub fn area(&self, id: AreaId) -> &AreaState {
        &self.areas[id.0 as usize]
    }

    /// Get mutable area state.
    pub fn area_mut(&mut self, id: AreaId) -> &mut AreaState {
        &mut self.areas[id.0 as usize]
    }

    /// Current player whose turn it is in the action phase.
    pub fn current_action_player(&self) -> HouseName {
        self.turn_order[self.action_player_index as usize]
    }
}

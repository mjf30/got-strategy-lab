// ═══════════════════════════════════════════════════════════════════════
// House cards, Westeros deck, Wildling deck — static data
// ═══════════════════════════════════════════════════════════════════════

use crate::types::*;

// ── House Cards ────────────────────────────────────────────────────────

pub fn house_cards(house: HouseName) -> Vec<HouseCard> {
    match house {
        HouseName::Stark => vec![
            HouseCard { id: HouseCardId::EddardStark,     house, strength: 4, swords: 2, fortifications: 0 },
            HouseCard { id: HouseCardId::RobbStark,        house, strength: 3, swords: 0, fortifications: 0 },
            HouseCard { id: HouseCardId::GreatjonUmber,    house, strength: 2, swords: 2, fortifications: 0 },
            HouseCard { id: HouseCardId::RooseBolton,      house, strength: 2, swords: 0, fortifications: 0 },
            HouseCard { id: HouseCardId::TheBlackfish,     house, strength: 1, swords: 0, fortifications: 0 },
            HouseCard { id: HouseCardId::SerRodrikCassel,  house, strength: 1, swords: 0, fortifications: 2 },
            HouseCard { id: HouseCardId::CatelynStark,     house, strength: 0, swords: 0, fortifications: 0 },
        ],
        HouseName::Lannister => vec![
            HouseCard { id: HouseCardId::TywinLannister,      house, strength: 4, swords: 0, fortifications: 0 },
            HouseCard { id: HouseCardId::SerGregorClegane,     house, strength: 3, swords: 3, fortifications: 0 },
            HouseCard { id: HouseCardId::SerJaimeLannister,    house, strength: 2, swords: 1, fortifications: 0 },
            HouseCard { id: HouseCardId::TheHound,             house, strength: 2, swords: 0, fortifications: 2 },
            HouseCard { id: HouseCardId::TyrionLannister,      house, strength: 1, swords: 0, fortifications: 0 },
            HouseCard { id: HouseCardId::SerKevanLannister,    house, strength: 1, swords: 0, fortifications: 0 },
            HouseCard { id: HouseCardId::CerseiLannister,      house, strength: 0, swords: 0, fortifications: 0 },
        ],
        HouseName::Baratheon => vec![
            HouseCard { id: HouseCardId::StannisBaratheon,   house, strength: 4, swords: 0, fortifications: 0 },
            HouseCard { id: HouseCardId::RenlyBaratheon,     house, strength: 3, swords: 0, fortifications: 0 },
            HouseCard { id: HouseCardId::BrienneOfTarth,     house, strength: 2, swords: 1, fortifications: 1 },
            HouseCard { id: HouseCardId::SerDavosSeaworth,   house, strength: 2, swords: 0, fortifications: 0 },
            HouseCard { id: HouseCardId::Melisandre,         house, strength: 1, swords: 1, fortifications: 0 },
            HouseCard { id: HouseCardId::SalladhorSaan,      house, strength: 1, swords: 0, fortifications: 0 },
            HouseCard { id: HouseCardId::Patchface,          house, strength: 0, swords: 0, fortifications: 0 },
        ],
        HouseName::Greyjoy => vec![
            HouseCard { id: HouseCardId::EuronCrowsEye,     house, strength: 4, swords: 1, fortifications: 0 },
            HouseCard { id: HouseCardId::VictarionGreyjoy,   house, strength: 3, swords: 0, fortifications: 0 },
            HouseCard { id: HouseCardId::BalonGreyjoy,       house, strength: 2, swords: 0, fortifications: 0 },
            HouseCard { id: HouseCardId::TheonGreyjoy,       house, strength: 2, swords: 0, fortifications: 0 },
            HouseCard { id: HouseCardId::AshaGreyjoy,        house, strength: 1, swords: 0, fortifications: 0 },
            HouseCard { id: HouseCardId::DagmerCleftjaw,     house, strength: 1, swords: 1, fortifications: 1 },
            HouseCard { id: HouseCardId::AeronDamphair,      house, strength: 0, swords: 0, fortifications: 0 },
        ],
        HouseName::Tyrell => vec![
            HouseCard { id: HouseCardId::MaceTyrell,         house, strength: 4, swords: 0, fortifications: 0 },
            HouseCard { id: HouseCardId::SerLorasTyrell,     house, strength: 3, swords: 0, fortifications: 0 },
            HouseCard { id: HouseCardId::SerGarlanTyrell,    house, strength: 2, swords: 2, fortifications: 0 },
            HouseCard { id: HouseCardId::RandyllTarly,       house, strength: 2, swords: 2, fortifications: 0 },
            HouseCard { id: HouseCardId::MargaeryTyrell,     house, strength: 1, swords: 0, fortifications: 1 },
            HouseCard { id: HouseCardId::AlesterFlorent,     house, strength: 1, swords: 0, fortifications: 1 },
            HouseCard { id: HouseCardId::QueenOfThorns,      house, strength: 0, swords: 0, fortifications: 0 },
        ],
        HouseName::Martell => vec![
            HouseCard { id: HouseCardId::TheRedViper,        house, strength: 4, swords: 2, fortifications: 1 },
            HouseCard { id: HouseCardId::AreoHotah,          house, strength: 3, swords: 0, fortifications: 1 },
            HouseCard { id: HouseCardId::ObaraSand,          house, strength: 2, swords: 1, fortifications: 0 },
            HouseCard { id: HouseCardId::Darkstar,           house, strength: 2, swords: 1, fortifications: 0 },
            HouseCard { id: HouseCardId::NymeriaSand,        house, strength: 1, swords: 0, fortifications: 0 },
            HouseCard { id: HouseCardId::ArianneMartell,     house, strength: 1, swords: 0, fortifications: 0 },
            HouseCard { id: HouseCardId::DoranMartell,       house, strength: 0, swords: 0, fortifications: 0 },
        ],
    }
}

pub fn all_house_card_ids(house: HouseName) -> Vec<HouseCardId> {
    house_cards(house).into_iter().map(|c| c.id).collect()
}

pub fn get_house_card(id: HouseCardId) -> HouseCard {
    // Flat lookup across all houses
    for h in HouseName::ALL {
        for c in house_cards(h) {
            if c.id == id { return c; }
        }
    }
    panic!("Unknown house card id: {:?}", id);
}

// ── Westeros Decks ─────────────────────────────────────────────────────

pub fn westeros_deck_1() -> Vec<WesterosCard> {
    use WesterosCardType::*;
    vec![
        WesterosCard { deck: 1, card_type: Supply,          wildling_icon: false },
        WesterosCard { deck: 1, card_type: Supply,          wildling_icon: false },
        WesterosCard { deck: 1, card_type: Supply,          wildling_icon: false },
        WesterosCard { deck: 1, card_type: Mustering,       wildling_icon: false },
        WesterosCard { deck: 1, card_type: Mustering,       wildling_icon: false },
        WesterosCard { deck: 1, card_type: Mustering,       wildling_icon: false },
        WesterosCard { deck: 1, card_type: AThroneOfBlades, wildling_icon: true },
        WesterosCard { deck: 1, card_type: AThroneOfBlades, wildling_icon: true },
        WesterosCard { deck: 1, card_type: WinterIsComing,  wildling_icon: false },
        WesterosCard { deck: 1, card_type: LastDaysOfSummer, wildling_icon: true },
    ]
}

pub fn westeros_deck_2() -> Vec<WesterosCard> {
    use WesterosCardType::*;
    vec![
        WesterosCard { deck: 2, card_type: ClashOfKings,       wildling_icon: false },
        WesterosCard { deck: 2, card_type: ClashOfKings,       wildling_icon: false },
        WesterosCard { deck: 2, card_type: ClashOfKings,       wildling_icon: false },
        WesterosCard { deck: 2, card_type: GameOfThrones,      wildling_icon: false },
        WesterosCard { deck: 2, card_type: GameOfThrones,      wildling_icon: false },
        WesterosCard { deck: 2, card_type: GameOfThrones,      wildling_icon: false },
        WesterosCard { deck: 2, card_type: DarkWingsDarkWords, wildling_icon: true },
        WesterosCard { deck: 2, card_type: DarkWingsDarkWords, wildling_icon: true },
        WesterosCard { deck: 2, card_type: WinterIsComing,     wildling_icon: false },
        WesterosCard { deck: 2, card_type: LastDaysOfSummer,   wildling_icon: true },
    ]
}

pub fn westeros_deck_3() -> Vec<WesterosCard> {
    use WesterosCardType::*;
    vec![
        WesterosCard { deck: 3, card_type: WildlingAttack,  wildling_icon: false },
        WesterosCard { deck: 3, card_type: WildlingAttack,  wildling_icon: false },
        WesterosCard { deck: 3, card_type: WildlingAttack,  wildling_icon: false },
        WesterosCard { deck: 3, card_type: PutToTheSword,   wildling_icon: true },
        WesterosCard { deck: 3, card_type: PutToTheSword,   wildling_icon: true },
        WesterosCard { deck: 3, card_type: SeaOfStorms,     wildling_icon: true },
        WesterosCard { deck: 3, card_type: RainsOfAutumn,   wildling_icon: true },
        WesterosCard { deck: 3, card_type: FeastForCrows,   wildling_icon: true },
        WesterosCard { deck: 3, card_type: WebOfLies,       wildling_icon: true },
        WesterosCard { deck: 3, card_type: StormOfSwords,   wildling_icon: true },
    ]
}

// ── Wildling Deck ──────────────────────────────────────────────────────

pub fn wildling_deck() -> Vec<WildlingCard> {
    use WildlingCardType::*;
    vec![
        WildlingCard { card_type: AKingBeyondTheWall },
        WildlingCard { card_type: CrowKillers },
        WildlingCard { card_type: MammothRiders },
        WildlingCard { card_type: MassingOnTheMilkwater },
        WildlingCard { card_type: PreemptiveRaid },
        WildlingCard { card_type: RattleshirtsRaiders },
        WildlingCard { card_type: SilenceAtTheWall },
        WildlingCard { card_type: SkinchangerScout },
        WildlingCard { card_type: TheHordeDescends },
    ]
}

// ═══════════════════════════════════════════════════════════════════════
// Static map data — ported from TypeScript map.ts
// All area properties that never change during a game.
// ═══════════════════════════════════════════════════════════════════════

use crate::types::{AreaId, AreaType};

/// Static description of a map area (compile-time constant).
#[derive(Debug, Clone)]
pub struct AreaDef {
    pub id: AreaId,
    pub name: &'static str,
    pub area_type: AreaType,
    pub castle: bool,
    pub stronghold: bool,
    pub supply_icons: u8,
    pub power_icons: u8,
    pub adjacent: &'static [AreaId],
    // Port-specific
    pub connected_land: Option<AreaId>,
    pub connected_sea: Option<AreaId>,
}

impl AreaDef {
    pub fn muster_points(&self) -> u8 {
        if self.stronghold { 2 }
        else if self.castle { 1 }
        else { 0 }
    }

    pub fn is_land(&self) -> bool { self.area_type == AreaType::Land }
    pub fn is_sea(&self) -> bool { self.area_type == AreaType::Sea }
    pub fn is_port(&self) -> bool { self.area_type == AreaType::Port }
    pub fn has_castle_or_stronghold(&self) -> bool { self.castle || self.stronghold }
}

// ── Area ID constants ──────────────────────────────────────────────────
// Ordered: Lands (0–33), Seas (34–46), Ports (47–55)

// LANDS — The North
pub const CASTLE_BLACK: AreaId        = AreaId(0);
pub const KARHOLD: AreaId             = AreaId(1);
pub const THE_STONY_SHORE: AreaId     = AreaId(2);
pub const WINTERFELL: AreaId          = AreaId(3);
pub const WHITE_HARBOR: AreaId        = AreaId(4);
pub const WIDOWS_WATCH: AreaId        = AreaId(5);
// LANDS — Riverlands / Central
pub const MOAT_CAILIN: AreaId         = AreaId(6);
pub const GREYWATER_WATCH: AreaId     = AreaId(7);
pub const FLINTS_FINGER: AreaId       = AreaId(8);
pub const SEAGARD: AreaId             = AreaId(9);
pub const THE_TWINS: AreaId           = AreaId(10);
pub const THE_FINGERS: AreaId         = AreaId(11);
pub const MOUNTAINS_OF_THE_MOON: AreaId = AreaId(12);
pub const THE_EYRIE: AreaId           = AreaId(13);
// LANDS — Westerlands
pub const RIVERRUN: AreaId            = AreaId(14);
pub const LANNISPORT: AreaId          = AreaId(15);
pub const STONEY_SEPT: AreaId         = AreaId(16);
pub const SEAROAD_MARCHES: AreaId     = AreaId(17);
// LANDS — Crownlands
pub const HARRENHAL: AreaId           = AreaId(18);
pub const CRACKCLAW_POINT: AreaId     = AreaId(19);
pub const KINGS_LANDING: AreaId       = AreaId(20);
pub const BLACKWATER: AreaId          = AreaId(21);
// LANDS — South
pub const KINGSWOOD: AreaId           = AreaId(22);
pub const STORMS_END: AreaId          = AreaId(23);
pub const HIGHGARDEN: AreaId          = AreaId(24);
pub const THE_REACH: AreaId           = AreaId(25);
pub const DORNISH_MARCHES: AreaId     = AreaId(26);
pub const OLDTOWN: AreaId             = AreaId(27);
pub const THREE_TOWERS: AreaId        = AreaId(28);
// LANDS — Dorne
pub const THE_BONEWAY: AreaId         = AreaId(29);
pub const PRINCES_PASS: AreaId        = AreaId(30);
pub const YRONWOOD: AreaId            = AreaId(31);
pub const STARFALL: AreaId            = AreaId(32);
pub const SALT_SHORE: AreaId          = AreaId(33);
pub const SUNSPEAR: AreaId            = AreaId(34);
// LANDS — Islands
pub const PYKE: AreaId                = AreaId(35);
pub const DRAGONSTONE: AreaId         = AreaId(36);
pub const THE_ARBOR: AreaId           = AreaId(37);
// SEAS
pub const BAY_OF_ICE: AreaId          = AreaId(38);
pub const THE_SHIVERING_SEA: AreaId   = AreaId(39);
pub const SUNSET_SEA: AreaId          = AreaId(40);
pub const IRONMANS_BAY: AreaId        = AreaId(41);
pub const THE_GOLDEN_SOUND: AreaId    = AreaId(42);
pub const THE_NARROW_SEA: AreaId      = AreaId(43);
pub const BLACKWATER_BAY: AreaId      = AreaId(44);
pub const SHIPBREAKER_BAY: AreaId     = AreaId(45);
pub const REDWYNE_STRAITS: AreaId     = AreaId(46);
pub const WEST_SUMMER_SEA: AreaId     = AreaId(47);
pub const EAST_SUMMER_SEA: AreaId     = AreaId(48);
pub const SEA_OF_DORNE: AreaId        = AreaId(49);
// PORTS
pub const WINTERFELL_PORT: AreaId     = AreaId(50);
pub const WHITE_HARBOR_PORT: AreaId   = AreaId(51);
pub const PYKE_PORT: AreaId           = AreaId(52);
pub const LANNISPORT_PORT: AreaId     = AreaId(53);
pub const DRAGONSTONE_PORT: AreaId    = AreaId(54);
pub const STORMS_END_PORT: AreaId     = AreaId(55);
pub const HIGHGARDEN_PORT: AreaId     = AreaId(56);
pub const OLDTOWN_PORT: AreaId        = AreaId(57);
pub const SUNSPEAR_PORT: AreaId       = AreaId(58);

pub const NUM_AREAS: usize = 59;

/// Lookup area name by AreaId.
pub fn area_name(id: AreaId) -> &'static str {
    AREAS[id.0 as usize].name
}

// ── Static area definitions ────────────────────────────────────────────

macro_rules! land {
    ($name:expr, $id:expr, castle: $c:expr, stronghold: $s:expr, supply: $su:expr, power: $p:expr, adj: [$($a:expr),*]) => {
        AreaDef {
            id: $id, name: $name, area_type: AreaType::Land,
            castle: $c, stronghold: $s, supply_icons: $su, power_icons: $p,
            adjacent: &[$($a),*], connected_land: None, connected_sea: None,
        }
    };
}

macro_rules! sea {
    ($name:expr, $id:expr, adj: [$($a:expr),*]) => {
        AreaDef {
            id: $id, name: $name, area_type: AreaType::Sea,
            castle: false, stronghold: false, supply_icons: 0, power_icons: 0,
            adjacent: &[$($a),*], connected_land: None, connected_sea: None,
        }
    };
}

macro_rules! port {
    ($name:expr, $id:expr, land: $land:expr, sea: $sea:expr) => {
        AreaDef {
            id: $id, name: $name, area_type: AreaType::Port,
            castle: false, stronghold: false, supply_icons: 0, power_icons: 0,
            adjacent: &[$land, $sea], connected_land: Some($land), connected_sea: Some($sea),
        }
    };
}

pub static AREAS: [AreaDef; NUM_AREAS] = [
    // 0: Castle Black
    land!("Castle Black", CASTLE_BLACK, castle: false, stronghold: false, supply: 0, power: 1,
        adj: [WINTERFELL, KARHOLD, BAY_OF_ICE, THE_SHIVERING_SEA]),
    // 1: Karhold
    land!("Karhold", KARHOLD, castle: false, stronghold: false, supply: 0, power: 1,
        adj: [CASTLE_BLACK, WINTERFELL, THE_SHIVERING_SEA]),
    // 2: The Stony Shore
    land!("The Stony Shore", THE_STONY_SHORE, castle: false, stronghold: false, supply: 1, power: 0,
        adj: [WINTERFELL, BAY_OF_ICE]),
    // 3: Winterfell
    land!("Winterfell", WINTERFELL, castle: false, stronghold: true, supply: 1, power: 1,
        adj: [CASTLE_BLACK, KARHOLD, THE_STONY_SHORE, WHITE_HARBOR, MOAT_CAILIN, BAY_OF_ICE, THE_SHIVERING_SEA]),
    // 4: White Harbor
    land!("White Harbor", WHITE_HARBOR, castle: true, stronghold: false, supply: 0, power: 0,
        adj: [WINTERFELL, MOAT_CAILIN, WIDOWS_WATCH, THE_NARROW_SEA, THE_SHIVERING_SEA]),
    // 5: Widow's Watch
    land!("Widow's Watch", WIDOWS_WATCH, castle: false, stronghold: false, supply: 1, power: 0,
        adj: [WHITE_HARBOR, THE_NARROW_SEA, THE_SHIVERING_SEA]),
    // 6: Moat Cailin
    land!("Moat Cailin", MOAT_CAILIN, castle: true, stronghold: false, supply: 0, power: 0,
        adj: [WINTERFELL, WHITE_HARBOR, GREYWATER_WATCH, SEAGARD, THE_TWINS, THE_NARROW_SEA]),
    // 7: Greywater Watch
    land!("Greywater Watch", GREYWATER_WATCH, castle: false, stronghold: false, supply: 1, power: 0,
        adj: [MOAT_CAILIN, SEAGARD, FLINTS_FINGER, BAY_OF_ICE, IRONMANS_BAY]),
    // 8: Flint's Finger
    land!("Flint's Finger", FLINTS_FINGER, castle: true, stronghold: false, supply: 0, power: 0,
        adj: [GREYWATER_WATCH, BAY_OF_ICE, IRONMANS_BAY, SUNSET_SEA]),
    // 9: Seagard
    land!("Seagard", SEAGARD, castle: false, stronghold: true, supply: 1, power: 1,
        adj: [MOAT_CAILIN, GREYWATER_WATCH, THE_TWINS, RIVERRUN, IRONMANS_BAY]),
    // 10: The Twins
    land!("The Twins", THE_TWINS, castle: false, stronghold: false, supply: 0, power: 1,
        adj: [MOAT_CAILIN, SEAGARD, THE_FINGERS, MOUNTAINS_OF_THE_MOON, THE_NARROW_SEA]),
    // 11: The Fingers
    land!("The Fingers", THE_FINGERS, castle: false, stronghold: false, supply: 1, power: 0,
        adj: [THE_TWINS, MOUNTAINS_OF_THE_MOON, THE_NARROW_SEA]),
    // 12: Mountains of the Moon
    land!("The Mountains of the Moon", MOUNTAINS_OF_THE_MOON, castle: false, stronghold: false, supply: 1, power: 0,
        adj: [THE_TWINS, THE_FINGERS, THE_EYRIE, CRACKCLAW_POINT, THE_NARROW_SEA]),
    // 13: The Eyrie
    land!("The Eyrie", THE_EYRIE, castle: true, stronghold: false, supply: 1, power: 1,
        adj: [MOUNTAINS_OF_THE_MOON, THE_NARROW_SEA]),
    // 14: Riverrun
    land!("Riverrun", RIVERRUN, castle: false, stronghold: true, supply: 1, power: 1,
        adj: [SEAGARD, LANNISPORT, STONEY_SEPT, HARRENHAL, IRONMANS_BAY, THE_GOLDEN_SOUND]),
    // 15: Lannisport
    land!("Lannisport", LANNISPORT, castle: false, stronghold: true, supply: 2, power: 0,
        adj: [RIVERRUN, STONEY_SEPT, SEAROAD_MARCHES, THE_GOLDEN_SOUND]),
    // 16: Stoney Sept
    land!("Stoney Sept", STONEY_SEPT, castle: false, stronghold: false, supply: 0, power: 1,
        adj: [RIVERRUN, LANNISPORT, HARRENHAL, SEAROAD_MARCHES, BLACKWATER]),
    // 17: Searoad Marches
    land!("Searoad Marches", SEAROAD_MARCHES, castle: false, stronghold: false, supply: 1, power: 0,
        adj: [LANNISPORT, STONEY_SEPT, HIGHGARDEN, BLACKWATER, THE_REACH, SUNSET_SEA, THE_GOLDEN_SOUND, WEST_SUMMER_SEA]),
    // 18: Harrenhal
    land!("Harrenhal", HARRENHAL, castle: true, stronghold: false, supply: 0, power: 1,
        adj: [RIVERRUN, STONEY_SEPT, CRACKCLAW_POINT, KINGS_LANDING]),
    // 19: Crackclaw Point
    land!("Crackclaw Point", CRACKCLAW_POINT, castle: true, stronghold: false, supply: 0, power: 0,
        adj: [HARRENHAL, KINGS_LANDING, MOUNTAINS_OF_THE_MOON, BLACKWATER_BAY, SHIPBREAKER_BAY, THE_NARROW_SEA]),
    // 20: King's Landing
    land!("King's Landing", KINGS_LANDING, castle: false, stronghold: true, supply: 0, power: 2,
        adj: [HARRENHAL, CRACKCLAW_POINT, BLACKWATER, KINGSWOOD, THE_REACH, BLACKWATER_BAY]),
    // 21: Blackwater
    land!("Blackwater", BLACKWATER, castle: false, stronghold: false, supply: 2, power: 0,
        adj: [KINGS_LANDING, STONEY_SEPT, SEAROAD_MARCHES, CRACKCLAW_POINT, THE_REACH, KINGSWOOD, THE_BONEWAY, DORNISH_MARCHES]),
    // 22: Kingswood
    land!("Kingswood", KINGSWOOD, castle: false, stronghold: false, supply: 1, power: 1,
        adj: [KINGS_LANDING, BLACKWATER, STORMS_END, THE_BONEWAY, THE_REACH, BLACKWATER_BAY, SHIPBREAKER_BAY]),
    // 23: Storm's End
    land!("Storm's End", STORMS_END, castle: true, stronghold: false, supply: 0, power: 0,
        adj: [KINGSWOOD, THE_BONEWAY, EAST_SUMMER_SEA, SEA_OF_DORNE, SHIPBREAKER_BAY]),
    // 24: Highgarden
    land!("Highgarden", HIGHGARDEN, castle: false, stronghold: true, supply: 2, power: 0,
        adj: [SEAROAD_MARCHES, THE_REACH, DORNISH_MARCHES, OLDTOWN, REDWYNE_STRAITS, WEST_SUMMER_SEA]),
    // 25: The Reach
    land!("The Reach", THE_REACH, castle: true, stronghold: false, supply: 0, power: 0,
        adj: [HIGHGARDEN, SEAROAD_MARCHES, BLACKWATER, KINGS_LANDING, KINGSWOOD, DORNISH_MARCHES, THE_BONEWAY, OLDTOWN]),
    // 26: Dornish Marches
    land!("Dornish Marches", DORNISH_MARCHES, castle: false, stronghold: false, supply: 0, power: 1,
        adj: [HIGHGARDEN, THE_REACH, BLACKWATER, THE_BONEWAY, PRINCES_PASS, OLDTOWN, THREE_TOWERS]),
    // 27: Oldtown
    land!("Oldtown", OLDTOWN, castle: false, stronghold: true, supply: 0, power: 0,
        adj: [HIGHGARDEN, THE_REACH, DORNISH_MARCHES, THREE_TOWERS, REDWYNE_STRAITS]),
    // 28: Three Towers
    land!("Three Towers", THREE_TOWERS, castle: false, stronghold: false, supply: 1, power: 0,
        adj: [OLDTOWN, DORNISH_MARCHES, PRINCES_PASS, REDWYNE_STRAITS, WEST_SUMMER_SEA]),
    // 29: The Boneway
    land!("The Boneway", THE_BONEWAY, castle: false, stronghold: false, supply: 0, power: 1,
        adj: [DORNISH_MARCHES, PRINCES_PASS, THE_REACH, KINGSWOOD, BLACKWATER, STORMS_END, YRONWOOD, SEA_OF_DORNE]),
    // 30: Prince's Pass
    land!("Prince's Pass", PRINCES_PASS, castle: false, stronghold: false, supply: 1, power: 1,
        adj: [DORNISH_MARCHES, THE_BONEWAY, THREE_TOWERS, STARFALL, YRONWOOD]),
    // 31: Yronwood
    land!("Yronwood", YRONWOOD, castle: true, stronghold: false, supply: 0, power: 0,
        adj: [PRINCES_PASS, THE_BONEWAY, STARFALL, SALT_SHORE, SUNSPEAR, SEA_OF_DORNE]),
    // 32: Starfall
    land!("Starfall", STARFALL, castle: true, stronghold: false, supply: 1, power: 0,
        adj: [PRINCES_PASS, YRONWOOD, SALT_SHORE, EAST_SUMMER_SEA, WEST_SUMMER_SEA]),
    // 33: Salt Shore
    land!("Salt Shore", SALT_SHORE, castle: false, stronghold: false, supply: 1, power: 0,
        adj: [YRONWOOD, STARFALL, SUNSPEAR, EAST_SUMMER_SEA]),
    // 34: Sunspear
    land!("Sunspear", SUNSPEAR, castle: false, stronghold: true, supply: 1, power: 1,
        adj: [YRONWOOD, SALT_SHORE, EAST_SUMMER_SEA, SEA_OF_DORNE]),
    // 35: Pyke
    land!("Pyke", PYKE, castle: false, stronghold: true, supply: 1, power: 1,
        adj: [IRONMANS_BAY]),
    // 36: Dragonstone
    land!("Dragonstone", DRAGONSTONE, castle: false, stronghold: true, supply: 1, power: 1,
        adj: [SHIPBREAKER_BAY]),
    // 37: The Arbor
    land!("The Arbor", THE_ARBOR, castle: false, stronghold: false, supply: 0, power: 1,
        adj: [REDWYNE_STRAITS, WEST_SUMMER_SEA]),

    // ═══ SEAS ═══

    // 38: Bay of Ice
    sea!("Bay of Ice", BAY_OF_ICE,
        adj: [CASTLE_BLACK, THE_STONY_SHORE, WINTERFELL, FLINTS_FINGER, GREYWATER_WATCH, SUNSET_SEA]),
    // 39: The Shivering Sea
    sea!("The Shivering Sea", THE_SHIVERING_SEA,
        adj: [CASTLE_BLACK, KARHOLD, WINTERFELL, WHITE_HARBOR, WIDOWS_WATCH, THE_NARROW_SEA]),
    // 40: Sunset Sea
    sea!("Sunset Sea", SUNSET_SEA,
        adj: [FLINTS_FINGER, SEAROAD_MARCHES, BAY_OF_ICE, IRONMANS_BAY, THE_GOLDEN_SOUND, WEST_SUMMER_SEA]),
    // 41: Ironman's Bay
    sea!("Ironman's Bay", IRONMANS_BAY,
        adj: [PYKE, FLINTS_FINGER, GREYWATER_WATCH, SEAGARD, RIVERRUN, SUNSET_SEA, THE_GOLDEN_SOUND]),
    // 42: The Golden Sound
    sea!("The Golden Sound", THE_GOLDEN_SOUND,
        adj: [LANNISPORT, RIVERRUN, SEAROAD_MARCHES, IRONMANS_BAY, SUNSET_SEA]),
    // 43: The Narrow Sea
    sea!("The Narrow Sea", THE_NARROW_SEA,
        adj: [MOAT_CAILIN, WHITE_HARBOR, WIDOWS_WATCH, THE_TWINS, THE_FINGERS, MOUNTAINS_OF_THE_MOON, THE_EYRIE, CRACKCLAW_POINT, THE_SHIVERING_SEA, SHIPBREAKER_BAY]),
    // 44: Blackwater Bay
    sea!("Blackwater Bay", BLACKWATER_BAY,
        adj: [KINGS_LANDING, CRACKCLAW_POINT, KINGSWOOD, SHIPBREAKER_BAY]),
    // 45: Shipbreaker Bay
    sea!("Shipbreaker Bay", SHIPBREAKER_BAY,
        adj: [DRAGONSTONE, CRACKCLAW_POINT, KINGSWOOD, STORMS_END, THE_NARROW_SEA, BLACKWATER_BAY, EAST_SUMMER_SEA]),
    // 46: Redwyne Straits
    sea!("Redwyne Straits", REDWYNE_STRAITS,
        adj: [HIGHGARDEN, OLDTOWN, THE_ARBOR, THREE_TOWERS, WEST_SUMMER_SEA]),
    // 47: West Summer Sea
    sea!("West Summer Sea", WEST_SUMMER_SEA,
        adj: [HIGHGARDEN, SEAROAD_MARCHES, THREE_TOWERS, THE_ARBOR, STARFALL, SUNSET_SEA, REDWYNE_STRAITS, EAST_SUMMER_SEA]),
    // 48: East Summer Sea
    sea!("East Summer Sea", EAST_SUMMER_SEA,
        adj: [SUNSPEAR, SALT_SHORE, STARFALL, STORMS_END, WEST_SUMMER_SEA, SEA_OF_DORNE, SHIPBREAKER_BAY]),
    // 49: Sea of Dorne
    sea!("Sea of Dorne", SEA_OF_DORNE,
        adj: [SUNSPEAR, YRONWOOD, STORMS_END, THE_BONEWAY, EAST_SUMMER_SEA]),

    // ═══ PORTS ═══

    // 50: Winterfell Port
    port!("Winterfell Port", WINTERFELL_PORT, land: WINTERFELL, sea: BAY_OF_ICE),
    // 51: White Harbor Port
    port!("White Harbor Port", WHITE_HARBOR_PORT, land: WHITE_HARBOR, sea: THE_NARROW_SEA),
    // 52: Pyke Port
    port!("Pyke Port", PYKE_PORT, land: PYKE, sea: IRONMANS_BAY),
    // 53: Lannisport Port
    port!("Lannisport Port", LANNISPORT_PORT, land: LANNISPORT, sea: THE_GOLDEN_SOUND),
    // 54: Dragonstone Port
    port!("Dragonstone Port", DRAGONSTONE_PORT, land: DRAGONSTONE, sea: SHIPBREAKER_BAY),
    // 55: Storm's End Port
    port!("Storm's End Port", STORMS_END_PORT, land: STORMS_END, sea: SHIPBREAKER_BAY),
    // 56: Highgarden Port
    port!("Highgarden Port", HIGHGARDEN_PORT, land: HIGHGARDEN, sea: REDWYNE_STRAITS),
    // 57: Oldtown Port
    port!("Oldtown Port", OLDTOWN_PORT, land: OLDTOWN, sea: REDWYNE_STRAITS),
    // 58: Sunspear Port
    port!("Sunspear Port", SUNSPEAR_PORT, land: SUNSPEAR, sea: EAST_SUMMER_SEA),
];

/// Initial garrison strengths. These belong to the home house once placed.
pub fn initial_garrison_strength(area: AreaId) -> Option<u8> {
    match area {
        KINGS_LANDING => Some(5),
        THE_EYRIE     => Some(6),
        DRAGONSTONE   => Some(2),
        WINTERFELL    => Some(2),
        LANNISPORT    => Some(2),
        HIGHGARDEN    => Some(2),
        SUNSPEAR      => Some(2),
        PYKE          => Some(2),
        _ => None,
    }
}

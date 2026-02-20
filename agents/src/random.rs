// ═══════════════════════════════════════════════════════════════════════
// Random Agent — makes all decisions randomly.
// Serves as baseline and for testing game engine stability.
// ═══════════════════════════════════════════════════════════════════════

use crate::agent::Agent;
use got_engine::types::*;
use got_engine::engine::MusterAction2;
use got_engine::visibility::PlayerView;
use rand::Rng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;
use rand::SeedableRng;

pub struct RandomAgent {
    house: HouseName,
    rng: ChaCha8Rng,
}

impl RandomAgent {
    pub fn new(house: HouseName, seed: u64) -> Self {
        RandomAgent {
            house,
            rng: ChaCha8Rng::seed_from_u64(seed),
        }
    }
}

impl Agent for RandomAgent {
    fn name(&self) -> &str { "Random" }
    fn house(&self) -> HouseName { self.house }

    fn place_orders(&mut self, view: &PlayerView) -> Vec<(AreaId, u8)> {
        let mut orders = Vec::new();
        let mut used_tokens: Vec<u8> = Vec::new();

        for area_view in &view.areas {
            if area_view.house == Some(self.house) && !area_view.units.is_empty() {
                // Pick a random unused order token
                let available: Vec<u8> = (0..15u8)
                    .filter(|t| !used_tokens.contains(t))
                    .filter(|&t| {
                        let def = got_engine::types::ORDER_TOKENS[t as usize];
                        // Respect order restrictions
                        !view.order_restrictions.contains(&def.order_type)
                    })
                    .collect();

                if let Some(&token) = available.choose(&mut self.rng) {
                    orders.push((area_view.id, token));
                    used_tokens.push(token);
                }
            }
        }
        orders
    }

    fn choose_raid(&mut self, _view: &PlayerView, _from: AreaId, targets: &[AreaId]) -> Option<AreaId> {
        if targets.is_empty() {
            None
        } else {
            targets.choose(&mut self.rng).copied()
        }
    }

    fn choose_march(&mut self, view: &PlayerView, from: AreaId, destinations: &[AreaId]) -> (AreaId, Vec<usize>) {
        if destinations.is_empty() {
            // Can't move anywhere — stay (will trigger MarchSkip path)
            return (from, vec![]);
        }
        let &to = destinations.choose(&mut self.rng).unwrap();
        let unit_count = view.areas.iter()
            .find(|a| a.id == from)
            .map_or(0, |a| a.units.len());
        // Move all units
        let unit_indices: Vec<usize> = (0..unit_count).collect();
        (to, unit_indices)
    }

    fn leave_power_token(&mut self, view: &PlayerView, _area: AreaId) -> bool {
        let my_power = view.house_info.get(&self.house).map_or(0, |h| h.power);
        my_power > 0 && self.rng.gen_bool(0.5)
    }

    fn declare_support(&mut self, _view: &PlayerView, _attacker: HouseName, _defender: HouseName) -> SupportChoice {
        // Random: support attacker, defender, or neither
        match self.rng.gen_range(0..3) {
            0 => SupportChoice::Attacker,
            1 => SupportChoice::Defender,
            _ => SupportChoice::None,
        }
    }

    fn select_house_card(&mut self, _view: &PlayerView, available: &[HouseCardId]) -> HouseCardId {
        *available.choose(&mut self.rng).expect("No cards available")
    }

    fn use_valyrian_blade(&mut self, _view: &PlayerView) -> bool {
        self.rng.gen_bool(0.5)
    }

    fn submit_bid(&mut self, view: &PlayerView, _bid_type: BiddingType, _track: Option<Track>) -> u8 {
        let my_power = view.house_info.get(&self.house).map_or(0, |h| h.power);
        self.rng.gen_range(0..=my_power)
    }

    fn westeros_choice(&mut self, _view: &PlayerView, options: &[String]) -> usize {
        self.rng.gen_range(0..options.len())
    }

    fn choose_muster(&mut self, _view: &PlayerView, areas: &[MusterArea]) -> Vec<(AreaId, MusterAction2)> {
        // Simple: try to build a footman in each area
        let mut actions = Vec::new();
        for muster_area in areas {
            if muster_area.points >= 1 {
                actions.push((muster_area.area_id, MusterAction2::Build(UnitType::Footman)));
            }
        }
        actions
    }

    fn choose_retreat(&mut self, _view: &PlayerView, options: &[AreaId]) -> AreaId {
        *options.choose(&mut self.rng).expect("No retreat options")
    }

    fn choose_reconcile(&mut self, _view: &PlayerView, area: AreaId) -> (AreaId, usize) {
        // Remove first unit in the area
        (area, 0)
    }

    fn use_messenger_raven(&mut self, _view: &PlayerView) -> Option<(AreaId, u8)> {
        None // Random agent doesn't bother swapping
    }

    fn use_aeron(&mut self, _view: &PlayerView) -> Option<HouseCardId> {
        None // Don't swap
    }

    fn tyrion_replacement(&mut self, view: &PlayerView) -> HouseCardId {
        // Pick another random card from hand
        *view.my_hand.choose(&mut self.rng).expect("No cards in hand")
    }

    fn patchface_discard(&mut self, _view: &PlayerView, visible: &[HouseCardId]) -> HouseCardId {
        *visible.choose(&mut self.rng).expect("No visible cards")
    }

    fn robb_retreat(&mut self, _view: &PlayerView, options: &[AreaId]) -> AreaId {
        *options.choose(&mut self.rng).expect("No retreat options")
    }

    fn wildling_penalty(&mut self, _view: &PlayerView, options: &[String]) -> usize {
        self.rng.gen_range(0..options.len())
    }

    fn cersei_remove_order(&mut self, view: &PlayerView) -> AreaId {
        // Pick a random opponent area with an order
        let targets: Vec<AreaId> = view.areas.iter()
            .filter(|a| a.house.is_some() && a.house != Some(self.house) && a.order.is_some())
            .map(|a| a.id)
            .collect();
        *targets.choose(&mut self.rng).unwrap_or(&AreaId(0))
    }

    fn doran_choose_track(&mut self, _view: &PlayerView) -> Track {
        match self.rng.gen_range(0..3) {
            0 => Track::IronThrone,
            1 => Track::Fiefdoms,
            _ => Track::KingsCourt,
        }
    }

    fn queen_of_thorns(&mut self, view: &PlayerView) -> AreaId {
        self.cersei_remove_order(view) // Same logic: random opponent order
    }
}

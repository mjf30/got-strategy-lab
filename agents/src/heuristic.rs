// ═══════════════════════════════════════════════════════════════════════
// Heuristic Agent — makes decisions using simple strategic heuristics.
// Significantly stronger than RandomAgent.
// ═══════════════════════════════════════════════════════════════════════

use crate::agent::Agent;
use got_engine::types::*;
use got_engine::engine::MusterAction2;
use got_engine::map::AREAS;
use got_engine::visibility::{PlayerView, AreaView};
use got_engine::cards;
use rand::Rng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;
use rand::SeedableRng;

pub struct HeuristicAgent {
    house: HouseName,
    rng: ChaCha8Rng,
}

#[allow(dead_code)]
impl HeuristicAgent {
    pub fn new(house: HouseName, seed: u64) -> Self {
        HeuristicAgent {
            house,
            rng: ChaCha8Rng::seed_from_u64(seed),
        }
    }

    /// Count castles/strongholds controlled by a house.
    fn count_castles(&self, view: &PlayerView, house: HouseName) -> u8 {
        view.areas.iter().enumerate()
            .filter(|(i, a)| a.house == Some(house) && AREAS[*i].has_castle_or_stronghold())
            .count() as u8
    }

    /// Count total units on the board for a house.
    fn count_units(&self, view: &PlayerView, house: HouseName) -> usize {
        view.areas.iter()
            .flat_map(|a| a.units.iter())
            .filter(|u| u.house == house)
            .count()
    }

    /// Find areas owned by a house that have a castle/stronghold.
    fn my_castle_areas(&self, view: &PlayerView) -> Vec<AreaId> {
        view.areas.iter().enumerate()
            .filter(|(i, a)| a.house == Some(self.house) && AREAS[*i].has_castle_or_stronghold())
            .map(|(_, a)| a.id)
            .collect()
    }

    /// Find areas owned by enemy that have a castle/stronghold.
    fn enemy_castle_areas(&self, view: &PlayerView) -> Vec<AreaId> {
        view.areas.iter().enumerate()
            .filter(|(i, a)| a.house.is_some() && a.house != Some(self.house) && AREAS[*i].has_castle_or_stronghold())
            .map(|(_, a)| a.id)
            .collect()
    }

    /// Check if an area has enemy neighbors.
    fn has_enemy_neighbor(&self, view: &PlayerView, area_id: AreaId) -> bool {
        let def = &AREAS[area_id.0 as usize];
        def.adjacent.iter().any(|&adj| {
            let a = &view.areas[adj.0 as usize];
            a.units.iter().any(|u| u.house != self.house)
        })
    }

    /// Get combat strength of units in an area.
    fn area_strength(&self, view: &PlayerView, area_id: AreaId) -> i16 {
        view.areas[area_id.0 as usize].units.iter()
            .filter(|u| u.house == self.house)
            .map(|u| match u.unit_type {
                UnitType::Footman => 1,
                UnitType::Knight => 2,
                UnitType::SiegeEngine => 4,
                UnitType::Ship => 1,
            })
            .sum()
    }
}

impl Agent for HeuristicAgent {
    fn name(&self) -> &str { "Heuristic" }
    fn house(&self) -> HouseName { self.house }

    fn place_orders(&mut self, view: &PlayerView) -> Vec<(AreaId, u8)> {
        let mut orders = Vec::new();
        let mut used_tokens: Vec<u8> = Vec::new();

        // Categorize areas
        let my_areas: Vec<&AreaView> = view.areas.iter()
            .filter(|a| a.house == Some(self.house) && !a.units.is_empty())
            .collect();

        for area_view in &my_areas {
            let has_threat = self.has_enemy_neighbor(view, area_view.id);
            let is_castle = {
                let idx = area_view.id.0 as usize;
                AREAS[idx].has_castle_or_stronghold()
            };
            let unit_str: i16 = area_view.units.iter()
                .filter(|u| u.house == self.house)
                .map(|u| match u.unit_type {
                    UnitType::Footman => 1, UnitType::Knight => 2,
                    UnitType::SiegeEngine => 4, UnitType::Ship => 1,
                })
                .sum();

            // Strategy:
            // - Castle with enemies nearby → Defense (star if available)
            // - Strong army → March (prefer +1 star)
            // - Weak or no threat → Consolidate Power
            // - Sea areas → Support

            let is_sea = AREAS[area_view.id.0 as usize].is_sea();

            let preferred_type = if is_sea {
                OrderType::Support
            } else if is_castle && has_threat && unit_str <= 2 {
                OrderType::Defense
            } else if unit_str >= 2 && !is_castle {
                OrderType::March
            } else if is_castle && !has_threat {
                OrderType::ConsolidatePower
            } else if has_threat {
                OrderType::Defense
            } else {
                // Alternate between march and consolidate
                if self.rng.gen_bool(0.6) { OrderType::March } else { OrderType::ConsolidatePower }
            };

            // Find the best available token of the preferred type
            let token = self.find_best_token(&used_tokens, preferred_type, &view.order_restrictions);

            if let Some(t) = token {
                orders.push((area_view.id, t));
                used_tokens.push(t);
            } else {
                // Fallback: any available token
                let fallback: Vec<u8> = (0..15u8)
                    .filter(|t| !used_tokens.contains(t))
                    .filter(|&t| !view.order_restrictions.contains(&ORDER_TOKENS[t as usize].order_type))
                    .collect();
                if let Some(&t) = fallback.choose(&mut self.rng) {
                    orders.push((area_view.id, t));
                    used_tokens.push(t);
                }
            }
        }
        orders
    }

    fn choose_raid(&mut self, _view: &PlayerView, _from: AreaId, targets: &[AreaId]) -> Option<AreaId> {
        if targets.is_empty() {
            return None;
        }
        // Prefer raiding consolidate power orders (deny enemy resources)
        // For now, just raid a random target
        targets.choose(&mut self.rng).copied()
    }

    fn choose_march(&mut self, view: &PlayerView, from: AreaId, destinations: &[AreaId]) -> (AreaId, Vec<usize>) {
        if destinations.is_empty() {
            return (from, vec![]);
        }

        let unit_count = view.areas.iter()
            .find(|a| a.id == from)
            .map_or(0, |a| a.units.len());

        // Prefer:
        // 1. Unoccupied castles/strongholds
        // 2. Weakly defended enemy castles
        // 3. Unoccupied supply areas
        // 4. Any unoccupied area

        let mut best_dest = *destinations.choose(&mut self.rng).unwrap();
        let mut best_score = -100i32;

        for &dest in destinations {
            let def = &AREAS[dest.0 as usize];
            let area = &view.areas[dest.0 as usize];
            let mut score: i32 = 0;

            // Castle/stronghold value
            if def.has_castle_or_stronghold() {
                score += 20;
            }

            // Supply value
            score += def.supply_icons as i32 * 3;

            // Power icon value
            if def.power_icons > 0 { score += 2; }

            // Unoccupied bonus
            if area.house.is_none() || area.house == Some(self.house) {
                score += 10;
            }

            // Avoid attacking strong enemies unless we're stronger
            if area.house.is_some() && area.house != Some(self.house) {
                let enemy_str: i16 = area.units.iter().map(|u| match u.unit_type {
                    UnitType::Footman => 1, UnitType::Knight => 2,
                    UnitType::SiegeEngine => 4, UnitType::Ship => 1,
                }).sum();
                let my_str = self.area_strength(view, from);
                if my_str > enemy_str + 1 {
                    score += 5; // We can likely win
                } else {
                    score -= 15; // Risky attack
                }
            }

            // Small random factor
            score += self.rng.gen_range(0..5);

            if score > best_score {
                best_score = score;
                best_dest = dest;
            }
        }

        // Move all units
        let unit_indices: Vec<usize> = (0..unit_count).collect();
        (best_dest, unit_indices)
    }

    fn leave_power_token(&mut self, view: &PlayerView, _area: AreaId) -> bool {
        // Leave if we have enough power
        let my_power = view.house_info.get(&self.house).map_or(0, |h| h.power);
        my_power >= 3
    }

    fn declare_support(&mut self, _view: &PlayerView, attacker: HouseName, defender: HouseName) -> SupportChoice {
        // Support ourselves if we're involved
        if attacker == self.house { return SupportChoice::Attacker; }
        if defender == self.house { return SupportChoice::Defender; }
        // Otherwise, support the weaker or don't support
        SupportChoice::None
    }

    fn select_house_card(&mut self, view: &PlayerView, available: &[HouseCardId]) -> HouseCardId {
        if available.len() == 1 {
            return available[0];
        }

        // Score each card based on situation
        let in_combat = view.combat.as_ref();
        let am_attacker = in_combat.is_some_and(|c| c.attacker == self.house);

        let mut best_card = available[0];
        let mut best_score = -100i32;

        for &card_id in available {
            let card = cards::get_house_card(card_id);
            let mut score = card.strength as i32 * 3;
            score += card.swords as i32 * 2;
            score += card.fortifications as i32 * 2;

            // Prefer high-strength cards when we have few left
            if available.len() <= 3 {
                score += card.strength as i32;
            }

            // Save strong cards for later if we have many
            if available.len() >= 5 && card.strength >= 3 {
                score -= 5;
            }

            // Prefer attacking-bonus cards when attacking
            if am_attacker {
                match card_id {
                    HouseCardId::SerJaimeLannister | HouseCardId::GreatjonUmber => score += 4,
                    _ => {}
                }
            }

            score += self.rng.gen_range(0..3);

            if score > best_score {
                best_score = score;
                best_card = card_id;
            }
        }

        best_card
    }

    fn use_valyrian_blade(&mut self, _view: &PlayerView) -> bool {
        // Always use the blade in combat — it's free
        true
    }

    fn submit_bid(&mut self, view: &PlayerView, bid_type: BiddingType, _track: Option<Track>) -> u8 {
        let my_power = view.house_info.get(&self.house).map_or(0, |h| h.power);

        match bid_type {
            BiddingType::Wildling => {
                let threat = view.wildling_threat;
                if threat >= 10 { my_power.min(5) }
                else if threat >= 6 { my_power.min(3) }
                else { my_power.min(1) }
            }
            BiddingType::IronThrone => my_power.min(4),
            BiddingType::Fiefdoms => my_power.min(3),
            BiddingType::KingsCourt => my_power.min(2),
        }
    }

    fn westeros_choice(&mut self, _view: &PlayerView, _options: &[String]) -> usize {
        // Choose first option (usually more conservative)
        0
    }

    fn choose_muster(&mut self, view: &PlayerView, areas: &[MusterArea]) -> Vec<(AreaId, MusterAction2)> {
        let mut actions = Vec::new();
        let avail = view.house_info.get(&self.house)
            .map_or(UnitPool { footmen: 0, knights: 0, siege_engines: 0, ships: 0 }, |h| h.available_units);

        let mut remaining_knights = avail.knights;
        let mut remaining_footmen = avail.footmen;
        let mut remaining_siege = avail.siege_engines;

        for muster_area in areas {
            let pts = muster_area.points;
            let is_land = AREAS[muster_area.area_id.0 as usize].is_land();

            if !is_land { continue; } // Skip sea areas

            if pts >= 2 && remaining_knights > 0 {
                // Build knight (costs 2 points)
                actions.push((muster_area.area_id, MusterAction2::Build(UnitType::Knight)));
                remaining_knights -= 1;
            } else if pts >= 2 && remaining_siege > 0 {
                actions.push((muster_area.area_id, MusterAction2::Build(UnitType::SiegeEngine)));
                remaining_siege -= 1;
            } else if pts >= 1 && remaining_footmen > 0 {
                actions.push((muster_area.area_id, MusterAction2::Build(UnitType::Footman)));
                remaining_footmen -= 1;
            }
        }
        actions
    }

    fn choose_retreat(&mut self, _view: &PlayerView, options: &[AreaId]) -> AreaId {
        // Prefer retreating to a castle/stronghold
        for &opt in options {
            if AREAS[opt.0 as usize].has_castle_or_stronghold() {
                return opt;
            }
        }
        *options.choose(&mut self.rng).expect("No retreat options")
    }

    fn choose_reconcile(&mut self, view: &PlayerView, area: AreaId) -> (AreaId, usize) {
        // Remove the weakest unit
        let units = &view.areas[area.0 as usize].units;
        let mut weakest = 0;
        let mut weakest_str = 999;
        for (i, u) in units.iter().enumerate() {
            if u.house != self.house { continue; }
            let str = match u.unit_type {
                UnitType::Footman => 1,
                UnitType::Knight => 2,
                UnitType::SiegeEngine => 4,
                UnitType::Ship => 1,
            };
            if str < weakest_str {
                weakest_str = str;
                weakest = i;
            }
        }
        (area, weakest)
    }

    fn use_messenger_raven(&mut self, _view: &PlayerView) -> Option<(AreaId, u8)> {
        None // Don't bother swapping for now
    }

    fn use_aeron(&mut self, _view: &PlayerView) -> Option<HouseCardId> {
        None // Don't swap — 2 power is expensive
    }

    fn tyrion_replacement(&mut self, view: &PlayerView) -> HouseCardId {
        // Pick lowest-strength card (save the good ones)
        let hand = &view.my_hand;
        hand.iter()
            .min_by_key(|&&c| cards::get_house_card(c).strength)
            .copied()
            .unwrap_or(hand[0])
    }

    fn patchface_discard(&mut self, _view: &PlayerView, visible: &[HouseCardId]) -> HouseCardId {
        // Discard opponent's highest-strength card
        visible.iter()
            .max_by_key(|&&c| cards::get_house_card(c).strength)
            .copied()
            .unwrap_or(visible[0])
    }

    fn robb_retreat(&mut self, _view: &PlayerView, options: &[AreaId]) -> AreaId {
        // Force enemy retreat to the worst area (fewest supply, no castle)
        options.iter()
            .min_by_key(|&&area| {
                let def = &AREAS[area.0 as usize];
                def.supply_icons as i32 * 3
                    + if def.has_castle_or_stronghold() { 10 } else { 0 }
            })
            .copied()
            .unwrap_or(options[0])
    }

    fn wildling_penalty(&mut self, _view: &PlayerView, _options: &[String]) -> usize {
        0 // Pick first option
    }

    fn cersei_remove_order(&mut self, view: &PlayerView) -> AreaId {
        // Remove march orders first (deny enemy movement), then support
        let targets: Vec<(AreaId, &Order)> = view.areas.iter()
            .filter(|a| a.house.is_some() && a.house != Some(self.house) && a.order.is_some())
            .map(|a| (a.id, a.order.as_ref().unwrap()))
            .collect();

        // Priority: March > Support > Defense > Raid > CP
        targets.iter()
            .max_by_key(|(_, o)| match o.order_type {
                OrderType::March => 5,
                OrderType::Support => 4,
                OrderType::Defense => 3,
                OrderType::Raid => 2,
                OrderType::ConsolidatePower => 1,
            })
            .map(|(id, _)| *id)
            .unwrap_or(AreaId(0))
    }

    fn doran_choose_track(&mut self, _view: &PlayerView) -> Track {
        // Move opponent down the track where they're highest (most damaging)
        // Since we don't know which opponent, just pick IronThrone (most powerful)
        Track::IronThrone
    }

    fn queen_of_thorns(&mut self, view: &PlayerView) -> AreaId {
        self.cersei_remove_order(view)
    }
}

impl HeuristicAgent {
    fn find_best_token(&mut self, used: &[u8], preferred: OrderType, restrictions: &[OrderType]) -> Option<u8> {
        // Find the best available token of the preferred type
        let mut candidates: Vec<(u8, i32)> = (0..15u8)
            .filter(|t| !used.contains(t))
            .filter(|&t| !restrictions.contains(&ORDER_TOKENS[t as usize].order_type))
            .filter(|&t| ORDER_TOKENS[t as usize].order_type == preferred)
            .map(|t| {
                let def = &ORDER_TOKENS[t as usize];
                let score = def.strength as i32 * 2 + if def.star { 1 } else { 0 };
                (t, score)
            })
            .collect();

        candidates.sort_by(|a, b| b.1.cmp(&a.1));
        candidates.first().map(|(t, _)| *t)
    }
}

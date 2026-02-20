// ═══════════════════════════════════════════════════════════════════════
// Agent Trait — interface that all AI agents must implement
//
// KEY DESIGN PRINCIPLE:
//   Agents receive a `PlayerView` (not raw GameState), which only
//   contains information the player is legally allowed to see.
//   This enforces information hiding at the type level.
//
//   The agent never gets to see:
//     - Opponent's hand of house cards
//     - Opponent's unrevealed orders
//     - Opponent's bid amounts
//     - Deck ordering (future Westeros/Wildling cards)
// ═══════════════════════════════════════════════════════════════════════

use got_engine::types::*;
use got_engine::engine::{Action, MusterAction2};
use got_engine::visibility::PlayerView;

/// Trait that all AI agents must implement.
/// Each method corresponds to a pending decision type.
/// The agent receives a PlayerView and must return an Action.
pub trait Agent: Send + Sync {
    /// Human-readable name for this agent (e.g., "Aggressive", "Turtle").
    fn name(&self) -> &str;

    /// The house this agent is playing.
    fn house(&self) -> HouseName;

    /// Make a decision based on the current player view.
    /// This is the universal entry point — dispatches to specific methods.
    fn decide(&mut self, view: &PlayerView) -> Action {
        match view.pending.as_ref().expect("No pending decision") {
            PendingDecision::PlaceOrders { .. } => {
                Action::PlaceOrders(self.place_orders(view))
            }
            PendingDecision::ChooseRaid { from_area, valid_targets, .. } => {
                Action::Raid(self.choose_raid(view, *from_area, valid_targets))
            }
            PendingDecision::ChooseMarch { from_area, valid_destinations, .. } => {
                let (to, units) = self.choose_march(view, *from_area, valid_destinations);
                Action::March { to, unit_indices: units }
            }
            PendingDecision::LeavePowerToken { area_id } => {
                Action::LeavePowerToken(self.leave_power_token(view, *area_id))
            }
            PendingDecision::SupportDeclaration { attacker, defender, .. } => {
                Action::DeclareSupport(self.declare_support(view, *attacker, *defender))
            }
            PendingDecision::SelectHouseCard { available_cards, .. } => {
                Action::SelectCard(self.select_house_card(view, available_cards))
            }
            PendingDecision::UseValyrianBlade => {
                Action::UseValyrianBlade(self.use_valyrian_blade(view))
            }
            PendingDecision::Bidding { bidding_type, track } => {
                Action::Bid(self.submit_bid(view, *bidding_type, *track))
            }
            PendingDecision::WesterosChoice { options, .. } => {
                Action::WesterosChoice(self.westeros_choice(view, options))
            }
            PendingDecision::Muster { areas, .. } => {
                Action::Muster(self.choose_muster(view, areas))
            }
            PendingDecision::Retreat { possible_areas, .. } => {
                Action::Retreat(self.choose_retreat(view, possible_areas))
            }
            PendingDecision::Reconcile { area_id, .. } => {
                let (aid, idx) = self.choose_reconcile(view, *area_id);
                Action::Reconcile(aid, idx)
            }
            PendingDecision::MessengerRaven { .. } => {
                Action::MessengerRaven(self.use_messenger_raven(view))
            }
            PendingDecision::AeronSwap { .. } => {
                Action::AeronSwap(self.use_aeron(view))
            }
            PendingDecision::TyrionReplace { .. } => {
                Action::TyrionReplace(self.tyrion_replacement(view))
            }
            PendingDecision::PatchfaceDiscard { visible_cards, .. } => {
                Action::PatchfaceDiscard(self.patchface_discard(view, visible_cards))
            }
            PendingDecision::RobbRetreat { possible_areas, .. } => {
                Action::RobbRetreat(self.robb_retreat(view, possible_areas))
            }
            PendingDecision::WildlingPenaltyChoice { options, .. } => {
                Action::WildlingPenalty(self.wildling_penalty(view, options))
            }
            PendingDecision::CerseiRemoveOrder { .. } => {
                Action::CerseiRemoveOrder(self.cersei_remove_order(view))
            }
            PendingDecision::DoranChooseTrack { .. } => {
                Action::DoranChooseTrack(self.doran_choose_track(view))
            }
            PendingDecision::QueenOfThornsRemoveOrder { .. } => {
                Action::QueenOfThorns(self.queen_of_thorns(view))
            }
        }
    }

    // ── Individual decision methods ────────────────────────────────────
    // Agents override these to implement their strategy.
    // Default implementations are provided (random/simple) so agents
    // only need to override the decisions they care about.

    /// Place orders on all areas with units. Returns Vec<(area_id, token_index)>.
    fn place_orders(&mut self, view: &PlayerView) -> Vec<(AreaId, u8)>;

    /// Choose raid target. None = skip raid.
    fn choose_raid(&mut self, view: &PlayerView, from: AreaId, targets: &[AreaId]) -> Option<AreaId>;

    /// Choose march destination + which unit indices to move.
    fn choose_march(&mut self, view: &PlayerView, from: AreaId, destinations: &[AreaId]) -> (AreaId, Vec<usize>);

    /// Whether to leave a power token when vacating an area.
    fn leave_power_token(&mut self, view: &PlayerView, area: AreaId) -> bool;

    /// Declare support in combat.
    fn declare_support(&mut self, view: &PlayerView, attacker: HouseName, defender: HouseName) -> SupportChoice;

    /// Select house card for combat.
    fn select_house_card(&mut self, view: &PlayerView, available: &[HouseCardId]) -> HouseCardId;

    /// Whether to use the Valyrian Steel Blade.
    fn use_valyrian_blade(&mut self, view: &PlayerView) -> bool;

    /// Submit bid (power tokens) for Clash of Kings or Wildling attack.
    fn submit_bid(&mut self, view: &PlayerView, bid_type: BiddingType, track: Option<Track>) -> u8;

    /// Choose option for Westeros card decision.
    fn westeros_choice(&mut self, view: &PlayerView, options: &[String]) -> usize;

    /// Choose mustering actions.
    fn choose_muster(&mut self, view: &PlayerView, areas: &[MusterArea]) -> Vec<(AreaId, MusterAction2)>;

    /// Choose retreat area.
    fn choose_retreat(&mut self, view: &PlayerView, options: &[AreaId]) -> AreaId;

    /// Choose which unit to disband for supply reconciliation.
    fn choose_reconcile(&mut self, view: &PlayerView, area: AreaId) -> (AreaId, usize);

    /// Messenger Raven: swap an order. None = don't swap.
    fn use_messenger_raven(&mut self, view: &PlayerView) -> Option<(AreaId, u8)>;

    /// Aeron Damphair: pay 2 power to swap card. None = decline.
    fn use_aeron(&mut self, view: &PlayerView) -> Option<HouseCardId>;

    /// Tyrion: choose replacement card.
    fn tyrion_replacement(&mut self, view: &PlayerView) -> HouseCardId;

    /// Patchface: choose card to discard from opponent's hand.
    fn patchface_discard(&mut self, view: &PlayerView, visible: &[HouseCardId]) -> HouseCardId;

    /// Robb Stark: choose defender's retreat area.
    fn robb_retreat(&mut self, view: &PlayerView, options: &[AreaId]) -> AreaId;

    /// Wildling penalty: choose from options.
    fn wildling_penalty(&mut self, view: &PlayerView, options: &[String]) -> usize;

    /// Cersei: choose area whose opponent order to remove.
    fn cersei_remove_order(&mut self, view: &PlayerView) -> AreaId;

    /// Doran: choose influence track to move opponent to bottom.
    fn doran_choose_track(&mut self, view: &PlayerView) -> Track;

    /// Queen of Thorns: choose area whose opponent order to remove.
    fn queen_of_thorns(&mut self, view: &PlayerView) -> AreaId;
}

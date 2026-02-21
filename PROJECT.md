# GoT Strategy Lab — Project Reference

> **Purpose**: This file is the single source of truth for the project's architecture,
> current state, known gaps, and implementation plan. Written so that any LLM session
> can pick up where the last one left off without losing context.
>
> **Last updated**: 2026-02-21 — commit `39049c2`

---

## 1. What Is This?

A headless **Game of Thrones: The Board Game (2nd Edition)** engine written in Rust,
designed for AI research. The goal is to run thousands of automated games with different
agent strategies, track results with ELO, and eventually train stronger agents.

**Repository**: `https://github.com/mjf30/got-strategy-lab.git` (branch: `master`)

---

## 2. Workspace Layout

```
got-strategy-lab/          Cargo workspace (resolver = "2")
├── engine/                got-engine     — pure game state machine (no I/O)
│   └── src/
│       ├── lib.rs         re-exports
│       ├── types.rs       (542 loc) enums, structs, GameState, PendingDecision, Action types
│       ├── map.rs         (320 loc) 56 areas (38 land, 9 sea, 9 port), adjacencies, AreaDef
│       ├── cards.rs       (136 loc) 42 house cards (6×7), 3 Westeros decks, 9 wildling cards
│       ├── engine.rs      (2320 loc) advance(), apply_action(), combat resolution, all game logic
│       ├── setup.rs       (281 loc) create_initial_state(player_count, seed)
│       ├── supply.rs      (70 loc)  supply violation checks & calculation
│       ├── navigation.rs  (75 loc)  land/sea movement via BFS transport chains
│       ├── visibility.rs  (243 loc) PlayerView (fog of war) — agents never see raw GameState
│       └── tests.rs       (699 loc) 58 tests: setup, supply, cards, combat, determinism, stress
├── agents/                got-agents    — AI agent trait + implementations
│   └── src/
│       ├── lib.rs         exports Agent, RandomAgent, HeuristicAgent
│       ├── agent.rs       (143 loc) Agent trait (one method per PendingDecision type)
│       ├── random.rs      (149 loc) fully random legal-move agent
│       └── heuristic.rs   (402 loc) scored-march, strategic-order, situational-card agent
├── tournament/            got-tournament — game runner, SQLite DB, ELO
│   └── src/
│       ├── lib.rs         re-exports
│       ├── runner.rs      (125 loc) run_game(), run_tournament()
│       └── database.rs    (155 loc) SQLite schema (agents, games, game_players), ELO updates
├── runner/                got-runner    — CLI entry point (clap)
│   └── src/
│       └── main.rs        (145 loc) play / tournament / leaderboard subcommands
└── Cargo.toml             workspace root
```

### Key dependencies

| Crate | Version | Used by | Purpose |
|-------|---------|---------|---------|
| serde + serde_json | 1 | all | GameState serialization |
| rand + rand_chacha | 0.8 / 0.3 | engine, agents | Deterministic RNG (ChaCha8Rng) |
| clap | 4 | runner | CLI parsing |
| rusqlite | 0.31 (bundled) | tournament | SQLite for game results + ELO |
| rayon | 1.8 | tournament | **NOT USED** — dead dependency, tournament runs sequentially |

---

## 3. Architecture

```
┌─────────────────────────────────────────────────────┐
│  Runner (main.rs)                                    │
│    creates agents + GameState                        │
│    loop:                                             │
│      engine::advance(&mut state)  ← pure, no I/O    │
│      if state.pending.is_some():                     │
│        view = visibility::player_view(&state, house) │
│        action = agent.decide(&view)                  │
│        engine::apply_action(&mut state, action)      │
│      if state.winner.is_some(): break                │
└─────────────────────────────────────────────────────┘
```

- **Pure state machine**: `engine.rs` never does I/O, never calls agents.
  It sets `state.pending = Some(PendingDecision::...)` and returns.
- **Deterministic**: seeded `ChaCha8Rng`. Same seed + same agent decisions = same outcome.
- **Iterative loops**: `advance()` and `advance_combat()` use iterative loops (not recursion)
  to prevent stack overflow in long games. Progress detection breaks infinite loops.
- **Visibility**: Agents receive `PlayerView` (fog of war), never raw `GameState`.
  They cannot see opponent hands, unrevealed orders, deck ordering, or bid amounts.

### Game phases (state.phase)

```
Phase::Westeros  → draw 3 cards, resolve effects (supply, mustering, CoK, wildlings)
Phase::Planning  → each house places orders, messenger raven, reveal
Phase::Action    → in turn order: Raid → March → Consolidate Power sub-phases
Phase::Combat    → triggered by March into occupied area
```

### Key types (types.rs)

- `GameState` — complete game state (~25 fields)
- `PendingDecision` — enum with ~20 variants (one per decision point)
- `Action` — enum matching PendingDecision variants (agent responses)
- `HouseName` — Stark, Lannister, Baratheon, Greyjoy, Tyrell, Martell
- `AreaId(u8)` — index into static `AREAS` array (56 areas)
- `CombatState` — attacker, defender, cards, strengths, support decisions
- `BiddingState` — tracks, bids, bid order
- `HouseCardId` — 42 card IDs across 6 houses (7 per house)

### Map constants (map.rs)

```
Lands 0–37:  CASTLE_BLACK=0, WINTERFELL=3, LANNISPORT=15, KINGS_LANDING=20,
             HIGHGARDEN=24, SUNSPEAR=34, PYKE=35, DRAGONSTONE=36, THE_ARBOR=37
Seas  38–49: BAY_OF_ICE=38, SHIVERING_SEA=39, ..., SEA_OF_DORNE=49
Ports 50–58: WINTERFELL_PORT=50, ..., SUNSPEAR_PORT=58
```

---

## 4. Current State — What Works

### ✅ Fully working

- **Game loop**: Westeros → Planning → Action → Combat, all transitions correct
- **Setup**: 3/4/5/6-player games with correct starting positions, tracks, units
- **Neutral garrisons**: King's Landing (5), The Eyrie (6), plus Dornish/Greyjoy areas for 4/5p
- **Blocked areas**: 3-player game blocks southern regions
- **Combat**: Full combat resolution with swords, fortifications, march bonus, defense bonus,
  garrison strength, siege engine bonus, Valyrian Steel Blade, support declarations
- **Bidding**: Clash of Kings (3 tracks) and Wildling Attack (cooperative)
- **Supply & reconcile**: Supply calculation, violation detection, forced reconciliation
- **Navigation**: BFS transport chains through friendly seas, march validation
- **Mustering**: Build (Footman/Knight/Ship/Siege) and upgrade (Footman→Knight) on land
- **Victory**: 7-castle instant win + round-10 tiebreaker
- **Visibility**: PlayerView fog of war hiding opponent hands, unrevealed orders, deck order
- **Determinism**: Same seed → same game. Verified across 500+ games.
- **CLI**: `cargo run -- play`, `cargo run -- tournament --games N`, `cargo run -- leaderboard`
- **SQLite + ELO**: Game results stored, multiplayer ELO tracked
- **Agents**: RandomAgent (all legal moves) + HeuristicAgent (strategic scoring)
- **Tests**: 58 tests covering setup, supply, cards, orders, combat, stress (3-6p), determinism
- **Clippy**: 0 warnings across entire workspace

### 26/42 house card abilities implemented

| House | Implemented | Stats-only (no special) |
|-------|-------------|------------------------|
| **Stark** | CatelynStark, RobbStark, GreatjonUmber, RooseBolton, TheBlackfish | EddardStark (4/2/0), SerRodrikCassel (1/0/2) |
| **Lannister** | TywinLannister, TyrionLannister, SerJaimeLannister, CerseiLannister, SerKevanLannister | SerGregorClegane (3/3/0), TheHound (2/0/2) |
| **Baratheon** | StannisBaratheon, RenlyBaratheon, BrienneOfTarth¹, SerDavosSeaworth, Melisandre, Patchface | — |
| **Greyjoy** | EuronCrowsEye¹, VictarionGreyjoy, BalonGreyjoy, TheonGreyjoy, AshaGreyjoy, AeronDamphair | DagmerCleftjaw (1/1/1) |
| **Tyrell** | MaceTyrell, SerLorasTyrell | SerGarlanTyrell (2/2/0), RandyllTarly (2/2/0), AlesterFlorent (1/0/1) |
| **Martell** | ObaraSand, NymeriaSand, ArianneMartell, DoranMartell | TheRedViper (4/2/1), Darkstar (2/1/0) |

¹ *BrienneOfTarth and EuronCrowsEye use base stats only (correct per some editions; see P1 #6).*

### 9/9 wildling cards exist (6 simplified on NW-wins side)

All 9 wildling cards have win/lose paths. Lose-side is mostly correct.
Win-side for 6 cards gives a flat +2 power or auto-action instead of the real rule.

---

## 5. Known Gaps — Prioritized

### P0 — Bugs / Rule violations (game-breaking)

| # | Issue | File:Line | Details |
|---|-------|-----------|---------|
| **P0-1** | **House card recycling missing** | engine.rs ~L982 | When hand is empty, discards should return to hand. Currently: card selection is skipped → house fights with no card for remaining game. **Bug in long games.** |
| **P0-2** | **Port destruction on conquest** | engine.rs (missing) | When a land area with a port is conquered, enemy ships in the connected port must be destroyed. **Not implemented at all.** Search for `connected_land` in engine.rs — no results. |
| **P0-3** | **Port control follows land** | engine.rs (missing) | When land changes owner, the connected port's `area.house` must update to new owner. **Not implemented.** Port ownership is never explicitly linked to land. |
| **P0-4** | **Queen of Thorns never triggers** | engine.rs | `PendingDecision::QueenOfThornsRemoveOrder` and `Action::QueenOfThorns` exist in types + apply_action, but `resolve_combat_final` never creates the pending decision. The card ability is dead code. |
| **P0-5** | **Round-10 tiebreaker scoring** | engine.rs ~L2294 `resolve_tiebreaker()` | Currently uses stronghold=2, castle=1 point weighting. Official 2nd Ed rules: count castles+strongholds (each = 1 region), then supply, then power, then Iron Throne. |

### P1 — Incomplete mechanics

| # | Issue | File:Line | Details |
|---|-------|-----------|---------|
| **P1-1** | **Power token cap (20)** | engine.rs (missing) | Real game limits each house to 20 power tokens. No cap enforced. Power accumulates without limit. Add `.min(20)` to all power-gaining code paths. |
| **P1-2** | **Star order limits not enforced** | engine.rs (missing) | `star_order_limit(player_count, position)` exists in types.rs but is **never called** during order placement. Agents can place unlimited star orders. Should validate in `apply_action` for `PlaceOrders`. |
| **P1-3** | **Order restriction enforcement** | engine.rs (missing) | Westeros cards set `order_restrictions` and `star_order_restrictions`, but these are never checked when orders are placed. Agents can ignore restrictions. |
| **P1-4** | **Muster cost validation** | engine.rs ~L1847 | `apply_action` Muster handler doesn't validate total cost ≤ muster points. Agents can over-muster. |
| **P1-5** | **Ship mustering in ports** | engine.rs ~L77 `get_muster_areas()` | Only returns castle/stronghold land areas. Ships should be muster-able into the connected port, not the land area. Need to add port as a valid muster target. |
| **P1-6** | **5 house cards with missing abilities** | engine.rs | See section 5a below. |
| **P1-7** | **6 wildling cards simplified (win side)** | engine.rs L485-775 | See section 5b below. |
| **P1-8** | **Messenger Raven: wildling peek** | engine.rs ~L806 | Alternative option "look at top wildling card" not implemented. Only order-swap is offered. |
| **P1-9** | **Duplicate order token validation** | engine.rs (missing) | Nothing prevents agents from placing the same token index on multiple areas. |

### P2 — Improvements & extras

| # | Issue | Details |
|---|-------|---------|
| **P2-1** | **Rayon parallelism** | `rayon = "1.8"` in tournament/Cargo.toml but never used. Tournament runs sequentially. Use `par_iter()` for multi-game batches. |
| **P2-2** | **Heuristic agent limitations** | Never uses Messenger Raven (always None), never uses Aeron (always None), never splits armies during march, never builds ships, never upgrades to knight during muster, bidding has hard-coded caps (4/3/2), `westeros_choice` always picks option 0, `doran_choose_track` always IronThrone. |
| **P2-3** | **Tides of Battle** | Optional variant not implemented. Low priority but would add randomness to combat. |
| **P2-4** | **More agent types** | MCTS agent, RL agent, or neural-network agent for stronger play. |
| **P2-5** | **Game replay / serialization** | Save complete action log for replay and analysis. |
| **P2-6** | **Port combat** | Ships in port cannot be directly attacked. Rules allow specific port-raid mechanics. |

### 5a. Missing house card abilities (P1-6)

| Card | House | Missing ability text |
|------|-------|---------------------|
| **SalladhorSaan** | Baratheon | When losing, may retreat ships to any adjacent sea (ignoring normal retreat rules) |
| **EuronCrowsEye** | Greyjoy | Some editions: when winning, remove one of loser's order tokens. Current impl uses base stats only. Verify which edition rules to follow. |
| **MargaeryTyrell** | Tyrell | If defending, remove the attacker's march order (no march bonus). This should trigger in strength calculation before combat resolution. |
| **AreoHotah** | Martell | If losing, may reduce enemy's sword casualties to 0 (your units still retreat, but nothing is killed). |
| **QueenOfThorns** | Tyrell | Pre-combat: remove one adjacent enemy order. Types and apply_action exist, **trigger in resolve_combat_final is missing**. |

### 5b. Simplified wildling card effects (P1-7)

| Card | Current win effect | Correct win effect |
|------|-------------------|-------------------|
| **AKingBeyondTheWall** | Auto-move to Iron Throne position 1 | Highest bidder **chooses** which influence track to take position 1 |
| **MammothRiders** | +5 power | Retrieve bid amount + gain power equal to bid |
| **PreemptiveRaid** | +2 power | Reduce wildling threat by 4 |
| **RattleshirtsRaiders** | +2 power | Highest bidder gains power equal to their bid |
| **SkinchangerScout** | +2 power | Peek at top Westeros deck card, optionally put it on bottom |
| **TheHordeDescends** | Auto-build 1 footman | Muster 2 points worth of units anywhere |

---

## 6. How to Run

```bash
# Build
cargo build --release

# Run a single game (prints winner + round)
cargo run -- play --seed 42 --players 6 --agent random

# Run tournament (50 games, heuristic agents, save to SQLite)
cargo run -- tournament --games 50 --players 6 --agent heuristic --db results.db

# View leaderboard
cargo run -- leaderboard --db results.db

# Run tests
cargo test

# Lint
cargo clippy
```

### Agent types for CLI `--agent`

- `random` — uniform random legal moves
- `heuristic` — scored march destinations, strategic orders, situational card play
- `mixed` — alternates heuristic/random per house (for comparison)

---

## 7. Code Navigation Guide

### Where is combat logic?

- `advance_combat()` at engine.rs ~L943: iterative loop for combat phases
- `begin_combat()` at engine.rs ~L1100: sets up CombatState
- `resolve_combat_final()` at engine.rs ~L1157: card abilities, strength calc, casualties, retreat
- `finalize_combat()` at engine.rs ~L1726: cleanup, advance to next action player

### Where are house card abilities?

Inside `resolve_combat_final()`:
- **Pre-combat abilities** (Tyrion, Aeron): ~L1015-1095
- **Strength modifiers** (Catelyn, Stannis, Victarion, Mace, Blackfish, Jaime, Greatjon, Obara, Balon): ~L1239-1347
- **Post-combat / winner abilities** (Renly, Arianne, Roose, Tywin, Davos, Theon, Kevan, Melisandre, Cersei, Asha, Robb, Nymeria, Patchface, Doran, Loras): ~L1375-1720

### Where is order placement?

- `advance_planning()` at engine.rs ~L787: iterates turn order, creates PlaceOrders pending
- `apply_action` PlaceOrders handler at engine.rs ~L1760: places tokens on areas
- MessengerRaven: engine.rs ~L806 (trigger) and ~L1776 (apply)

### Where is bidding?

- `begin_clash_of_kings()` at engine.rs ~L337
- `begin_wildling_attack()` at engine.rs ~L348
- `advance_bidding()` at engine.rs ~L363
- `resolve_track_bidding()` at engine.rs ~L394
- `resolve_wildling_bidding()` at engine.rs ~L454: all 9 wildling card effects

### Where is setup?

- `create_initial_state(player_count, seed)` in setup.rs
- `house_setups()` defines per-house starting config
- Neutral garrisons added at setup.rs ~L168-210

### Where are tests?

- `engine/src/tests.rs` — 55 tests (setup, supply, cards, combat, visibility, determinism, stress)
- `engine/src/setup.rs` — 3 tests (at bottom of file)
- Run with `cargo test`

---

## 8. Agent Trait Interface

```rust
// agents/src/agent.rs
pub trait Agent: Send + Sync {
    fn name(&self) -> &str;
    fn house(&self) -> HouseName;
    fn decide(&mut self, view: &PlayerView) -> Action;

    // Individual decision methods (called by default decide() impl):
    fn place_orders(&self, view: &PlayerView) -> Vec<(AreaId, u8)>;
    fn choose_raid(&self, view: &PlayerView, from: AreaId, targets: &[AreaId]) -> Option<AreaId>;
    fn choose_march(&self, view: &PlayerView, from: AreaId, dests: &[AreaId]) -> Action;
    fn leave_power_token(&self, view: &PlayerView, area: AreaId) -> bool;
    fn declare_support(&self, view: &PlayerView, ...) -> SupportChoice;
    fn select_card(&self, view: &PlayerView, cards: &[HouseCardId]) -> HouseCardId;
    fn use_valyrian_blade(&self, view: &PlayerView) -> bool;
    fn submit_bid(&self, view: &PlayerView, bid_type: BiddingType, ...) -> u8;
    fn westeros_choice(&self, view: &PlayerView, options: &[String]) -> usize;
    fn choose_muster(&self, view: &PlayerView, areas: &[MusterArea]) -> Vec<(AreaId, MusterAction2)>;
    fn choose_retreat(&self, ...) -> AreaId;
    fn reconcile(&self, ...) -> (AreaId, usize);
    fn use_messenger_raven(&self, view: &PlayerView) -> Option<(AreaId, u8)>;
    fn use_aeron(&self, view: &PlayerView) -> Option<HouseCardId>;
    fn tyrion_replace(&self, ...) -> HouseCardId;
    fn patchface_discard(&self, ...) -> HouseCardId;
    fn robb_retreat(&self, ...) -> AreaId;
    fn cersei_remove_order(&self, ...) -> AreaId;
    fn doran_choose_track(&self, ...) -> Track;
    fn queen_of_thorns(&self, ...) -> AreaId;
    fn wildling_penalty(&self, ...) -> usize;
}
```

---

## 9. Performance Benchmarks

From tournament runs (debug build, single-threaded):

- **500 random games (6p)**: ~2 seconds total, 0 errors, 0 panics
- **50 heuristic-only (6p)**: Greyjoy dominant at 48% win rate
- **50 mixed heuristic/random (6p)**: Heuristic houses win 56% vs random 44%
- **Stack overflow**: Fixed by converting `advance()` and `advance_combat()` from
  recursive to iterative loops with progress detection.

---

## 10. Implementation Order for Next Session

Recommended priority for the next LLM session:

### Phase 1: P0 bug fixes (do first)
1. **P0-1**: House card recycling — add check in `advance_combat()` before card selection:
   if hand is empty and discards > 0, move all discards back to hand.
2. **P0-4**: Queen of Thorns trigger — add pre-combat check in `resolve_combat_final()`
   that fires `PendingDecision::QueenOfThornsRemoveOrder` when QoT is played.
3. **P0-2 + P0-3**: Port destruction + control — after march conquers land, check
   `AREAS[area].connected_port`, destroy enemy ships there, update port ownership.
4. **P0-5**: Fix tiebreaker to count regions (each castle/stronghold = 1).

### Phase 2: P1 validation & mechanics
5. **P1-1**: Add `.min(20)` power cap everywhere power is gained.
6. **P1-2 + P1-3 + P1-9**: Add order placement validation (star limits, restrictions, no dupes).
7. **P1-4**: Muster cost validation.
8. **P1-5**: Port mustering — extend `get_muster_areas()` to include port for ship building.
9. **P1-6**: Missing card abilities (QoT trigger, Margaery, Areo, Salladhor, Euron).
10. **P1-7**: Fix 6 simplified wildling card win effects.
11. **P1-8**: Messenger Raven wildling peek option.

### Phase 3: Improvements
12. **P2-1**: Wire up rayon for parallel tournament games.
13. **P2-2**: Improve heuristic agent decision-making.
14. Run `cargo test` + `cargo clippy` after each phase. Commit + push.

---

## 11. File Hashes / Line Counts (for drift detection)

```
engine/src/engine.rs      2320 lines
engine/src/types.rs        542 lines
engine/src/map.rs          320 lines
engine/src/cards.rs        136 lines
engine/src/setup.rs        281 lines
engine/src/supply.rs        70 lines
engine/src/navigation.rs    75 lines
engine/src/visibility.rs   243 lines
engine/src/tests.rs        699 lines
agents/src/agent.rs        143 lines
agents/src/heuristic.rs    402 lines
agents/src/random.rs       149 lines
tournament/src/database.rs 155 lines
tournament/src/runner.rs   125 lines
runner/src/main.rs         145 lines
```

**Git**: 3 commits on `master`, latest `39049c2`

---

## 12. Conventions

- All game logic is in `engine/` — pure functions, no I/O, no agent calls.
- Agents only receive `PlayerView`, never `GameState`.
- Deterministic RNG: `ChaCha8Rng::seed_from_u64(seed + counter * 6364136223846793005)`.
- Area IDs are `AreaId(u8)` indices into `AREAS: [AreaDef; 56]`.
- Order tokens are indexed 0–14 into `ORDER_TOKENS: [OrderTokenDef; 15]`.
- House cards are identified by `HouseCardId` enum, looked up via `cards::get_house_card()`.
- Portuguese may appear in user messages; code and comments are English.

// ═══════════════════════════════════════════════════════════════════════
// Database — SQLite storage for tournament results and ELO ratings
// ═══════════════════════════════════════════════════════════════════════

use rusqlite::{Connection, params};
use crate::runner::GameResult;

pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open (or create) a database at the given path.
    pub fn new(path: &str) -> Self {
        let conn = Connection::open(path).expect("Failed to open database");
        let db = Database { conn };
        db.create_schema();
        db
    }

    /// In-memory database (useful for tests).
    pub fn in_memory() -> Self {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory database");
        let db = Database { conn };
        db.create_schema();
        db
    }

    fn create_schema(&self) {
        self.conn.execute_batch("
            CREATE TABLE IF NOT EXISTS agents (
                id          INTEGER PRIMARY KEY,
                name        TEXT NOT NULL UNIQUE,
                elo         REAL NOT NULL DEFAULT 1500.0,
                games       INTEGER NOT NULL DEFAULT 0,
                wins        INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS games (
                id          INTEGER PRIMARY KEY,
                seed        INTEGER NOT NULL,
                rounds      INTEGER NOT NULL,
                winner      TEXT NOT NULL,
                played_at   TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS game_players (
                id          INTEGER PRIMARY KEY,
                game_id     INTEGER NOT NULL REFERENCES games(id),
                agent_id    INTEGER NOT NULL REFERENCES agents(id),
                house       TEXT NOT NULL,
                castles     INTEGER NOT NULL,
                supply      INTEGER NOT NULL,
                power       INTEGER NOT NULL,
                iron_throne INTEGER NOT NULL,
                fiefdoms    INTEGER NOT NULL,
                kings_court INTEGER NOT NULL
            );
        ").expect("Failed to create schema");
    }

    /// Register an agent (or return existing ID).
    pub fn register_agent(&self, name: &str) -> i64 {
        self.conn.execute(
            "INSERT OR IGNORE INTO agents (name) VALUES (?1)",
            params![name],
        ).expect("Failed to register agent");
        self.conn.query_row(
            "SELECT id FROM agents WHERE name = ?1",
            params![name],
            |row| row.get(0),
        ).expect("Failed to get agent id")
    }

    /// Store a completed game result.
    pub fn store_game(&self, result: &GameResult, agent_ids: &[(String, i64)]) -> i64 {
        self.conn.execute(
            "INSERT INTO games (seed, rounds, winner) VALUES (?1, ?2, ?3)",
            params![result.seed as i64, result.rounds_played as i64, result.winner.to_string()],
        ).expect("Failed to store game");
        let game_id = self.conn.last_insert_rowid();

        for pr in &result.player_results {
            let agent_id = agent_ids.iter()
                .find(|(name, _)| *name == pr.agent_name || name == &pr.house.to_string())
                .map(|(_, id)| *id)
                .unwrap_or(0);

            self.conn.execute(
                "INSERT INTO game_players (game_id, agent_id, house, castles, supply, power, iron_throne, fiefdoms, kings_court)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    game_id,
                    agent_id,
                    pr.house.to_string(),
                    pr.final_castles as i64,
                    pr.final_supply as i64,
                    pr.final_power as i64,
                    pr.final_iron_throne as i64,
                    pr.final_fiefdoms as i64,
                    pr.final_kings_court as i64,
                ],
            ).expect("Failed to store game player");
        }

        // Update agent stats
        for (name, agent_id) in agent_ids {
            let won = result.winner.to_string() == *name
                || result.player_results.iter().any(|pr| pr.agent_name == *name && pr.house == result.winner);
            self.conn.execute(
                "UPDATE agents SET games = games + 1, wins = wins + ?1 WHERE id = ?2",
                params![if won { 1 } else { 0 }, agent_id],
            ).expect("Failed to update agent stats");
        }

        game_id
    }

    /// Update ELO ratings for a set of agents after a game.
    /// Simple multiplayer ELO: winner gains K points from each loser.
    pub fn update_elo(&self, winner_id: i64, loser_ids: &[i64], k: f64) {
        let winner_elo: f64 = self.conn.query_row(
            "SELECT elo FROM agents WHERE id = ?1",
            params![winner_id],
            |row| row.get(0),
        ).unwrap_or(1500.0);

        for &loser_id in loser_ids {
            let loser_elo: f64 = self.conn.query_row(
                "SELECT elo FROM agents WHERE id = ?1",
                params![loser_id],
                |row| row.get(0),
            ).unwrap_or(1500.0);

            let expected_winner = 1.0 / (1.0 + 10f64.powf((loser_elo - winner_elo) / 400.0));
            let expected_loser = 1.0 - expected_winner;

            let delta_w = k * (1.0 - expected_winner);
            let delta_l = k * (0.0 - expected_loser);

            self.conn.execute(
                "UPDATE agents SET elo = elo + ?1 WHERE id = ?2",
                params![delta_w, winner_id],
            ).expect("Failed to update winner ELO");
            self.conn.execute(
                "UPDATE agents SET elo = elo + ?1 WHERE id = ?2",
                params![delta_l, loser_id],
            ).expect("Failed to update loser ELO");
        }
    }

    /// Get ELO leaderboard.
    pub fn leaderboard(&self) -> Vec<(String, f64, u32, u32)> {
        let mut stmt = self.conn.prepare(
            "SELECT name, elo, games, wins FROM agents ORDER BY elo DESC"
        ).expect("Failed to prepare leaderboard query");

        stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, u32>(2)?,
                row.get::<_, u32>(3)?,
            ))
        })
        .expect("Failed to query leaderboard")
        .filter_map(|r| r.ok())
        .collect()
    }

    /// Get total number of games stored.
    pub fn game_count(&self) -> u32 {
        self.conn.query_row("SELECT COUNT(*) FROM games", [], |row| row.get(0))
            .unwrap_or(0)
    }
}

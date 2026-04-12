use crate::error::{Result, SpecStoreError};
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub id: String,
    pub text: String,
    pub tags: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worktree {
    pub branch: String,
    pub contract: Option<String>,
    pub owner: Option<String>,
    pub claimed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredFn {
    pub id: String,
    pub name: String,
    pub file: String,
    pub line: usize,
    pub description: String,
    pub is_test: bool,
    pub created_at: String,
}

pub struct StructuredStore {
    conn: Connection,
}

impl StructuredStore {
    pub fn open(root: &Path) -> Result<Self> {
        let db_path = root.join(".spec-store").join("store.db");
        std::fs::create_dir_all(db_path.parent().unwrap())
            .map_err(|e| SpecStoreError::Store(e.to_string()))?;
        let conn =
            Connection::open(&db_path).map_err(|e| SpecStoreError::Database(e.to_string()))?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn =
            Connection::open_in_memory().map_err(|e| SpecStoreError::Database(e.to_string()))?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<()> {
        self.conn
            .execute_batch(SCHEMA)
            .map_err(|e| SpecStoreError::Database(e.to_string()))?;
        // V2: add is_test column (ignore if already exists)
        let _ = self
            .conn
            .execute_batch("ALTER TABLE functions ADD COLUMN is_test INTEGER NOT NULL DEFAULT 0");
        self.conn
            .execute_batch(SCHEMA_V2)
            .map_err(|e| SpecStoreError::Database(e.to_string()))
    }

    pub fn add_decision(&self, text: &str, tags: &[String]) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let tags_json = serde_json::to_string(tags).unwrap_or_default();
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO decisions (id, text, tags, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![id, text, tags_json, now],
            )
            .map_err(|e| SpecStoreError::Database(e.to_string()))?;
        Ok(id)
    }

    pub fn list_decisions(&self) -> Result<Vec<Decision>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, text, tags, created_at FROM decisions ORDER BY created_at DESC")
            .map_err(|e| SpecStoreError::Database(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|e| SpecStoreError::Database(e.to_string()))?;
        rows.map(|r| {
            let (id, text, tags_json, created_at) =
                r.map_err(|e| SpecStoreError::Database(e.to_string()))?;
            let tags = serde_json::from_str(&tags_json).unwrap_or_default();
            Ok(Decision {
                id,
                text,
                tags,
                created_at,
            })
        })
        .collect()
    }

    pub fn claim_worktree(
        &self,
        branch: &str,
        contract: Option<&str>,
        owner: Option<&str>,
    ) -> Result<()> {
        if self.get_worktree(branch)?.is_some() {
            return Err(SpecStoreError::WorktreeConflict(format!(
                "Worktree '{branch}' is already claimed"
            )));
        }
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO worktrees (branch, contract, owner, claimed_at) VALUES (?1, ?2, ?3, ?4)",
            params![branch, contract, owner, now],
        ).map_err(|e| SpecStoreError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn release_worktree(&self, branch: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM worktrees WHERE branch = ?1", params![branch])
            .map_err(|e| SpecStoreError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn get_worktree(&self, branch: &str) -> Result<Option<Worktree>> {
        let mut stmt = self
            .conn
            .prepare("SELECT branch, contract, owner, claimed_at FROM worktrees WHERE branch = ?1")
            .map_err(|e| SpecStoreError::Database(e.to_string()))?;
        let mut rows = stmt
            .query(params![branch])
            .map_err(|e| SpecStoreError::Database(e.to_string()))?;
        if let Some(row) = rows
            .next()
            .map_err(|e| SpecStoreError::Database(e.to_string()))?
        {
            return Ok(Some(Worktree {
                branch: row.get(0).unwrap_or_default(),
                contract: row.get(1).unwrap_or(None),
                owner: row.get(2).unwrap_or(None),
                claimed_at: row.get(3).unwrap_or_default(),
            }));
        }
        Ok(None)
    }

    pub fn list_worktrees(&self) -> Result<Vec<Worktree>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT branch, contract, owner, claimed_at FROM worktrees ORDER BY claimed_at",
            )
            .map_err(|e| SpecStoreError::Database(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Worktree {
                    branch: row.get(0)?,
                    contract: row.get(1)?,
                    owner: row.get(2)?,
                    claimed_at: row.get(3)?,
                })
            })
            .map_err(|e| SpecStoreError::Database(e.to_string()))?;
        rows.map(|r| r.map_err(|e| SpecStoreError::Database(e.to_string())))
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn register_fn(
        &self,
        name: &str,
        file: &str,
        line: usize,
        desc: &str,
        is_test: bool,
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT OR REPLACE INTO functions (id, name, file, line, description, is_test, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![id, name, file, line as i64, desc, is_test, now],
            )
            .map_err(|e| SpecStoreError::Database(e.to_string()))?;
        Ok(id)
    }

    pub fn list_functions(&self) -> Result<Vec<RegisteredFn>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, file, line, description, is_test, created_at FROM functions")
            .map_err(|e| SpecStoreError::Database(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(RegisteredFn {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    file: row.get(2)?,
                    line: row.get::<_, i64>(3)? as usize,
                    description: row.get(4)?,
                    is_test: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })
            .map_err(|e| SpecStoreError::Database(e.to_string()))?;
        rows.map(|r| r.map_err(|e| SpecStoreError::Database(e.to_string())))
            .collect()
    }
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS decisions (
    id TEXT PRIMARY KEY, text TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT '[]', created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS features (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'planned', owner TEXT,
    worktree TEXT, created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS worktrees (
    branch TEXT PRIMARY KEY, contract TEXT,
    owner TEXT, claimed_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS functions (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, file TEXT NOT NULL,
    line INTEGER NOT NULL, description TEXT NOT NULL, created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS coverage_baselines (
    file TEXT PRIMARY KEY, percentage REAL NOT NULL, updated_at TEXT NOT NULL
);
";

const SCHEMA_V2: &str = "
CREATE TABLE IF NOT EXISTS test_mappings (
    id TEXT PRIMARY KEY,
    function_file TEXT NOT NULL, function_name TEXT NOT NULL,
    test_file TEXT NOT NULL, test_name TEXT NOT NULL,
    match_type TEXT NOT NULL, created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS function_coverage (
    file TEXT NOT NULL, function_name TEXT NOT NULL,
    line_start INTEGER NOT NULL, line_count INTEGER NOT NULL,
    lines_found INTEGER NOT NULL, lines_hit INTEGER NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (file, function_name)
);
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory_succeeds() {
        assert!(StructuredStore::open_in_memory().is_ok());
    }

    #[test]
    fn add_and_list_decisions() {
        let store = StructuredStore::open_in_memory().unwrap();
        store
            .add_decision("Use HMAC-SHA256 for all tokens", &["security".into()])
            .unwrap();
        let decisions = store.list_decisions().unwrap();
        assert_eq!(decisions.len(), 1);
        assert!(decisions[0].text.contains("HMAC"));
        assert_eq!(decisions[0].tags, vec!["security"]);
    }

    #[test]
    fn claim_and_release_worktree() {
        let store = StructuredStore::open_in_memory().unwrap();
        store
            .claim_worktree("feat/auth", Some("contracts/auth.yaml"), Some("agent-1"))
            .unwrap();
        let wt = store.get_worktree("feat/auth").unwrap().unwrap();
        assert_eq!(wt.owner, Some("agent-1".into()));
        store.release_worktree("feat/auth").unwrap();
        assert!(store.get_worktree("feat/auth").unwrap().is_none());
    }

    #[test]
    fn double_claim_returns_error() {
        let store = StructuredStore::open_in_memory().unwrap();
        store.claim_worktree("feat/auth", None, None).unwrap();
        let result = store.claim_worktree("feat/auth", None, None);
        assert!(matches!(result, Err(SpecStoreError::WorktreeConflict(_))));
    }

    #[test]
    fn register_and_list_functions() {
        let store = StructuredStore::open_in_memory().unwrap();
        store
            .register_fn(
                "validate_stake",
                "src/risk.rs",
                42,
                "Validates stake limit",
                false,
            )
            .unwrap();
        let fns = store.list_functions().unwrap();
        assert_eq!(fns.len(), 1);
        assert_eq!(fns[0].name, "validate_stake");
        assert_eq!(fns[0].line, 42);
    }

    #[test]
    fn list_worktrees_returns_all() {
        let store = StructuredStore::open_in_memory().unwrap();
        store.claim_worktree("feat/a", None, None).unwrap();
        store.claim_worktree("feat/b", None, None).unwrap();
        assert_eq!(store.list_worktrees().unwrap().len(), 2);
    }
}

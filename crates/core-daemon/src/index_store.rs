use core_daemon::types::{IndexScope, SearchResultItem};
use rusqlite::{params, Connection};

#[derive(Clone)]
pub struct IndexStore {
    db_path: String,
}

impl IndexStore {
    pub fn new(db_path: String) -> Result<Self, String> {
        let store = Self { db_path };
        store.init()?;
        Ok(store)
    }

    fn connect(&self) -> Result<Connection, String> {
        Connection::open(&self.db_path).map_err(|e| format!("db_open_failed: {e}"))
    }

    fn init(&self) -> Result<(), String> {
        let conn = self.connect()?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS index_scopes (
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                enabled INTEGER NOT NULL,
                created_at TEXT NOT NULL
            );
            ",
        )
        .map_err(|e| format!("db_init_failed: {e}"))?;
        Ok(())
    }

    pub fn list_scopes(&self) -> Result<Vec<IndexScope>, String> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare("SELECT id, path, enabled, created_at FROM index_scopes ORDER BY created_at")
            .map_err(|e| format!("db_prepare_list_failed: {e}"))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(IndexScope {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    enabled: row.get::<_, i64>(2)? != 0,
                    created_at: row.get(3)?,
                })
            })
            .map_err(|e| format!("db_query_list_failed: {e}"))?;

        let mut scopes = Vec::new();
        for item in rows {
            scopes.push(item.map_err(|e| format!("db_row_parse_failed: {e}"))?);
        }
        Ok(scopes)
    }

    pub fn create_scope(&self, scope: &IndexScope) -> Result<(), String> {
        let conn = self.connect()?;
        conn.execute(
            "INSERT OR IGNORE INTO index_scopes (id, path, enabled, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![scope.id, scope.path, if scope.enabled { 1 } else { 0 }, scope.created_at],
        )
        .map_err(|e| format!("db_insert_scope_failed: {e}"))?;
        Ok(())
    }

    pub fn delete_scope(&self, id: &str) -> Result<bool, String> {
        let conn = self.connect()?;
        let changed = conn
            .execute("DELETE FROM index_scopes WHERE id = ?1", params![id])
            .map_err(|e| format!("db_delete_scope_failed: {e}"))?;
        Ok(changed > 0)
    }

    pub fn search_scope_paths(&self, q: &str) -> Result<Vec<SearchResultItem>, String> {
        let conn = self.connect()?;
        let pattern = format!("%{}%", q.to_lowercase());

        let mut stmt = conn
            .prepare(
                "SELECT id, path FROM index_scopes WHERE lower(path) LIKE ?1 ORDER BY created_at LIMIT 50",
            )
            .map_err(|e| format!("db_prepare_search_failed: {e}"))?;

        let rows = stmt
            .query_map(params![pattern], |row| {
                Ok(SearchResultItem {
                    scope_id: row.get(0)?,
                    path: row.get(1)?,
                    match_reason: "scope_path_contains_query",
                })
            })
            .map_err(|e| format!("db_query_search_failed: {e}"))?;

        let mut results = Vec::new();
        for item in rows {
            results.push(item.map_err(|e| format!("db_search_row_parse_failed: {e}"))?);
        }
        Ok(results)
    }
}

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

            CREATE TABLE IF NOT EXISTS file_metadata (
                scope_id TEXT NOT NULL,
                path TEXT PRIMARY KEY,
                size_bytes INTEGER,
                mtime TEXT,
                updated_at TEXT NOT NULL,
                FOREIGN KEY(scope_id) REFERENCES index_scopes(id) ON DELETE CASCADE
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

    pub fn upsert_file_metadata(
        &self,
        scope_id: &str,
        path: &str,
        size_bytes: Option<i64>,
        mtime: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.connect()?;
        conn.execute(
            "
            INSERT INTO file_metadata (scope_id, path, size_bytes, mtime, updated_at)
            VALUES (?1, ?2, ?3, ?4, datetime('now'))
            ON CONFLICT(path) DO UPDATE SET
                scope_id = excluded.scope_id,
                size_bytes = excluded.size_bytes,
                mtime = excluded.mtime,
                updated_at = datetime('now')
            ",
            params![scope_id, path, size_bytes, mtime],
        )
        .map_err(|e| format!("db_upsert_file_metadata_failed: {e}"))?;
        Ok(())
    }

    pub fn delete_file_metadata(&self, path: &str) -> Result<bool, String> {
        let conn = self.connect()?;
        let changed = conn
            .execute("DELETE FROM file_metadata WHERE path = ?1", params![path])
            .map_err(|e| format!("db_delete_file_metadata_failed: {e}"))?;
        Ok(changed > 0)
    }

    pub fn rename_file_metadata(
        &self,
        old_path: &str,
        new_path: &str,
        scope_id: &str,
        size_bytes: Option<i64>,
        mtime: Option<&str>,
    ) -> Result<(), String> {
        let _ = self.delete_file_metadata(old_path)?;
        self.upsert_file_metadata(scope_id, new_path, size_bytes, mtime)
    }

    pub fn search_scope_paths(&self, q: &str) -> Result<Vec<SearchResultItem>, String> {
        let conn = self.connect()?;
        let pattern = format!("%{}%", q.to_lowercase());

        let mut results = Vec::new();

        let mut scope_stmt = conn
            .prepare(
                "SELECT id, path FROM index_scopes WHERE lower(path) LIKE ?1 ORDER BY created_at LIMIT 25",
            )
            .map_err(|e| format!("db_prepare_scope_search_failed: {e}"))?;

        let scope_rows = scope_stmt
            .query_map(params![&pattern], |row| {
                Ok(SearchResultItem {
                    scope_id: row.get(0)?,
                    path: row.get(1)?,
                    match_reason: "scope_path_contains_query",
                })
            })
            .map_err(|e| format!("db_query_scope_search_failed: {e}"))?;

        for item in scope_rows {
            results.push(item.map_err(|e| format!("db_scope_search_row_parse_failed: {e}"))?);
        }

        let mut file_stmt = conn
            .prepare(
                "SELECT scope_id, path FROM file_metadata WHERE lower(path) LIKE ?1 ORDER BY updated_at DESC LIMIT 25",
            )
            .map_err(|e| format!("db_prepare_file_search_failed: {e}"))?;

        let file_rows = file_stmt
            .query_map(params![pattern], |row| {
                Ok(SearchResultItem {
                    scope_id: row.get(0)?,
                    path: row.get(1)?,
                    match_reason: "file_path_contains_query",
                })
            })
            .map_err(|e| format!("db_query_file_search_failed: {e}"))?;

        for item in file_rows {
            results.push(item.map_err(|e| format!("db_file_search_row_parse_failed: {e}"))?);
        }

        Ok(results)
    }
}

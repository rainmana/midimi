use serde::Serialize;
use turso::{Builder, Value};

#[derive(Serialize, Clone, Debug)]
pub struct LibraryRow {
    pub id: i64,
    pub path: String,
    pub title: Option<String>,
    pub duration_sec: f64,
}

#[derive(Serialize, Clone, Debug)]
pub struct SoundfontRow {
    pub id: i64,
    pub path: String,
    pub name: String,
    pub is_builtin: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

pub async fn open(path: &str) -> Result<turso::Connection, String> {
    let db = Builder::new_local(path)
        .build()
        .await
        .map_err(|e| e.to_string())?;
    let conn = db.connect().map_err(|e| e.to_string())?; // connect() is SYNC
    for stmt in [
        "CREATE TABLE IF NOT EXISTS library (id INTEGER PRIMARY KEY, path TEXT UNIQUE NOT NULL, title TEXT, duration_sec REAL, last_opened_at INTEGER NOT NULL)",
        "CREATE TABLE IF NOT EXISTS soundfonts (id INTEGER PRIMARY KEY, path TEXT UNIQUE NOT NULL, name TEXT NOT NULL, is_builtin INTEGER NOT NULL DEFAULT 0)",
        "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
    ] {
        conn.execute(stmt, ()).await.map_err(|e| e.to_string())?;
    }
    Ok(conn)
}

fn opt_text(v: Value) -> Option<String> {
    match v {
        Value::Text(s) => Some(s),
        _ => None,
    }
}

pub async fn upsert_recent(
    conn: &turso::Connection,
    path: &str,
    title: Option<&str>,
    duration_sec: f64,
    now_unix: i64,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO library (path, title, duration_sec, last_opened_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(path) DO UPDATE SET title=?2, duration_sec=?3, last_opened_at=?4",
        (path, title, duration_sec, now_unix),
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn list_recent(
    conn: &turso::Connection,
    limit: i64,
) -> Result<Vec<LibraryRow>, String> {
    let mut rows = conn
        .query(
            "SELECT id, path, title, duration_sec FROM library ORDER BY last_opened_at DESC LIMIT ?1",
            (limit,),
        )
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(LibraryRow {
            id: row.get::<i64>(0).map_err(|e| e.to_string())?,
            path: row.get::<String>(1).map_err(|e| e.to_string())?,
            title: opt_text(row.get_value(2).map_err(|e| e.to_string())?),
            duration_sec: row.get::<f64>(3).map_err(|e| e.to_string())?,
        });
    }
    Ok(out)
}

pub async fn register_soundfont(
    conn: &turso::Connection,
    path: &str,
    name: &str,
    is_builtin: bool,
) -> Result<SoundfontRow, String> {
    conn.execute(
        "INSERT INTO soundfonts (path, name, is_builtin) VALUES (?1, ?2, ?3)
         ON CONFLICT(path) DO UPDATE SET name=?2, is_builtin=?3",
        (path, name, is_builtin as i64),
    )
    .await
    .map_err(|e| e.to_string())?;
    let mut rows = conn
        .query(
            "SELECT id, path, name, is_builtin FROM soundfonts WHERE path=?1",
            (path,),
        )
        .await
        .map_err(|e| e.to_string())?;
    let row = rows
        .next()
        .await
        .map_err(|e| e.to_string())?
        .ok_or("soundfont insert vanished")?;
    Ok(SoundfontRow {
        id: row.get::<i64>(0).map_err(|e| e.to_string())?,
        path: row.get::<String>(1).map_err(|e| e.to_string())?,
        name: row.get::<String>(2).map_err(|e| e.to_string())?,
        is_builtin: row.get::<i64>(3).map_err(|e| e.to_string())? != 0,
    })
}

pub async fn list_soundfonts(conn: &turso::Connection) -> Result<Vec<SoundfontRow>, String> {
    let mut rows = conn
        .query(
            "SELECT id, path, name, is_builtin FROM soundfonts ORDER BY is_builtin DESC, name ASC",
            (),
        )
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(SoundfontRow {
            id: row.get::<i64>(0).map_err(|e| e.to_string())?,
            path: row.get::<String>(1).map_err(|e| e.to_string())?,
            name: row.get::<String>(2).map_err(|e| e.to_string())?,
            is_builtin: row.get::<i64>(3).map_err(|e| e.to_string())? != 0,
        });
    }
    Ok(out)
}

pub async fn get_setting(
    conn: &turso::Connection,
    key: &str,
) -> Result<Option<String>, String> {
    let mut rows = conn
        .query("SELECT value FROM settings WHERE key=?1", (key,))
        .await
        .map_err(|e| e.to_string())?;
    match rows.next().await.map_err(|e| e.to_string())? {
        Some(row) => Ok(Some(row.get::<String>(0).map_err(|e| e.to_string())?)),
        None => Ok(None),
    }
}

pub async fn set_setting(
    conn: &turso::Connection,
    key: &str,
    value: &str,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=?2",
        (key, value),
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn list_settings(conn: &turso::Connection) -> Result<Vec<Setting>, String> {
    let mut rows = conn
        .query("SELECT key, value FROM settings", ())
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(Setting {
            key: row.get::<String>(0).map_err(|e| e.to_string())?,
            value: row.get::<String>(1).map_err(|e| e.to_string())?,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn settings_roundtrip() {
        let conn = open(":memory:").await.unwrap();
        assert_eq!(get_setting(&conn, "theme").await.unwrap(), None);
        set_setting(&conn, "theme", "cosmic").await.unwrap();
        set_setting(&conn, "theme", "aurora").await.unwrap(); // upsert overwrites
        assert_eq!(get_setting(&conn, "theme").await.unwrap(), Some("aurora".to_string()));
        assert_eq!(list_settings(&conn).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn recent_is_ordered_and_deduped() {
        let conn = open(":memory:").await.unwrap();
        upsert_recent(&conn, "/a.mid", Some("A"), 1.0, 100).await.unwrap();
        upsert_recent(&conn, "/b.mid", Some("B"), 2.0, 200).await.unwrap();
        upsert_recent(&conn, "/a.mid", Some("A"), 1.0, 300).await.unwrap(); // re-open A, newest
        let rows = list_recent(&conn, 10).await.unwrap();
        assert_eq!(rows.len(), 2, "path is unique");
        assert_eq!(rows[0].path, "/a.mid", "most-recent first");
        assert_eq!(rows[1].path, "/b.mid");
    }

    #[tokio::test]
    async fn soundfont_register_and_list() {
        let conn = open(":memory:").await.unwrap();
        let row = register_soundfont(&conn, "/gm.sf2", "GeneralUser GS", true).await.unwrap();
        assert!(row.id > 0);
        assert!(row.is_builtin);
        let all = list_soundfonts(&conn).await.unwrap();
        assert_eq!(all.len(), 1);
        // Re-register same path is idempotent (no duplicate).
        register_soundfont(&conn, "/gm.sf2", "GeneralUser GS", true).await.unwrap();
        assert_eq!(list_soundfonts(&conn).await.unwrap().len(), 1);
    }
}

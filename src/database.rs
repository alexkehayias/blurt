//! Database module for reading macOS notifications from SQLite.

use tokio_rusqlite::Connection as TokioConnection;
use std::path::Path;
use rusqlite::{OpenFlags, params};

/// Represents a notification from the system database
#[derive(Debug, Clone, serde::Serialize)]
pub struct Notification {
    pub id: i64,
    pub title: String,
    pub subtitle: Option<String>,
    pub body: String,
    pub date: i64,
    pub bundle_id: Option<String>,
}

/// Database handler for macOS notification database
pub struct NotificationDatabase {
    db_path: String,
    read_only: bool,
}

impl NotificationDatabase {
    /// Create a new database handler (read-only by default)
    pub fn new(db_path: &str) -> Self {
        Self {
            db_path: db_path.to_string(),
            read_only: true,
        }
    }

    /// Create a new database handler with specified read-only mode
    pub fn new_with_mode(db_path: &str, read_only: bool) -> Self {
        Self {
            db_path: db_path.to_string(),
            read_only,
        }
    }

    /// Connect to the database
    pub async fn connect(&self) -> Result<TokioConnection, Box<dyn std::error::Error>> {
        let db_path = self.db_path.clone();
        let flags = if self.read_only {
            OpenFlags::SQLITE_OPEN_READ_ONLY
        } else {
            OpenFlags::default()
        };
        let conn = tokio_rusqlite::Connection::open_with_flags(db_path, flags).await?;
        Ok(conn)
    }

    
    /// Check if the database file exists
    pub fn exists(&self) -> bool {
        Path::new(&self.db_path).exists()
    }

    /// Get the database path
    pub fn db_path(&self) -> &str {
        &self.db_path
    }

    /// Initialize the database with the notification schema
    pub async fn init_schema(&self) -> Result<(), Box<dyn std::error::Error>> {
        // For in-memory databases, we need to open with the shared cache URI
        let db_path = self.db_path.clone();
        let conn = tokio_rusqlite::Connection::open_with_flags(db_path, OpenFlags::default()).await?;
        conn.call(|db_conn| {
            db_conn.execute_batch(SCHEMA)?;
            Ok(())
        }).await?;
        Ok(())
    }

    /// Insert a test notification record
    pub async fn insert_test_notification(&self, app_id: i64, uuid: Vec<u8>, data: Vec<u8>,
                                          request_date: f64, request_last_date: f64,
                                          delivered_date: f64, presented: bool,
                                          style: i64, snooze_fire_date: f64) -> Result<i64, Box<dyn std::error::Error>> {
        let conn = self.connect().await?;
        let rec_id = conn.call(move |db_conn| {
            let rec_id: i64 = db_conn.query_row(
                "INSERT INTO record (app_id, uuid, data, request_date, request_last_date,
                  delivered_date, presented, style, snooze_fire_date)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![app_id, uuid, data, request_date, request_last_date, delivered_date, presented, style, snooze_fire_date],
                |row| row.get(0)
            )?;
            Ok(rec_id)
        }).await?;
        Ok(rec_id)
    }
}

/// SQL schema for the notification database
pub const SCHEMA: &str = r#"
CREATE TABLE record (
    rec_id INTEGER PRIMARY KEY,
    app_id INTEGER,
    uuid BLOB,
    data BLOB,
    request_date REAL,
    request_last_date REAL,
    delivered_date REAL,
    presented Bool,
    style INTEGER,
    snooze_fire_date REAL
);
"#;
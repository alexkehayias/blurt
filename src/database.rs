//! Database module for reading macOS notifications from SQLite.

use tokio_rusqlite::Connection as TokioConnection;
use std::path::Path;

/// Represents a notification from the system database
#[derive(Debug, Clone)]
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
}

impl NotificationDatabase {
    /// Create a new database handler
    pub fn new(db_path: &str) -> Self {
        Self {
            db_path: db_path.to_string(),
        }
    }

    /// Connect to the database
    pub async fn connect(&self) -> Result<TokioConnection, Box<dyn std::error::Error>> {
        let db_path = self.db_path.clone();
        let conn = tokio_rusqlite::Connection::open(db_path).await?;
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
}
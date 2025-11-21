//! Daemon module for running the notification monitoring daemon.

use crate::database::{NotificationDatabase};
use tokio_rusqlite::Connection as TokioConnection;
use plist::Value;
use std::str;
use tokio::time::{sleep, Duration};

/// The main daemon structure
pub struct NotificationDaemon {
    db: NotificationDatabase,
    last_rowid: Option<i64>,
}

impl NotificationDaemon {
    /// Create a new daemon instance
    pub fn new(db_path: &str) -> Self {
        Self {
            db: NotificationDatabase::new(db_path),
            last_rowid: None,
        }
    }

    /// Start the daemon in continuous monitoring mode
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting macOS notification daemon in monitoring mode...");

        if !self.db.exists() {
            eprintln!("Database file does not exist: {}", self.db.db_path());
            return Err("Database file not found".into());
        }

        println!("Connected to database: {}", self.db.db_path());

        // Start monitoring loop
        self.monitor_notifications().await?;

        Ok(())
    }

    /// Monitor notifications continuously
    async fn monitor_notifications(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Monitoring for new notifications (Ctrl+C to stop)...");

        loop {
            // Check for new notifications
            self.check_for_new_notifications().await?;

            // Wait before next check (5 seconds between checks)
            sleep(Duration::from_secs(5)).await;
        }
    }

    /// Check for new notifications since last check
    async fn check_for_new_notifications(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.db.connect().await?;

        // Get the maximum ROWID to know how far we've checked
        let max_rowid = conn.call(|db_conn| {
            let mut stmt = db_conn.prepare("SELECT MAX(ROWID) FROM record")?;
            let max_rowid: Option<i64> = stmt.query_row([], |row| row.get(0))?;
            Ok(max_rowid)
        }).await?;

        match max_rowid {
            Some(max_id) => {
                // If this is our first run, set the initial rowid
                if self.last_rowid.is_none() {
                    self.last_rowid = Some(max_id);
                    println!("Initialized monitoring from ROWID: {}", max_id);
                    return Ok(());
                }

                let last_rowid = self.last_rowid.unwrap();

                // If there are new records
                if max_id > last_rowid {
                    println!("Found {} new notification(s) since last check", max_id - last_rowid);

                    // Query all new records since last check
                    self.query_new_notifications(&conn, last_rowid).await?;

                    // Update our last checked rowid
                    self.last_rowid = Some(max_id);
                } else {
                    // No new records since last check
                    println!("No new notifications since last check");
                }
            }
            None => {
                println!("No records found in the record table");
            }
        }

        Ok(())
    }

    /// Query new notifications since last check
    async fn query_new_notifications(&self, conn: &TokioConnection, last_rowid: i64) -> Result<(), Box<dyn std::error::Error>> {
        // Query all new records since last checked ROWID
        let new_records = conn.call(move |db_conn| {
            let mut stmt = db_conn.prepare("SELECT ROWID, data FROM record WHERE ROWID > ? ORDER BY ROWID ASC")?;
            let mut rows = stmt.query([last_rowid])?;

            let mut records = Vec::new();
            while let Some(row) = rows.next()? {
                let rowid: i64 = row.get(0)?;
                let data_bytes: Vec<u8> = row.get(1)?;
                records.push((rowid, data_bytes));
            }

            Ok(records)
        }).await?;

        // Process each new record
        for (rowid, bytes) in new_records {
            println!("Processing notification from ROWID: {}", rowid);

            // Try to parse as binary plist
            match plist::from_bytes::<Value>(&bytes) {
                Ok(plist_value) => {
                    println!("  Parsed notification data:");
                    println!("  {:#?}", plist_value);
                }
                Err(e) => {
                    println!("  Failed to parse as binary plist: {}", e);
                    // If it's not a plist, show raw data
                    let hex_string = hex::encode(&bytes);
                    println!("  Raw hex data: {}", hex_string);
                }
            }
        }

        Ok(())
    }
}
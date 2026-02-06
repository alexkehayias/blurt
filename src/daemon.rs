//! Daemon module for running the notification monitoring daemon.

use tokio_rusqlite::Connection as TokioConnection;
use plist::Value;
use tokio::time::{sleep, Duration};
use serde_json;
use crate::database::{NotificationDatabase, Notification};

#[cfg(feature = "webhook")]
use reqwest::Client;

/// The main daemon structure
pub struct NotificationDaemon {
    db: NotificationDatabase,
    pub last_rowid: Option<i64>,
    #[cfg(feature = "webhook")]
    webhook_url: Option<String>,
}

impl NotificationDaemon {
    /// Create a new daemon instance
    pub fn new(db_path: &str) -> Self {
        Self {
            db: NotificationDatabase::new(db_path),
            last_rowid: None,
            #[cfg(feature = "webhook")]
            webhook_url: None,
        }
    }

    /// Create a new daemon instance with a webhook URL
    #[cfg(feature = "webhook")]
    pub fn with_webhook(db_path: &str, webhook_url: String) -> Self {
        Self {
            db: NotificationDatabase::new(db_path),
            last_rowid: None,
            webhook_url: Some(webhook_url),
        }
    }

    /// Start the daemon in continuous monitoring mode
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.db.exists() {
            eprintln!("Database file does not exist: {}", self.db.db_path());
            return Err("Database file not found".into());
        }

        // Start monitoring loop
        self.monitor_notifications().await?;

        Ok(())
    }

    /// Monitor notifications continuously
    async fn monitor_notifications(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            // Check for new notifications
            self.check_for_new_notifications().await?;

            // Wait before next check (5 seconds between checks)
            sleep(Duration::from_secs(5)).await;
        }
    }

    /// Check for new notifications since last check
    ///
    /// The max ROWID always goes up but the last ROWID can change
    /// to a lower number. This happens when rows are deleted when
    /// a user dismisses notifications.
    ///
    /// The algorithm for detecting new notifications is to hold on
    /// to the last observed max_id and comparing to the current
    /// max_id. If they don't match, query for everything above the
    /// current max ID.
    pub async fn check_for_new_notifications(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
                    return Ok(());
                }

                let last_rowid = self.last_rowid.unwrap();

                // If there are new records
                if max_id > last_rowid {
                    let new_max_rowid = self.query_new_notifications(&conn, last_rowid).await?;
                    self.last_rowid = Some(new_max_rowid);
                }
                // The user dismissed some notices so the ROWID is now lower
                if max_id < last_rowid {
                    let new_max_rowid = self.query_new_notifications(&conn, max_id).await?;
                    self.last_rowid = Some(new_max_rowid);
                }
            }
            None => {
                // No records found, nothing to do
            }
        }

        Ok(())
    }

    /// Query new notifications since last check
    pub async fn query_new_notifications(&self, conn: &TokioConnection, last_rowid: i64) -> Result<i64, Box<dyn std::error::Error>> {
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

        // Track the actual maximum ROWID we retrieved
        let mut actual_max_rowid = last_rowid;

        // Process each new record
        for (rowid, bytes) in &new_records {
            // Update the maximum ROWID seen
            actual_max_rowid = *rowid;

            // Try to parse as binary plist
            match plist::from_bytes::<Value>(bytes) {
                Ok(plist_value) => {
                    // Parse the plist into our Notification struct
                    if let Some(notification) = parse_notification_from_plist(&plist_value, *rowid) {
                        // Either forward the notification to the
                        // webhook OR print json to stdout
                        if cfg!(feature = "webhook") {
                            #[cfg(feature = "webhook")]
                            if let Some(webhook_url) = &self.webhook_url {
                                if let Err(e) = forward_to_webhook(webhook_url, &notification).await {
                                    eprintln!("Failed to forward notification: {}", e);
                                }
                            }
                        } else {
                            println!(r"{}", serde_json::to_string(&notification).unwrap());
                        }
                    } else {
                        eprintln!("Failed to parse notification data into structured format");
                    }
                }
                Err(e) => {
                    eprintln!("Failed to parse as binary plist: {}", e);
                }
            }
        }

        Ok(actual_max_rowid)
    }
}

/// Parse a plist Value into a Notification struct
fn parse_notification_from_plist(plist_value: &Value, rowid: i64) -> Option<Notification> {
    // Try to extract a dictionary from the plist value
    match plist_value {
        Value::Dictionary(dict) => {
            // Extract fields from the main dictionary
            let mut title = String::new();
            let mut subtitle: Option<String> = None;
            let mut body = String::new();
            let mut date = 0i64;
            let mut bundle_id: Option<String> = None;

            // Extract bundle ID from the main dictionary (app field)
            if let Some(bundle_id_value) = dict.get("app")
                && let Some(bundle_id_str) = bundle_id_value.as_string() {
                    bundle_id = Some(bundle_id_str.to_string());
                }

            // Extract date from the main dictionary (date field)
            if let Some(date_value) = dict.get("date") {
                // Extract as f64 first, then convert to i64
                if let Some(date_num) = date_value.as_real() {
                    date = date_num as i64;
                }
            }

            // Look for the nested request dictionary that contains notification details
            if let Some(req_value) = dict.get("req")
                && let Value::Dictionary(req_dict) = req_value {
                    // Extract title from nested req dictionary (field "titl")
                    if let Some(title_value) = req_dict.get("titl")
                        && let Some(title_str) = title_value.as_string() {
                            title = title_str.to_string();
                        }

                    // Extract subtitle from nested req dictionary (field "subt")
                    if let Some(subtitle_value) = req_dict.get("subt")
                        && let Some(subtitle_str) = subtitle_value.as_string() {
                            subtitle = Some(subtitle_str.to_string());
                        }

                    // Extract body from nested req dictionary (field "body")
                    if let Some(body_value) = req_dict.get("body")
                        && let Some(body_str) = body_value.as_string() {
                            body = body_str.to_string();
                        }
                }

            // Create and return the Notification struct
            Some(Notification {
                id: rowid,
                title,
                subtitle,
                body,
                date,
                bundle_id,
            })
        }
        _ => None
    }
}

/// Forward a notification to a webhook URL via HTTP POST
#[cfg(feature = "webhook")]
async fn forward_to_webhook(webhook_url: &str, notification: &Notification) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    client.post(webhook_url)
        .timeout(Duration::from_secs(5))
        .json(notification)
        .timeout
        .send()
        .await?;

    Ok(())
}

//! End-to-end tests for the notification daemon.
//!
//! These tests use SQLite databases to simulate the macOS notification
//! database and verify that the daemon correctly detects and processes
//! new notifications.

use blurt::daemon::NotificationDaemon;
use tempfile::TempDir;

/// Helper function to create a test database with the notification schema
async fn create_test_database() -> (tempfile::TempDir, blurt::database::NotificationDatabase) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("notifications.db");
    // Use read-write mode for tests (notifications need to be inserted)
    let db = blurt::database::NotificationDatabase::new_with_mode(db_path.to_str().unwrap(), false);

    // Initialize the schema
    db.init_schema().await.unwrap();

    (temp_dir, db)
}

/// Helper function to create a binary plist notification data
fn create_test_plist_data(title: &str, body: &str, bundle_id: &str, date: f64) -> Vec<u8> {
    use plist::Value;

    let mut req_dict = plist::Dictionary::new();
    req_dict.insert("titl".to_string(), Value::String(title.to_string()));
    req_dict.insert("body".to_string(), Value::String(body.to_string()));

    let mut main_dict = plist::Dictionary::new();
    main_dict.insert("req".to_string(), Value::Dictionary(req_dict));
    main_dict.insert("app".to_string(), Value::String(bundle_id.to_string()));
    main_dict.insert("date".to_string(), Value::Real(date));

    let mut buffer = Vec::new();
    plist::to_writer_binary(&mut buffer, &Value::Dictionary(main_dict)).unwrap();
    buffer
}

/// Helper function to create a notification record in the database
async fn insert_notification(
    db: &blurt::database::NotificationDatabase,
    rec_id: i64,
    app_id: i64,
    title: &str,
    body: &str,
    bundle_id: &str,
    date: f64,
) {
    let uuid = vec![0u8; 16]; // Dummy UUID
    let data = create_test_plist_data(title, body, bundle_id, date);

    db.connect().await.unwrap()
        .call(move |db_conn| {
            db_conn.execute(
                "INSERT INTO record (rec_id, app_id, uuid, data, request_date, request_last_date,
                  delivered_date, presented, style, snooze_fire_date)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    rec_id,
                    app_id,
                    uuid,
                    data,
                    date,      // request_date
                    date,      // request_last_date
                    date,      // delivered_date
                    true,      // presented
                    0i32,      // style
                    0.0f64     // snooze_fire_date
                ],
            )?;
            Ok(())
        }).await.unwrap();
}

#[tokio::test]
async fn test_notification_plist_parsing() {
    let (_temp_dir, db) = create_test_database().await;

    // Insert a notification with specific values
    let expected_id = 1;
    let expected_app_id = 42;
    let expected_title = "Test Title";
    let expected_body = "Test Body";
    let expected_bundle_id = "com.test.app";
    let expected_date = 1234567890.5;

    insert_notification(
        &db,
        expected_id,
        expected_app_id,
        expected_title,
        expected_body,
        expected_bundle_id,
        expected_date,
    ).await;

    // Retrieve and verify the notification data
    let conn = db.connect().await.unwrap();
    let result = conn.call(move |db_conn| {
        let mut stmt = db_conn.prepare("SELECT rec_id, app_id, data FROM record WHERE rec_id = ?")?;
        let row = stmt.query_row([&expected_id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, Vec<u8>>(2)?
            ))
        })?;
        Ok(row)
    }).await.unwrap();

    assert_eq!(result.0, expected_id);
    assert_eq!(result.1, expected_app_id);

    // Parse the plist data to verify it was stored correctly
    let plist_value = plist::from_bytes::<plist::Value>(&result.2).unwrap();
    match &plist_value {
        plist::Value::Dictionary(dict) => {
            assert!(dict.contains_key("app"));
            assert!(dict.contains_key("date"));
            assert!(dict.contains_key("req"));
        }
        _ => panic!("Expected dictionary"),
    }
}

#[tokio::test]
async fn test_daemon_integration_with_mock_db() {
    // This test verifies the daemon's check_for_new_notifications logic
    // by directly calling it on a database with known state

    let (temp_dir, db) = create_test_database().await;
    let db_path = temp_dir.path().join("notifications.db").to_str().unwrap().to_string();

    // Insert initial notification
    insert_notification(&db, 1, 1, "Initial Notification", "Initial message", "com.example.testapp", 1234567890.0).await;

    // Create daemon using the same database path
    let mut daemon = NotificationDaemon::new(&db_path);

    // First check should set initial rowid
    daemon.check_for_new_notifications().await.unwrap();
    assert_eq!(daemon.last_rowid, Some(1));

    // Insert a new notification
    insert_notification(&db, 2, 1, "New Notification", "New message", "com.example.testapp", 1234567891.0).await;

    // Second check should detect the new notification
    daemon.check_for_new_notifications().await.unwrap();
    assert_eq!(daemon.last_rowid, Some(2));
}

#[tokio::test]
async fn test_daemon_deletion_scenario() {
    // Test the scenario where notifications are deleted and a new one is inserted
    // This tests the daemon's ability to handle ROWID decrease

    let (temp_dir, db) = create_test_database().await;
    let db_path = temp_dir.path().join("notifications.db").to_str().unwrap().to_string();

    // Insert two notifications
    insert_notification(&db, 1, 1, "First", "Message 1", "com.example.testapp", 1234567890.0).await;
    insert_notification(&db, 2, 1, "Second", "Message 2", "com.example.testapp", 1234567891.0).await;

    // Create daemon and process notifications
    let mut daemon = NotificationDaemon::new(&db_path);

    // First check - sets initial rowid to 2
    daemon.check_for_new_notifications().await.unwrap();
    assert_eq!(daemon.last_rowid, Some(2));

    // Delete all notifications (user dismisses them)
    let conn = db.connect().await.unwrap();
    conn.call(|db_conn| {
        db_conn.execute("DELETE FROM record", [])?;
        Ok(())
    }).await.unwrap();

    // Insert a new notification (simulating after deletion)
    insert_notification(&db, 1, 1, "Third", "Message 3", "com.example.testapp", 1234567892.0).await;

    // Check for new notifications - should handle the ROWID change correctly
    daemon.check_for_new_notifications().await.unwrap();

    // The last_rowid should be updated to 1
    assert_eq!(daemon.last_rowid, Some(1));
}

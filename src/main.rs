//! Main entry point for the macOS notification daemon.

use mattdaemon::daemon::NotificationDaemon;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Default path to the macOS notification database using fully qualified path
    let db_path = "~/Library/Group Containers/group.com.apple.usernoted/db2/db";

    // Allow override via command line argument
    let db_path = match env::args().nth(1) {
        Some(path) => path,
        None => db_path.to_string(),
    };

    // Expand the path if it contains ~
    let expanded_path = if db_path.starts_with("~/") {
        let home_dir = std::env::var("HOME").unwrap();
        format!("{}/{}", home_dir, &db_path[2..])
    } else {
        db_path
    };

    let mut daemon = NotificationDaemon::new(&expanded_path);

    // Start the daemon
    daemon.start().await?;

    Ok(())
}

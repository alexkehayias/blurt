//! Main entry point for the macOS notification daemon.

use blurt::daemon::NotificationDaemon;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    // Path to the macOS notification database
    let db_path = "~/Library/Group Containers/group.com.apple.usernoted/db2/db";

    // Expand the path if it contains ~
    let expanded_path = if db_path.starts_with("~/") {
        let home_dir = std::env::var("HOME").unwrap();
        format!("{}/{}", home_dir, &db_path[2..])
    } else {
        db_path.to_string()
    };

    let mut daemon = if args.len() >= 2 {
        #[cfg(feature = "webhook")]
        {
            println!("Webhook enabled: forwarding notifications to {}", args[1]);
            NotificationDaemon::with_webhook(&expanded_path, args[1].clone())
        }
        #[cfg(not(feature = "webhook"))]
        {
            panic!("Webhook feature is not enabled. Rebuild with --features webhook");
        }
    } else {
        NotificationDaemon::new(&expanded_path)
    };

    // Start the daemon
    daemon.start().await?;

    Ok(())
}

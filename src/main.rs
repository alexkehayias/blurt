use blurt::daemon::NotificationDaemon;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    let home_dir = std::env::var("HOME").unwrap();
    let db_path = format!("{}/Library/Group Containers/group.com.apple.usernoted/db2/db", home_dir);

    let mut daemon = if args.len() >= 2 {
        if !cfg!(feature = "webhook") {
            panic!("Webhook feature was enabled but no forwarding URL was provided. Rebuild without --features webhook if you want to emit json to stdout.");
        }
        #[cfg(feature = "webhook")]
        {
            NotificationDaemon::with_webhook(&db_path, args[1].clone())
        }
        #[cfg(not(feature = "webhook"))]
        {
            panic!("Webhook feature is not enabled. Rebuild with --features webhook");
        }
    } else {
        NotificationDaemon::new(&db_path)
    };

    daemon.start().await?;

    Ok(())
}

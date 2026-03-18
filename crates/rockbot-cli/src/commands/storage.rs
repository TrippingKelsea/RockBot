use anyhow::Result;
use rockbot_storage_runtime::StorageRuntime;
use std::path::PathBuf;

use crate::{load_config, StorageCommands};

pub async fn run(command: &StorageCommands, config_path: &PathBuf) -> Result<()> {
    match command {
        StorageCommands::Plan { config } => {
            let config_path = config.as_ref().unwrap_or(config_path);
            let cfg = load_config(config_path).await?;
            let runtime = StorageRuntime::new(config_path, &cfg).await?;
            let plan = runtime.plan()?;

            println!("Storage root: {}", plan.storage_root.display());
            println!("Virtual disk: {}", plan.disk_path.display());
            println!();
            println!("Store plan:");
            for store in plan.stores {
                println!("- {}: {:?} ({})", store.label, store.resolution, store.descriptor);
            }

            Ok(())
        }
        StorageCommands::Repair { config } => {
            let config_path = config.as_ref().unwrap_or(config_path);
            let cfg = load_config(config_path).await?;
            let runtime = StorageRuntime::new(config_path, &cfg).await?;

            let _ = runtime.open_vault_volume_sync(&cfg.credentials.vault_path)?;
            let _ = runtime.open_sessions_store().await?;
            let _ = runtime.open_cron_store().await?;
            let _ = runtime.open_agents_store(&cfg.credentials.vault_path).await?;
            let _ = runtime.open_routing_store().await;

            println!("Storage repair/import pass completed.");
            println!("Use `rockbot storage plan` to review the resolved store sources.");
            Ok(())
        }
    }
}

use std::sync::Arc;

use cache::SqliteCache;
use clap::Parser;
use dev_vault::{op::WrapContext, ExecContext, PrintState};
use interactor::TermInteractor;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod adapter;
mod arg;
mod cache;
mod config;
mod interactor;

#[tokio::main]
async fn main() {
    tracing_subscriber::Registry::default()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().with_thread_ids(true))
        // .with(tracing_subscriber::fmt::layer().pretty())
        .init();

    let args = arg::Cli::parse();
    if let arg::Which::FullConfig { extension } = args.command {
        println!(
            "{}",
            config::example(
                xcfg::Format::match_ext(&extension.unwrap_or_else(|| "toml".to_string()))
                    .expect("can't match extension")
            )
        );
        return;
    }
    match args.command {
        arg::Which::Exec { dry_run, plan_id } => {
            let dir = args
                .directory
                .canonicalize()
                .expect("can't canonicalize directory");
            let cfg_path = dir.join("config");
            let config = config::Config::new(&cfg_path, dry_run)
                .map_err(|e| format!("can't load config {}: {}", cfg_path.display(), e))
                .unwrap();
            let interactor = TermInteractor::new();
            let (dm, plans) = config.cast(dir.clone(), plan_id.as_deref()).await;
            dm.print(&interactor).await;
            let cache = SqliteCache::new(&dir.join("cache.db"));
            let context = Arc::new(ExecContext::new(dm, cache, interactor).await.wrap());

            for plan in plans {
                plan.run(context.clone()).await;
            }
            info!("[Over ] All plan over");
        }
        arg::Which::FullConfig { .. } => unreachable!(),
    }
}

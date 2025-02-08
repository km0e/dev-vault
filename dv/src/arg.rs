use clap::{Parser, Subcommand};
use std::path::PathBuf;

fn default_config() -> PathBuf {
    home::home_dir()
        .expect("can't find home directory")
        .join(".config/dv/")
}

#[derive(Parser, Debug)]
#[command(version = env!("CARGO_PKG_VERSION"), about = "Simple CLI to show how to use xcfg")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Which,
    #[arg(short, long, default_value_os_t = default_config())]
    pub directory: PathBuf,
    #[arg(short, long)]
    pub config: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Which {
    #[command(visible_alias = "fc", about = "Print full config")]
    FullConfig { extension: Option<String> },
    #[command(visible_alias = "e", about = "Execute plan")]
    Exec {
        #[arg(short = 'n', long, default_value = "false")]
        dry_run: bool,
        plan_id: Option<String>,
    },
}

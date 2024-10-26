use clap::{Parser, Subcommand};
use std::path::PathBuf;

fn default_config() -> PathBuf {
    home::home_dir()
        .expect("can't find home directory")
        .join(".config/rbin/config")
}

#[derive(Parser, Debug)]
#[command(version = "0.1", about = "Simple CLI to show how to use xcfg")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Which,
    #[arg(short, long, default_value_os_t = default_config())]
    pub config: PathBuf,
}

#[derive(Subcommand, Debug)]
pub enum Which {
    #[command(visible_alias = "fc", about = "Print full config")]
    FullConfig { extension: Option<String> },
    #[command(visible_alias = "n", about = "Print name")]
    Name,
    #[command(visible_alias = "a", about = "Print age")]
    Age,
}

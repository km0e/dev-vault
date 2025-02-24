use clap::Parser;
use std::path::PathBuf;

fn default_config() -> PathBuf {
    home::home_dir()
        .expect("can't find home directory")
        .join(".config/dv/")
}

#[derive(Parser, Debug)]
#[command(version = env!("CARGO_PKG_VERSION"), about = "Simple CLI to use dv-api with rune")]
pub struct Cli {
    #[arg(short, long, default_value_os_t = default_config())]
    pub directory: PathBuf,
    #[arg(short, long)]
    pub config: Option<PathBuf>,
    #[arg(short = 'b', long, help = "default is $directory/.cache")]
    pub dbpath: Option<PathBuf>,
    #[arg(short = 'n', long, default_value = "false")]
    pub dry_run: bool,
    #[arg(default_value = "main")]
    pub entry: String,
    #[arg(trailing_var_arg = true)]
    pub rargs: Vec<String>,
}

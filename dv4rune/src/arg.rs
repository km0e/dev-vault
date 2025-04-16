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
    #[arg(short, long, help = "The config file to use")]
    pub config: Option<PathBuf>,
    #[arg(short = 'b', long, help = "Default is $directory/.cache")]
    pub dbpath: Option<PathBuf>,
    #[arg(
        short = 'n',
        long,
        default_value = "false",
        help = "Do not actually modify anything"
    )]
    pub dry_run: bool,
    #[arg(
        long,
        default_value = "false",
        help = "Run the script directly without running the __build.rn script"
    )]
    pub direct_run: bool,
    #[arg(default_value = "main", help = "The entry point of the script")]
    pub entry: String,
    #[arg(trailing_var_arg = true, help = "Arguments to pass to the entry point")]
    pub rargs: Vec<String>,
}

use clap::Parser;
use xcfg::XCfg;

mod arg;
mod config;
fn main() {
    let args = arg::Cli::parse();
    if let arg::Which::FullConfig { extension } = args.command {
        let config = config::Config::default();
        println!(
            "{}",
            config
                .fmt_to_string(
                    xcfg::Format::match_ext(&extension.unwrap_or_else(|| "toml".to_string()))
                        .expect("can't match extension")
                )
                .expect("can't format config")
        );
        return;
    }
    let config = config::Config::load(&args.config)
        .expect("can't load config")
        .into_inner();
    match args.command {
        arg::Which::Name => println!("Name: {}", config.name),
        arg::Which::Age => println!("Age: {}", config.age),
        _ => unreachable!(),
    }
}

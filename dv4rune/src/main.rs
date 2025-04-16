mod arg;
mod cache;
mod dv;
mod interactor;
mod multi;
mod utils;
mod error {
    pub use dv_api::error::*;
}
use clap::Parser;
use rune::{
    Diagnostics, Vm,
    termcolor::{ColorChoice, StandardStream},
    to_value,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use std::sync::Arc;

#[tokio::main]
async fn main() -> rune::support::Result<()> {
    tracing_subscriber::Registry::default()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().with_thread_ids(true))
        // .with(tracing_subscriber::fmt::layer().pretty())
        .init();

    let args = arg::Cli::parse();
    let dbpath = args.dbpath.unwrap_or_else(|| args.directory.join(".cache"));
    let m = dv::module()?;

    let mut context = rune_modules::default_context()?;
    context.install(m)?;
    let runtime = Arc::new(context.runtime()?);

    let build_path = args.directory.join("__build.rn");

    let mut diagnostics = Diagnostics::new();
    let mut sources = if !args.direct_run && build_path.exists() {
        let mut sources = rune::Sources::new();
        sources.insert(rune::Source::from_path(build_path)?)?;

        let result = rune::prepare(&mut sources)
            .with_context(&context)
            .with_diagnostics(&mut diagnostics)
            .build();

        if !diagnostics.is_empty() {
            let mut writer = StandardStream::stderr(ColorChoice::Always);
            diagnostics.emit(&mut writer, &sources)?;
        }

        let unit = result?;

        let mut vm = Vm::new(runtime.clone(), Arc::new(unit));
        let res = vm
            .execute(
                ["main"],
                (rune::to_value(dv::Dv::new(&dbpath, args.dry_run))?,),
            )?
            .async_complete()
            .await
            .into_result()?;
        let res: rune::support::Result<Vec<String>> = rune::from_value(res)?;
        let mut sources = rune::Sources::new();
        for s in res? {
            sources.insert(rune::Source::from_path(&s)?)?;
        }
        sources
    } else {
        rune::Sources::new()
    };
    sources.insert(rune::Source::from_path(
        args.config
            .unwrap_or_else(|| args.directory.join("config.rn")),
    )?)?;

    let rargs = args
        .rargs
        .into_iter()
        .map(to_value)
        .collect::<Result<Vec<_>, _>>()?;

    let result = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build();

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        diagnostics.emit(&mut writer, &sources)?;
    }

    let unit = result?;

    let mut vm = Vm::new(runtime.clone(), Arc::new(unit));
    let dv = rune::to_value(dv::Dv::new(dbpath, args.dry_run))?;

    let output = vm
        .execute(
            [args.entry.as_str()],
            std::iter::once(dv).chain(rargs).collect::<Vec<_>>(),
        )?
        .async_complete()
        .await
        .into_result()?;
    rune::from_value(output)?
}

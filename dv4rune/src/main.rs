mod arg;
mod cache;
mod dvl;
mod interactor;
mod multi;
mod utils;

use clap::Parser;
use rune::{
    termcolor::{ColorChoice, StandardStream},
    to_value, ContextError, Diagnostics, Module, Vm,
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

    let m = module()?;

    let mut context = rune_modules::default_context()?;
    context.install(m)?;
    let runtime = Arc::new(context.runtime()?);

    let mut sources = rune::Sources::new();
    sources.insert(rune::Source::from_path(
        args.config
            .unwrap_or_else(|| args.directory.join("config.rn")),
    )?)?;

    let mut diagnostics = Diagnostics::new();

    let result = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build();

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        diagnostics.emit(&mut writer, &sources)?;
    }

    let unit = result?;

    let mut vm = Vm::new(runtime, Arc::new(unit));

    let rargs = args
        .rargs
        .into_iter()
        .map(to_value)
        .collect::<Result<Vec<_>, _>>()?;

    let output = vm
        .execute(
            [args.entry.as_str()],
            std::iter::once(rune::to_value(dvl::Dv::new(
                args.directory.join(".cache"),
                args.dry_run,
            ))?)
            .chain(rargs)
            .collect::<Vec<_>>(),
        )?
        .async_complete()
        .await
        .into_result()?;
    let _: () = rune::from_value(output)?;

    Ok(())
}

fn module() -> Result<Module, ContextError> {
    let mut m = Module::default();
    m.ty::<dvl::Dv>()?;
    m.function_meta(dvl::Dv::add_current)?;
    m.function_meta(dvl::Dv::add_ssh_user)?;
    m.function_meta(dvl::Dv::copy)?;
    m.function_meta(dvl::Dv::app)?;
    m.function_meta(dvl::Dv::auto)?;
    m.function_meta(dvl::Dv::exec)?;
    Ok(m)
}

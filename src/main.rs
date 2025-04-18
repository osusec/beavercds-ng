use beavercds_ng::commands;
use clap::Parser;
use tracing::{trace, Level};
use tracing_subscriber::{fmt::time, EnvFilter};

mod cli;

fn main() {
    let cli = cli::Cli::parse();

    // number of 'v' flags influences our crate's log level, all other log levels, and whether or
    // not we display the span stack, respectively
    let (brcds_level, dep_level, display_spans) = match cli.verbosity {
        0 => (Level::INFO, Level::WARN, false),
        1 => (Level::DEBUG, Level::INFO, false),
        2 => (Level::TRACE, Level::DEBUG, true),
        _ => (Level::TRACE, Level::TRACE, true),
    };

    // Use RUST_LOG env variable to set log levels if it's set
    // Otherwise we use the above levels. Span-stack display always influenced by -v
    let timer = time::ChronoLocal::new("%H:%M:%S".to_owned());
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            format!("{}={brcds_level},{dep_level}", env!("CARGO_CRATE_NAME")).into()
        }))
        .with_timer(timer)
        .with_target(display_spans)
        .init();

    trace!("args: {:?}", cli);

    // dispatch commands
    match &cli.command {
        cli::Commands::Validate => commands::validate::run(),

        cli::Commands::CheckAccess {
            profile,
            kubernetes,
            frontend,
            registry,
            bucket,
        } => {
            commands::validate::run();
            commands::check_access::run(profile, kubernetes, frontend, registry, bucket)
        }

        #[allow(unused_variables)]
        cli::Commands::Build {
            profile,
            push,
            no_push,
            extract_assets,
        } => {
            commands::validate::run();
            commands::build::run(profile, &!no_push, extract_assets)
        }

        cli::Commands::Deploy {
            profile,
            no_build,
            dry_run,
        } => {
            commands::validate::run();
            commands::deploy::run(profile, no_build, dry_run)
        }

        cli::Commands::ClusterSetup { profile } => {
            commands::cluster_setup::run(profile);
        }
    }
}

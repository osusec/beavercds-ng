use beavercds_ng::commands;
use clap::Parser;
use tracing::trace;
use tracing_subscriber::EnvFilter;

mod cli;

fn main() {
    let cli = cli::Cli::parse();

    // Use RUST_LOG env variable if it's set
    // Otherwise our tracing is filtered by -q|-v* flag, all others always INFO and above
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            format!(
                "{}={},INFO",
                env!("CARGO_CRATE_NAME"),
                cli.verbose.log_level_filter()
            )
            .into()
        }))
        .without_time()
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

use beavercds_ng::commands;
use clap::Parser;
use simplelog::*;

mod cli;

fn main() {
    let cli = cli::Cli::parse();

    let log_config = ConfigBuilder::new()
        .set_time_level(LevelFilter::Off)
        .build();

    TermLogger::init(
        cli.verbose.log_level_filter(),
        log_config,
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();

    debug!("args: {:?}", cli);

    // dispatch commands
    match &cli.command {
        cli::Commands::Validate => commands::validate::run(),

        cli::Commands::CheckAccess {
            kubernetes,
            frontend,
            registry,
        } => commands::check_access::run(kubernetes, frontend, registry),

        cli::Commands::Build { profile, push } => commands::build::run(profile, push),

        cli::Commands::Deploy {
            profile,
            no_build,
            dry_run,
        } => commands::deploy::run(profile, no_build, dry_run),
    }
}

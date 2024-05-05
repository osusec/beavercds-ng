use clap::Parser;
use simplelog::*;

mod commands;
mod lib {
    pub mod build;
    pub mod check_access;
    pub mod configparser;
    pub mod deploy;
    pub mod validate;
}

fn main() {
    println!();
    println!();

    let cli = commands::Cli::parse();

    setup_logging(cli.verbose);

    debug!("args: {:?}", cli);

    // dispatch commands
    match &cli.command {
        commands::Commands::Validate {} => lib::validate::run(),

        commands::Commands::CheckAccess {
            kubernetes,
            frontend,
            registry,
        } => lib::check_access::run(kubernetes, frontend, registry),

        commands::Commands::Build { profile, push } => lib::build::run(profile, push),

        commands::Commands::Deploy {
            profile,
            no_build,
            dry_run,
        } => lib::deploy::run(profile, no_build, dry_run),
    }
}

fn setup_logging(verbose: bool) {
    let log_level = match verbose {
        true => LevelFilter::Debug,
        _ => LevelFilter::Info,
    };

    let log_config = ConfigBuilder::new()
        .set_time_level(LevelFilter::Trace)
        .build();

    TermLogger::init(
        log_level,
        log_config,
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();
}

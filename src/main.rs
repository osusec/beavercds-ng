use clap::Parser;

mod commands;
mod lib {
    pub mod build;
    pub mod check_access;
    pub mod deploy;
    pub mod validate;
}

fn main() {
    let cli = commands::Cli::parse();

    println!("args: {:?}", cli);

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

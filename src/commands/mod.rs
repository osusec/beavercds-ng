use clap::{Parser, Subcommand};

#[derive(Parser)]
/// Deployment manager for rCTF/beaverCTF challenges deployed on Kubernetes.
pub struct Cli {
    #[arg(short, global = true, help = "Show verbose output")]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Build all challenge container images, optionally pushing them to the configured registry.
    ///
    /// Images are tagged as <registry>/<chal>-<container>:<profile>.
    Build {
        #[arg(long, value_name = "PROFILE", help = "deployment profile")]
        profile: String,

        #[arg(long, help = "Whether to push container images to registry")]
        push: bool,
    },

    /// Deploy enabled challenges to cluster, updating any backing resources as necessary.
    ///
    /// Also builds and pushes images to registry, unless --no-build is specified.
    Deploy {
        #[arg(long, value_name = "PROFILE", help = "deployment profile")]
        profile: String,

        #[arg(long, help = "Whether to not build/deploy challenge images")]
        no_build: bool,

        #[arg(long, help = "Test changes without actually applying")]
        dry_run: bool,
    },

    /// Validate contents of rcds.yaml and any challenge.yaml files.
    Validate,

    /// Checks access to various frontend/backend components.
    CheckAccess {
        #[arg(long, help = "Check Kubernetes cluster access")]
        kubernetes: bool,

        #[arg(long, help = "Check frontend (rCTF) status")]
        frontend: bool,

        #[arg(long, help = "Check container registry access/permissions")]
        registry: bool,
    },
}

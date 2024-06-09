use clap::{Parser, Subcommand};
use clap_verbosity_flag::{InfoLevel, Verbosity};

#[derive(Parser, Debug)]
/// Deployment manager for rCTF/beaverCTF challenges deployed on Kubernetes.
pub struct Cli {
    #[command(flatten)]
    pub verbose: Verbosity<InfoLevel>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Build all challenge container images, optionally pushing them to the configured registry.
    ///
    /// Images are tagged as <registry>/<chal>-<container>:<profile>.
    Build {
        #[arg(short, long, value_name = "PROFILE", help = "deployment profile")]
        profile: String,

        #[arg(
            long,
            help = "Whether to push container images to registry (default: true)",
            default_value = "true"
        )]
        push: bool,
    },

    /// Deploy enabled challenges to cluster, updating any backing resources as necessary.
    ///
    /// Also builds and pushes images to registry, unless --no-build is specified.
    Deploy {
        #[arg(short, long, value_name = "PROFILE", help = "deployment profile")]
        profile: String,

        #[arg(long, help = "Whether to not build/deploy challenge images")]
        no_build: bool,

        #[arg(short = 'n', long, help = "Test changes without actually applying")]
        dry_run: bool,
    },

    /// Validate contents of rcds.yaml and any challenge.yaml files.
    Validate, // no args

    /// Checks access to various frontend/backend components.
    CheckAccess {
        #[arg(
            short,
            long,
            value_name = "PROFILE",
            help = "deployment profile to check",
            default_value = "all"
        )]
        profile: String,

        #[arg(short, long, help = "Check Kubernetes cluster access")]
        kubernetes: bool,

        #[arg(short, long, help = "Check frontend (rCTF) access")]
        frontend: bool,

        #[arg(short, long, help = "Check container registry access and permissions")]
        registry: bool,
    },
}

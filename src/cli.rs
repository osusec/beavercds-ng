use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
/// Deployment manager for rCTF/beaverCTF challenges deployed on Kubernetes.
pub struct Cli {
    /// Increase output verbosity. One usage increases slightly. Two increases significantly. Three or
    /// more maximize output.
    #[arg(short = 'v', action = clap::ArgAction::Count, global = true)]
    pub verbosity: u8,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Build all challenge container images, optionally pushing them to the
    /// configured registry and extracting build artifacts.
    ///
    /// Images are tagged as <registry>/<chal>-<container>:<profile>.
    Build {
        /// Deployment profile
        #[arg(short, long, value_name = "PROFILE")]
        profile: String,
        /// Push container images to registry (default: true)
        #[arg(long, default_value = "true")]
        push: bool,

        /// Don't push container images to registry
        // TODO: this is hacky. revisit when automatic negation flags are
        // implemented: https://github.com/clap-rs/clap/issues/815
        #[arg(long, default_value = "false")]
        no_push: bool,

        /// Extract build assets to challenge source directory (default: true)
        #[arg(long, default_value = "true")]
        extract_assets: bool,
    },

    /// Deploy enabled challenges to cluster, updating any backing resources as
    /// necessary.
    ///
    /// Also builds and pushes images to registry, unless --no-build is
    /// specified.
    Deploy {
        /// Deployment profile
        #[arg(short, long, value_name = "PROFILE")]
        profile: String,

        /// Whether to not build/deploy challenge images
        #[arg(long)]
        no_build: bool,

        /// Test changes without actually applying
        #[arg(short = 'n', long)]
        dry_run: bool,
    },

    /// Validate contents of rcds.yaml and any challenge.yaml files.
    Validate, // no args

    /// Check access to various frontend/backend components.
    CheckAccess {
        /// Deployment profile to check
        #[arg(short, long, value_name = "PROFILE", default_value = "all")]
        profile: String,

        /// Check Kubernetes cluster access
        #[arg(short, long)]
        kubernetes: bool,

        /// Check frontend (rCTF) access
        #[arg(short, long)]
        frontend: bool,

        /// Check container registry access and permissions
        #[arg(short, long)]
        registry: bool,

        #[arg(short, long, help = "Check S3 asset bucket access and permissions")]
        bucket: bool,
    },

    /// Set up required cluster resources (ingress, cert-manager, etc)
    ClusterSetup {
        /// Deployment profile to use
        #[arg(short, long, value_name = "PROFILE")]
        profile: String,
    },
}

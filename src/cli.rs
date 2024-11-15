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
        /// Deployment profile
        #[arg(short, long, value_name = "PROFILE")]
        profile: String,
        /// Push container images to registry (default: true)
        #[arg(long, default_value = "true")]
        push: bool,

        /// Don't push container images to registry
        #[arg(long, default_value = "false")]
        no_push: bool,
        // TODO: this is hacky. revisit when automatic negation flags are implemented:
        // https://github.com/clap-rs/clap/issues/815
    },

    /// Deploy enabled challenges to cluster, updating any backing resources as necessary.
    ///
    /// Also builds and pushes images to registry, unless --no-build is specified.
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

    /// Checks access to various frontend/backend components.
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
    },

    /// Copy an initial rcds.yaml to the current working directory.
    ///
    /// If interactive is enabled, then it will prompt for the various fields of the config file. If left disabled, then it will copy it out with fake data of the expected format.
    /// 
    /// If blank is enabled, then it will copy out the file without any fields set. If left disabled, it will write the default non-interactive example config to file.
    Init {
        /// Guided filling out of the config
        #[arg(short = 'i', long)]
        interactive: bool,
        #[arg(short = 'b', long)]
        blank: bool
    }
}

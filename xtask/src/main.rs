use clap::{Parser, Subcommand};
use xtask::{Result, commands};

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Build automation for sntpc crate and examples")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build no-std examples (simple-no-std)
    BuildNostd,
    /// Build Unix-specific examples (all except simple-no-std)
    BuildUnix,
    /// Build cross-platform examples (simple-request, tokio, timesync)
    BuildCrossPlatform,
    /// Build all examples
    BuildAll,
    /// Build the main sntpc crate
    BuildCrate {
        /// Build with all features enabled
        #[arg(long, conflicts_with = "no_default_features")]
        all_features: bool,
        /// Build with no default features
        #[arg(long, conflicts_with = "all_features")]
        no_default_features: bool,
    },
    /// Run tests for the main crate
    Test,
    /// Run tests for the main crate with code coverage
    Coverage,
    /// Check all code (main crate and examples)
    Check,
    /// Run clippy on all code with strict linting
    Clippy,
    /// Check code formatting for the main crate and all examples
    Format {
        /// Check formatting without making changes
        #[arg(long, conflicts_with = "fix")]
        check: bool,
        /// Fix formatting issues
        #[arg(long, conflicts_with = "check")]
        fix: bool,
    },
    /// Clean all build artifacts
    Clean,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::BuildNostd => commands::build::build_nostd_examples(),
        Commands::BuildUnix => commands::build::build_unix_examples(),
        Commands::BuildCrossPlatform => commands::build::build_cross_platform_examples(),
        Commands::BuildAll => commands::build::build_all_examples(),
        Commands::BuildCrate {
            all_features,
            no_default_features,
        } => commands::build::build_main_crate(all_features, no_default_features),
        Commands::Test => commands::test::run_tests(),
        Commands::Coverage => commands::test::run_tests_with_coverage(),
        Commands::Check => commands::check::check_all(),
        Commands::Clean => commands::clean::clean_all(),
        Commands::Clippy => commands::clippy::run_clippy(),
        Commands::Format { check, fix } => {
            if check {
                commands::format::check_formatting()?;
            } else if fix {
                commands::format::fix_formatting()?;
            } else {
                // Default to checking if no flag is provided
                commands::format::check_formatting()?;
            }

            Ok(())
        }
    }
}

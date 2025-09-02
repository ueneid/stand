use clap::Parser;
use stand::cli::commands::{Cli, Commands};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { force } => {
            println!("Init command called with force: {}", force);
            std::process::exit(1); // Temporary - will implement properly
        }
        Commands::Shell { environment } => {
            println!("Shell command called with environment: {}", environment);
            std::process::exit(1); // Temporary - will implement properly
        }
        Commands::Exec {
            environment,
            command,
        } => {
            println!(
                "Exec command called with environment: {} and command: {:?}",
                environment, command
            );
            std::process::exit(1); // Temporary - will implement properly
        }
        Commands::List => {
            println!("List command called");
            std::process::exit(1); // Temporary - will implement properly
        }
        Commands::Show {
            environment,
            values,
        } => {
            println!(
                "Show command called with environment: {} and values: {}",
                environment, values
            );
            std::process::exit(1); // Temporary - will implement properly
        }
        Commands::Switch { environment } => {
            println!("Switch command called with environment: {}", environment);
            std::process::exit(1); // Temporary - will implement properly
        }
        Commands::Set { name, value } => {
            println!(
                "Set command called with name: {} and value: {}",
                name, value
            );
            std::process::exit(1); // Temporary - will implement properly
        }
        Commands::Unset { name } => {
            println!("Unset command called with name: {}", name);
            std::process::exit(1); // Temporary - will implement properly
        }
        Commands::Validate => {
            println!("Validate command called");
            std::process::exit(1); // Temporary - will implement properly
        }
        Commands::Current => {
            println!("Current command called");
            std::process::exit(1); // Temporary - will implement properly
        }
    }
}

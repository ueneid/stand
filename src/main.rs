use clap::Parser;
use stand::cli::commands::{Cli, Commands};
use stand::commands::{current, exec, list, show, validate};

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
            yes,
            command,
        } => {
            let current_dir = std::env::current_dir()?;
            match exec::execute_with_environment(&current_dir, &environment, command, yes) {
                Ok(exit_code) => {
                    std::process::exit(exit_code);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::List => {
            let current_dir = std::env::current_dir()?;
            match list::list_environments(&current_dir) {
                Ok(output) => {
                    println!("{}", output);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Show {
            environment,
            values,
        } => {
            let current_dir = std::env::current_dir()?;
            match show::show_environment(&current_dir, &environment, values) {
                Ok(output) => {
                    println!("{}", output);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
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
            validate::handle_validate()?;
        }
        Commands::Current => {
            current::handle_current()?;
        }
    }

    Ok(())
}

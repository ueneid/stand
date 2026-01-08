use clap::Parser;
use stand::cli::commands::{Cli, Commands};
use stand::commands::{current, env, exec, list, shell, show, validate};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { force } => {
            println!("Init command called with force: {}", force);
            std::process::exit(1); // Temporary - will implement properly
        }
        Commands::Shell {
            environment,
            yes,
            shell: shell_override,
        } => {
            let current_dir = std::env::current_dir()?;
            match shell::start_shell_with_environment(
                &current_dir,
                &environment,
                yes,
                shell_override,
            ) {
                Ok(exit_code) => {
                    std::process::exit(exit_code);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
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
        Commands::Env {
            json,
            stand_only,
            user_only,
        } => {
            let current_dir = std::env::current_dir()?;
            let options = env::EnvOptions {
                json,
                stand_only,
                user_only,
            };
            let output = env::show_env(&current_dir, options)?;
            print!("{}", output);
        }
    }

    Ok(())
}

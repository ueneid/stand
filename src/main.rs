use clap::Parser;
use stand::cli::commands::{Cli, Commands, EncryptCommands};
use stand::commands::{current, encrypt, env, exec, get, init, list, set, shell, show, validate};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init {
            force,
            encrypt: enable_encrypt,
        } => {
            let current_dir = std::env::current_dir()?;
            init::handle_init(&current_dir, force)?;

            // If --encrypt flag is set, also enable encryption
            if enable_encrypt {
                if let Err(e) = encrypt::enable_encryption(&current_dir) {
                    eprintln!("Warning: Failed to enable encryption: {}", e);
                }
            }
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
        Commands::Inspect {
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
        Commands::Set {
            environment,
            key,
            value,
            encrypt: should_encrypt,
        } => {
            let current_dir = std::env::current_dir()?;
            match set::set_variable(&current_dir, &environment, &key, value, should_encrypt) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Get { environment, key } => {
            let current_dir = std::env::current_dir()?;
            match get::get_variable(&current_dir, &environment, &key) {
                Ok(value) => {
                    println!("{}", value);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Encrypt(subcmd) => {
            let current_dir = std::env::current_dir()?;
            match subcmd {
                EncryptCommands::Enable => {
                    if let Err(e) = encrypt::enable_encryption(&current_dir) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                EncryptCommands::Disable => {
                    if let Err(e) = encrypt::disable_encryption(&current_dir) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
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

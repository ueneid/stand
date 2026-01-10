//! # Stand
//!
//! A CLI tool for explicit environment variable management.
//!
//! Stand provides a clean, organized way to handle different environments
//! (dev, staging, prod) with their specific configurations. Unlike tools that
//! automatically load variables on directory entry, Stand gives you explicit
//! control over environment switching.
//!
//! ## Features
//!
//! - **Environment Management**: Define and switch between multiple environments
//! - **Variable Inheritance**: Use `extends` to inherit from other environments
//! - **Variable Interpolation**: Reference system environment variables with `${VAR}`
//! - **Shell Integration**: Start shell sessions with environment loaded
//! - **Command Execution**: Execute commands with specific environment variables
//!
//! ## Installation
//!
//! ```bash
//! cargo install stand
//! ```
//!
//! ## Quick Start
//!
//! ```bash
//! # Initialize a new project
//! stand init
//!
//! # List available environments
//! stand list
//!
//! # Start a shell with an environment
//! stand shell dev
//!
//! # Execute a command with an environment
//! stand exec prod -- npm start
//! ```
//!
//! For more information, see the [GitHub repository](https://github.com/ueneid/stand).

pub mod cli;
pub mod commands;
pub mod config;
pub mod environment;
pub mod error;
pub mod process;
pub mod shell;
pub mod state;
pub mod utils;

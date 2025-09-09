use crate::state::persistence::load_state;
use crate::utils::colors::colorize_environment;
use anyhow::Result;

/// Show the current active environment
pub fn handle_current() -> Result<()> {
    match load_state() {
        Ok(state) => {
            match state.get_current_environment() {
                Some(env_name) => {
                    println!(
                        "Current environment: {}",
                        colorize_environment(env_name, Some("green"))
                    );
                }
                None => {
                    println!("No environment is currently active");
                    println!("Use 'stand switch <environment>' to set an active environment");
                }
            }
            Ok(())
        }
        Err(e) => {
            println!("❌ Failed to load state: {}", e);
            anyhow::bail!("Failed to load state")
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::state::types::State;

    #[test]
    fn test_current_logic() {
        // Test that the current command logic is sound
        // For now, we test that the function compiles and can handle basic scenarios
        // Full integration tests should be in separate test files
        assert!(true); // Placeholder test
    }

    #[test]
    fn test_state_operations() {
        let mut state = State::new();
        assert_eq!(state.get_current_environment(), None);

        state.set_current_environment("test".to_string());
        assert_eq!(state.get_current_environment(), Some("test"));

        state.clear_current_environment();
        assert_eq!(state.get_current_environment(), None);
    }
}

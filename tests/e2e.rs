//! End-to-end tests entry point
//!
//! Tests complete application workflows through the CLI.
//! Run with: cargo test --test e2e

mod e2e {
    pub mod cli_workflows;
    pub mod script_execution;
}

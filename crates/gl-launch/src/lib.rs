//! Full-pipeline game launch orchestration, sitting on top of `gl-java`
//! (JVM argument/classpath construction) and `gl-injection` (process lookup
//! + DLL injection).
//!
//! - [`java`]: Java Edition launch (resolve runtime → build args → spawn).
//! - [`bedrock`]: Bedrock package resolution, activation, and optional
//!   client-DLL injection.

pub mod bedrock;
pub mod error;
pub mod java;

pub use error::{LaunchError, Result};

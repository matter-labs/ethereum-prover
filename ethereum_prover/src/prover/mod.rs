//! Actual implementation of ZKsync OS/Ethereum STF logic.
//! This module is intended to be a "glue" layer between this project needs
//! and the interfaces ZKsync OS provides, making it easier to use ZKsync OS
//! functionality in the context of the Ethereum prover.

pub mod cpu_witness;
pub mod gpu_prover;
pub mod oracle;
pub mod types;

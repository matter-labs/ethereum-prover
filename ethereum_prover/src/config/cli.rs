use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::types::ProofSecurity;

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Cli {
    #[arg(long)]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Block {
        block_number: Option<u64>,
    },
    GenerateVerifierArtifacts {
        #[arg(long, default_value = "../artifacts")]
        output_dir: PathBuf,
        #[arg(long)]
        security: Option<ProofSecurity>,
    },
    Run,
}

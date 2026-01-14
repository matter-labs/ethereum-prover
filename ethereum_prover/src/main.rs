use clap::Parser;
use ethereum_prover::{
    Runner,
    config::{Cli, EthProverConfig},
};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive("ethereum_prover=INFO".parse().unwrap())
                .from_env()
                .unwrap(),
        )
        .init();

    let cli = Cli::parse();
    let config = EthProverConfig::load(&cli.config).expect("Failed to load config");

    let runner = Runner::new();
    runner.run(cli, config).await
}

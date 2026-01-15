use anyhow::Context as _;
use clap::Parser;
use ethereum_prover::{
    Runner,
    config::{Cli, EthProverConfig},
};
use smart_config::value::ExposeSecret;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive("ethereum_prover=INFO".parse().unwrap())
                .from_env()
                .context("failed to load log filter from env")?,
        )
        .init();

    let cli = Cli::parse();
    let config = EthProverConfig::load(&cli.config).context("failed to load config")?;
    let _sentry_guard = init_sentry(&config);

    let runner = Runner::new();
    runner.run(cli, config).await
}

fn init_sentry(config: &EthProverConfig) -> Option<sentry::ClientInitGuard> {
    let dsn = config.sentry_dsn.as_ref()?.expose_secret().to_string();
    let guard = sentry::init((
        dsn,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));
    Some(guard)
}

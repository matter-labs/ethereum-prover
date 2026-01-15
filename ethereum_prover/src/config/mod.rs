use anyhow::Context as _;
use smart_config::{
    ConfigRepository, ConfigSchema, ConfigSources, DescribeConfig, DeserializeConfig, Environment,
    Yaml, de::Serde, value::SecretString,
};
use std::path::PathBuf;

use crate::types::{CachePolicy, EthProofsSubmission, Mode, OnFailure};

mod cli;
pub use cli::{Cli, Command};

/// Ethereum prover configuration.
#[derive(Debug, DescribeConfig, DeserializeConfig)]
#[config(derive(Default))] // derive according to default values for params
pub struct EthProverConfig {
    /// RISC-V executable to use for proving.
    #[config(default_t = "../../artifacts/app.bin".into())]
    pub app_bin_path: PathBuf,

    /// Stages to execute.
    #[config(default_t = Mode::CpuWitness)]
    #[config(with = Serde![str])]
    pub mode: Mode,

    /// Cache policy for prover artifacts.
    #[config(default_t = CachePolicy::OnFailure)]
    #[config(with = Serde![str])]
    pub cache_policy: CachePolicy,

    /// EthProofs submission target.
    #[config(default_t = EthProofsSubmission::Off)]
    #[config(with = Serde![str])]
    pub ethproofs_submission: EthProofsSubmission,

    /// Process only every N-th block.
    #[config(default_t = 1)]
    pub block_mod: u64,

    /// Prover identifier.
    /// Used as an "offset" for matching blocks to process.
    /// Intended use is to enable multiple provers working in parallel.
    #[config(default_t = 0)]
    pub prover_id: u64,

    /// Action to perform on failure.
    #[config(default_t = OnFailure::Exit)]
    #[config(with = Serde![str])]
    pub on_failure: OnFailure,

    /// Ethereum RPC endpoint.
    #[config(default_t = None)]
    pub rpc_url: Option<SecretString>,

    /// EthProofs token.
    #[config(default_t = None)]
    pub ethproofs_token: Option<SecretString>,

    /// EthProofs cluster ID.
    #[config(default_t = None)]
    pub ethproofs_cluster_id: Option<u64>,

    /// Sentry DSN for error reporting.
    #[config(default_t = None)]
    pub sentry_dsn: Option<SecretString>,
}

impl EthProverConfig {
    pub fn schema() -> ConfigSchema {
        let mut schema = ConfigSchema::default();
        schema
            .insert(&Self::DESCRIPTION, "eth_prover")
            .expect("Failed to insert eth_prover config");
        schema
    }

    pub fn load(config_path: &Option<PathBuf>) -> anyhow::Result<Self> {
        let config_schema = Self::schema();
        let mut config_sources = ConfigSources::default();

        // Load YAML config, if provided.
        if let Some(config_path) = config_path {
            let config_contents = std::fs::read_to_string(config_path)
                .with_context(|| format!("failed to read config file {}", config_path.display()))?;
            let filename = config_path
                .file_name()
                .context("config path does not refer to a file")?
                .to_str()
                .context("config filename is not valid UTF-8")?;
            let config_yaml = Yaml::new(filename, serde_yaml::from_str(&config_contents)?)?;
            config_sources.push(config_yaml);
        }

        // If `.env` file exists in the current directory, load it as an environment source.
        if std::fs::exists(".env").context("failed to check .env existence")? {
            let dotenv_contents = std::fs::read_to_string(".env")
                .context("failed to read .env file from the current directory")?;

            let mut dotenv = Environment::from_dotenv(".env", &dotenv_contents)
                .context("failed to load .env")?;
            // Enables JSON coercion - env variables with `__JSON` suffix can be used to force value
            // deserialization as JSON instead of plain string. This is useful to distinguish between "null"
            // and `null` (missing value). Usage example: `GENESIS_BRIDGEHUB_ADDRESS__JSON=null`
            dotenv
                .coerce_json()
                .context("failed to coerce JSON envvar values from .env")?;
            config_sources.push(dotenv);
        }

        // Load environment variables.
        let mut env = Environment::prefixed("");
        // Enables JSON coercion - env variables with `__JSON` suffix can be used to force value
        // deserialization as JSON instead of plain string. This is useful to distinguish between "null"
        env.coerce_json()
            .context("failed to coerce JSON envvar values from environment")?;
        config_sources.push(env);

        let config_repo = ConfigRepository::new(&config_schema).with_all(config_sources);
        let config = config_repo
            .single::<Self>()
            .context("failed to load general config")?
            .parse()
            .context("failed to parse general config")?;

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::EthProverConfig;
    use crate::types::{CachePolicy, Mode, OnFailure};

    #[test]
    fn load_config_from_yaml() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let config_path = temp_dir.path().join("config.yaml");
        let contents = r#"
eth_prover:
  mode: cpu_witness
  cache_policy: off
  block_mod: 10
  prover_id: 2
  on_failure: exit
"#;
        std::fs::write(&config_path, contents).expect("write config");

        let config = EthProverConfig::load(&Some(config_path)).expect("load config");
        assert!(matches!(config.mode, Mode::CpuWitness));
        assert!(matches!(config.cache_policy, CachePolicy::Off));
        assert_eq!(config.block_mod, 10);
        assert_eq!(config.prover_id, 2);
        assert!(matches!(config.on_failure, OnFailure::Exit));
    }
}

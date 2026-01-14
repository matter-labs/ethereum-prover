use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    CpuWitness,
    GpuProve,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum CachePolicy {
    Off,
    OnFailure,
    Always,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum EthProofsSubmission {
    Off,
    Staging,
    Prod,
}

impl EthProofsSubmission {
    pub fn enabled(&self) -> bool {
        match self {
            EthProofsSubmission::Off => false,
            EthProofsSubmission::Staging | EthProofsSubmission::Prod => true,
        }
    }

    pub fn is_staging(&self) -> bool {
        matches!(self, EthProofsSubmission::Staging)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum OnFailure {
    Exit,
    Continue,
}

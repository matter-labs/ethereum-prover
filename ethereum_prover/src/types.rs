use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Copy, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum ProofSecurity {
    #[serde(rename = "security_80")]
    #[value(name = "security_80")]
    Security80,
    #[serde(rename = "security_100")]
    #[value(name = "security_100")]
    Security100,
}

impl ProofSecurity {
    pub fn airbender_security_model(self) -> airbender_verifier_common::SecurityModel {
        match self {
            Self::Security80 => airbender_verifier_common::SecurityModel::Security80,
            Self::Security100 => airbender_verifier_common::SecurityModel::Security100,
        }
    }

    pub fn proof_wire_value(self) -> u8 {
        match self {
            Self::Security80 => 80,
            Self::Security100 => 100,
        }
    }
}

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

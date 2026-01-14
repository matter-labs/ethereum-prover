use anyhow::Context;
use base64::Engine;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::io::Write;

const ETHPROOFS_STAGING_URL: &str = "https://staging--ethproofs.netlify.app/api/v0/";
const ETHPROOFS_PRODUCTION_URL: &str = "https://ethproofs.netlify.app/api/v0/";

#[derive(Clone, Debug)]
pub struct EthproofsClient {
    auth_token: String,
    cluster_id: u64,
    url: String,
    client: reqwest::Client,
}

impl EthproofsClient {
    pub fn new(staging: bool, auth_token: String, cluster_id: u64) -> Self {
        let url = if staging {
            ETHPROOFS_STAGING_URL.to_string()
        } else {
            ETHPROOFS_PRODUCTION_URL.to_string()
        };
        Self {
            auth_token,
            cluster_id,
            url,
            client: reqwest::Client::new(),
        }
    }

    pub async fn queue_proof(&self, block_number: u64) -> anyhow::Result<()> {
        let payload = ProofRequest {
            block_number,
            cluster_id: self.cluster_id,
        };
        let endpoint = format!("{}proofs/queued", self.url);
        self.post(&endpoint, &payload, "ethproofs request update failed")
            .await?;
        Ok(())
    }

    pub async fn proving_proof(&self, block_number: u64) -> anyhow::Result<()> {
        let payload = ProofRequest {
            block_number,
            cluster_id: self.cluster_id,
        };
        let endpoint = format!("{}proofs/proving", self.url);
        self.post(&endpoint, &payload, "ethproofs request update failed")
            .await?;
        Ok(())
    }

    pub async fn send_proof(
        &self,
        block_number: u64,
        proof_bytes: &[u8],
        proving_time_secs: f64,
        cycles: u64,
    ) -> anyhow::Result<()> {
        let encoded_proof = encode_proof(proof_bytes)?;
        let payload = EthProofPayload {
            block_number,
            cluster_id: self.cluster_id,
            proving_time: (proving_time_secs * 1000.0) as u64,
            proving_cycles: cycles,
            proof: encoded_proof,
            verifier_id: "None".to_string(),
        };
        let endpoint = format!("{}proofs/proved", self.url);
        self.post(&endpoint, &payload, "ethproofs submission failed")
            .await?;
        Ok(())
    }

    async fn post<T: Serialize>(
        &self,
        endpoint: &str,
        payload: &T,
        context: &'static str,
    ) -> anyhow::Result<()> {
        self.client
            .post(endpoint)
            .bearer_auth(&self.auth_token)
            .json(payload)
            .send()
            .await
            .context(context)?
            .error_for_status()
            .context(context)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthProofPayload {
    pub block_number: u64,
    pub cluster_id: u64,
    pub proving_time: u64,
    pub proving_cycles: u64,
    pub proof: String,
    pub verifier_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofRequest {
    pub block_number: u64,
    pub cluster_id: u64,
}

fn encode_proof(proof_bytes: &[u8]) -> anyhow::Result<String> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(proof_bytes)?;
    let compressed = encoder.finish()?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(compressed);
    Ok(encoded)
}

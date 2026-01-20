use anyhow::Context as _;
use base64::Engine;
use flate2::Compression;
use flate2::write::GzEncoder;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::time::Duration;

use crate::metrics::METRICS;
const ETHPROOFS_STAGING_URL: &str = "https://staging--ethproofs.netlify.app/api/v0/";
const ETHPROOFS_PRODUCTION_URL: &str = "https://ethproofs.netlify.app/api/v0/";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_ATTEMPTS: usize = 3;
const BASE_BACKOFF_MS: u64 = 200;

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
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .expect("failed to build ethproofs http client");
        Self {
            auth_token,
            cluster_id,
            url,
            client,
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
        let latency = METRICS.ethproofs_request_duration.start();
        for attempt in 1..=MAX_ATTEMPTS {
            let response = self
                .client
                .post(endpoint)
                .bearer_auth(&self.auth_token)
                .json(payload)
                .send()
                .await;

            match response {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        METRICS.ethproofs_request_success_total.inc();
                        latency.observe();
                        return Ok(());
                    }
                    if should_retry_status(status) && attempt < MAX_ATTEMPTS {
                        tracing::warn!(
                            "ethproofs request failed with status {}, retrying (attempt {}/{})",
                            status,
                            attempt,
                            MAX_ATTEMPTS
                        );
                    } else {
                        METRICS.ethproofs_request_failure_total.inc();
                        latency.observe();
                        return Err(anyhow::anyhow!(
                            "{context}: request failed with status {status}"
                        ));
                    }
                }
                Err(err) => {
                    if should_retry_error(&err) && attempt < MAX_ATTEMPTS {
                        tracing::warn!(
                            "ethproofs request error: {}, retrying (attempt {}/{})",
                            err,
                            attempt,
                            MAX_ATTEMPTS
                        );
                    } else {
                        METRICS.ethproofs_request_failure_total.inc();
                        latency.observe();
                        return Err(err).context(context);
                    }
                }
            }

            let backoff_ms = BASE_BACKOFF_MS.saturating_mul(1 << (attempt - 1));
            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
        }

        METRICS.ethproofs_request_failure_total.inc();
        latency.observe();
        Err(anyhow::anyhow!("{context}: request failed after retries"))
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

fn should_retry_status(status: StatusCode) -> bool {
    status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS
}

fn should_retry_error(err: &reqwest::Error) -> bool {
    err.is_timeout() || err.is_connect()
}

#[cfg(test)]
mod tests {
    use super::encode_proof;
    use base64::Engine as _;
    use flate2::read::GzDecoder;
    use std::io::Read;

    #[test]
    fn encode_proof_roundtrips() {
        let input = b"proof-bytes-test-vector";
        let encoded = encode_proof(input).expect("encode proof");
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .expect("decode base64");
        let mut decoder = GzDecoder::new(decoded.as_slice());
        let mut output = Vec::new();
        decoder.read_to_end(&mut output).expect("decompress");
        assert_eq!(output, input);
    }
}

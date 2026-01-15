use std::net::Ipv4Addr;
use std::time::Duration;

use anyhow::Context as _;
use vise::{Buckets, Counter, Gauge, Histogram, Metrics, MetricsCollection, Unit};
use vise_exporter::MetricsExporter;

#[derive(Debug, Metrics)]
#[metrics(prefix = "ethereum_prover")]
pub struct ProverMetrics {
    pub blocks_received_total: Counter<u64>,
    pub witness_success_total: Counter<u64>,
    pub witness_failure_total: Counter<u64>,
    #[metrics(buckets = Buckets::LATENCIES, unit = Unit::Seconds)]
    pub witness_duration_seconds: Histogram<Duration>,
    pub inflight_witness_tasks: Gauge<u64>,
    pub proof_success_total: Counter<u64>,
    pub proof_failure_total: Counter<u64>,
    #[metrics(buckets = Buckets::LATENCIES, unit = Unit::Seconds)]
    pub proof_duration_seconds: Histogram<Duration>,
    pub inflight_proof_tasks: Gauge<u64>,
    pub last_processed_block: Gauge<u64>,
    pub ethproofs_request_success_total: Counter<u64>,
    pub ethproofs_request_failure_total: Counter<u64>,
    #[metrics(buckets = Buckets::LATENCIES, unit = Unit::Seconds)]
    pub ethproofs_request_duration_seconds: Histogram<Duration>,
}

#[vise::register]
pub(crate) static METRICS: vise::Global<ProverMetrics> = vise::Global::new();

pub(crate) struct InflightGuard<'a> {
    gauge: &'a Gauge<u64>,
}

impl<'a> InflightGuard<'a> {
    pub fn new(gauge: &'a Gauge<u64>) -> Self {
        gauge.inc_by(1);
        Self { gauge }
    }
}

impl Drop for InflightGuard<'_> {
    fn drop(&mut self) {
        self.gauge.dec_by(1);
    }
}

pub async fn run_prometheus_exporter(port: u16) -> anyhow::Result<()> {
    let registry = MetricsCollection::lazy().collect();
    let exporter = MetricsExporter::new(registry.into());
    let bind_address = (Ipv4Addr::UNSPECIFIED, port).into();
    exporter
        .start(bind_address)
        .await
        .context("failed starting metrics server")?;
    Ok(())
}

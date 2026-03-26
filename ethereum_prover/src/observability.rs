use anyhow::Error;
use sentry::{Hub, SentryFutureExt as _};
use sentry_anyhow::AnyhowHubExt as _;
use std::{future::Future, sync::Arc};

// =============================================================================
// Sentry Hub Management
// =============================================================================
//
// This module keeps the rest of the prover isolated from Sentry-specific hub
// mechanics. The core idea is that each long-lived task gets its own hub, and
// each block-processing iteration gets a child hub with block metadata attached.

pub(crate) fn bind_task<F>(task_name: &'static str, future: F) -> impl Future<Output = F::Output>
where
    F: Future,
{
    bind_hub(task_hub(task_name), future)
}

pub(crate) fn bind_block<F>(
    mode: &'static str,
    block_number: u64,
    future: F,
) -> impl Future<Output = F::Output>
where
    F: Future,
{
    bind_hub(block_hub(mode, block_number), future)
}

pub(crate) async fn spawn_blocking_on_current_hub<R, F>(
    work: F,
) -> Result<R, tokio::task::JoinError>
where
    R: Send + 'static,
    F: FnOnce() -> R + Send + 'static,
{
    let hub = Hub::current();
    tokio::task::spawn_blocking(move || Hub::run(hub, work)).await
}

pub(crate) fn capture_anyhow(error: &Error) {
    Hub::with_active(|hub| {
        hub.capture_anyhow(error);
    });
}

fn bind_hub<F>(hub: Arc<Hub>, future: F) -> impl Future<Output = F::Output>
where
    F: Future,
{
    future.bind_hub(hub)
}

fn task_hub(task_name: &'static str) -> Arc<Hub> {
    child_hub(Hub::current(), |scope| {
        scope.set_tag("task", task_name);
    })
}

fn block_hub(mode: &'static str, block_number: u64) -> Arc<Hub> {
    child_hub(Hub::current(), |scope| {
        scope.set_tag("mode", mode);
        scope.set_tag("block_number", block_number.to_string());
    })
}

fn child_hub<F>(base_hub: Arc<Hub>, configure_scope: F) -> Arc<Hub>
where
    F: FnOnce(&mut sentry::Scope),
{
    let hub = Arc::new(Hub::new_from_top(base_hub));
    hub.configure_scope(configure_scope);
    hub
}

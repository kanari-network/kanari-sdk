// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use kanari_rpc_api::methods;
use std::future::Future;

// VM-backed RPCs are bounded separately from ordinary reads. The outer router
// limit protects connections, while this permit prevents a caller from
// occupying all executor workers with verification/execution work at once.
pub(super) static VM_RPC_PERMIT: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(8);
static RPC_DISPATCH_PERMIT: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(32);

/// Poll handlers on the blocking pool. RPC handlers include synchronous state,
/// database, and VM operations, so polling them on Tokio workers can starve
/// unrelated requests even when a handler is declared `async`.
pub(super) async fn run_rpc_dispatch<T, F>(future: F) -> Result<T, String>
where
    T: Send + 'static,
    F: Future<Output = T> + Send + 'static,
{
    let permit = RPC_DISPATCH_PERMIT
        .try_acquire()
        .map_err(|_| "RPC execution capacity is temporarily exhausted".to_string())?;
    let runtime = tokio::runtime::Handle::current();
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        runtime.block_on(future)
    })
    .await
    .map_err(|error| format!("RPC dispatch task failed: {error}"))
}

#[cfg(test)]
mod blocking_engine_tests {
    use super::run_rpc_dispatch;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    #[tokio::test(flavor = "current_thread")]
    async fn synchronous_rpc_handler_work_does_not_block_tokio_worker() {
        let started = Arc::new(AtomicBool::new(false));
        let handler_started = Arc::clone(&started);
        let task = tokio::spawn(run_rpc_dispatch(async move {
            handler_started.store(true, Ordering::Release);
            std::thread::sleep(std::time::Duration::from_millis(100));
            9
        }));

        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while !started.load(Ordering::Acquire) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("synchronous handler work should run outside the async worker");

        assert_eq!(task.await.expect("join task").expect("RPC dispatch"), 9);
    }
}

pub(super) fn is_vm_heavy_rpc(method: &str) -> bool {
    matches!(
        method,
        methods::VERIFY_MODULE
            | methods::BUILD_PUBLISH_MODULE
            | methods::BUILD_UPGRADE_MODULE
            | methods::PUBLISH_MODULE
            | methods::UPGRADE_MODULE
            | methods::BUILD_PUBLISH_PACKAGE
            | methods::BUILD_UPGRADE_PACKAGE
            | methods::PUBLISH_PACKAGE
            | methods::UPGRADE_PACKAGE
            | methods::BUILD_CALL_FUNCTION
            | methods::BUILD_TOKEN_TRANSFER
            | methods::CALL_FUNCTION
            | methods::VIEW_FUNCTION
    )
}

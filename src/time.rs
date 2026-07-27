// Copyright (c) 2022-2023 Yuki Kishimoto
// Distributed under the MIT software license

//! Time module

use core::future::Future;
use core::time::Duration;
use std::pin::pin;

use futures_util::future::{self, Either};

#[cfg(not(target_arch = "wasm32"))]
use crate::runtime;

/// Sleep
pub async fn sleep(duration: Duration) {
    #[cfg(not(target_arch = "wasm32"))]
    if runtime::is_tokio_context() {
        tokio::time::sleep(duration).await;
    } else {
        // No need to propagate error
        let _ = runtime::handle()
            .spawn(async move {
                tokio::time::sleep(duration).await;
            })
            .await;
    }

    #[cfg(target_arch = "wasm32")]
    gloo_timers::future::sleep(duration).await;
}

/// Timeout
pub async fn timeout<F>(duration: Option<Duration>, future: F) -> Option<F::Output>
where
    F: Future,
{
    let Some(duration) = duration else {
        return Some(future.await);
    };

    let future = pin!(future);
    let timer = pin!(sleep(duration));

    match future::select(future, timer).await {
        Either::Left((output, _timer)) => Some(output),
        Either::Right(((), _future)) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: test also wasm

    #[tokio::test]
    #[cfg(not(target_arch = "wasm32"))]
    async fn test_sleep_in_tokio() {
        sleep(Duration::from_secs(5)).await;
    }

    #[async_std::test]
    #[cfg(not(target_arch = "wasm32"))]
    async fn test_sleep_in_async_std() {
        sleep(Duration::from_secs(5)).await;
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_sleep_in_smol() {
        smol::block_on(async {
            sleep(Duration::from_secs(5)).await;
        });
    }

    #[tokio::test]
    #[cfg(not(target_arch = "wasm32"))]
    async fn test_timeout_tokio() {
        // Timeout
        let result = timeout(Some(Duration::from_secs(1)), async {
            sleep(Duration::from_secs(2)).await;
        })
        .await;
        assert!(result.is_none());

        // Not timeout
        let result = timeout(Some(Duration::from_secs(10)), async {
            sleep(Duration::from_secs(1)).await;
        })
        .await;
        assert!(result.is_some());
    }

    #[async_std::test]
    #[cfg(not(target_arch = "wasm32"))]
    async fn test_timeout_async_std() {
        // Timeout
        let result = timeout(Some(Duration::from_secs(1)), async {
            sleep(Duration::from_secs(2)).await;
        })
        .await;
        assert!(result.is_none());

        // Not timeout
        let result = timeout(Some(Duration::from_secs(10)), async {
            sleep(Duration::from_secs(1)).await;
        })
        .await;
        assert!(result.is_some());
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_timeout_smol() {
        smol::block_on(async {
            // Timeout
            let result = timeout(Some(Duration::from_secs(1)), async {
                sleep(Duration::from_secs(2)).await;
            })
            .await;
            assert!(result.is_none());

            // Not timeout
            let result = timeout(Some(Duration::from_secs(10)), async {
                sleep(Duration::from_secs(1)).await;
            })
            .await;
            assert!(result.is_some());
        });
    }
}

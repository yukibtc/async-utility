// Copyright (c) 2022-2023 Yuki Kishimoto
// Distributed under the MIT software license

//! Task

use core::fmt;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::OnceLock;

use futures_util::stream::{AbortHandle, Abortable};
use futures_util::Future;
#[cfg(not(target_arch = "wasm32"))]
use tokio::runtime::{Builder, Handle, Runtime};
#[cfg(not(target_arch = "wasm32"))]
use tokio::task::JoinHandle as TokioJoinHandle;

#[cfg(target_arch = "wasm32")]
mod wasm;

// TODO: use LazyLock when MSRV will be at 1.80.0
#[cfg(not(target_arch = "wasm32"))]
static RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// Task error
#[derive(Debug)]
pub enum Error {
    /// Join Error
    JoinError,
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JoinError => write!(f, "impossible to join thread"),
        }
    }
}

/// Join Handle
pub enum JoinHandle<T> {
    /// Tokio
    #[cfg(not(target_arch = "wasm32"))]
    Tokio(TokioJoinHandle<T>),
    /// Wasm
    #[cfg(target_arch = "wasm32")]
    Wasm(self::wasm::JoinHandle<T>),
}

impl<T> JoinHandle<T> {
    /// Join
    pub async fn join(self) -> Result<T, Error> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Tokio(handle) => handle.await.map_err(|_| Error::JoinError),
            #[cfg(target_arch = "wasm32")]
            Self::Wasm(handle) => handle.join().await.map_err(|_| Error::JoinError),
        }
    }
}

/// Spawn new thread
#[cfg(not(target_arch = "wasm32"))]
pub fn spawn<T>(future: T) -> JoinHandle<T::Output>
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    let handle = if is_tokio_context() {
        tokio::task::spawn(future)
    } else {
        runtime().spawn(future)
    };
    JoinHandle::Tokio(handle)
}

/// Spawn a new thread
#[cfg(target_arch = "wasm32")]
pub fn spawn<T>(future: T) -> JoinHandle<T::Output>
where
    T: Future + 'static,
{
    let handle = self::wasm::spawn(future);
    JoinHandle::Wasm(handle)
}

/// Spawn abortable thread
#[cfg(not(target_arch = "wasm32"))]
pub fn abortable<T>(future: T) -> AbortHandle
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    let (abort_handle, abort_registration) = AbortHandle::new_pair();
    let _ = spawn(Abortable::new(future, abort_registration));
    abort_handle
}

/// Spawn abortable thread
#[cfg(target_arch = "wasm32")]
pub fn abortable<T>(future: T) -> AbortHandle
where
    T: Future + 'static,
{
    let (abort_handle, abort_registration) = AbortHandle::new_pair();
    let _ = spawn(Abortable::new(future, abort_registration));
    abort_handle
}

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_blocking<F, R>(f: F) -> TokioJoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    if is_tokio_context() {
        tokio::task::spawn_blocking(f)
    } else {
        runtime().spawn_blocking(f)
    }
}

#[inline]
#[cfg(not(target_arch = "wasm32"))]
fn is_tokio_context() -> bool {
    Handle::try_current().is_ok()
}

#[cfg(not(target_arch = "wasm32"))]
fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime")
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::time;

    #[tokio::test]
    #[cfg(not(target_arch = "wasm32"))]
    async fn test_is_tokio_context_macros() {
        assert!(is_tokio_context());
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_is_tokio_context_once_lock() {
        let rt = runtime();
        let _guard = rt.enter();
        assert!(is_tokio_context());
    }

    #[tokio::test]
    #[cfg(not(target_arch = "wasm32"))]
    async fn test_spawn() {
        let future = async {
            time::sleep(Duration::from_secs(1)).await;
            42
        };
        let handle = spawn(future);
        let result = handle.join().await.unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_spawn_outside_tokio_ctx() {
        let future = async {
            time::sleep(Duration::from_secs(1)).await;
            42
        };
        let _handle = spawn(future);
    }

    #[tokio::test]
    #[cfg(not(target_arch = "wasm32"))]
    async fn test_spawn_blocking() {
        let handle = spawn_blocking(|| 42);
        let result = handle.await.unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_spawn_blocking_outside_tokio_ctx() {
        let _handle = spawn_blocking(|| 42);
    }

    #[tokio::test]
    #[cfg(not(target_arch = "wasm32"))]
    async fn test_abortable() {
        let future = async {
            time::sleep(Duration::from_secs(1)).await;
            42
        };
        let abort_handle = abortable(future);
        abort_handle.abort();
        assert!(abort_handle.is_aborted());
    }
}

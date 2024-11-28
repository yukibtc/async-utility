// Copyright (c) 2022-2023 Yuki Kishimoto
// Distributed under the MIT software license

//! Thread

use core::fmt;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::OnceLock;

use futures_util::stream::{AbortHandle, Abortable};
use futures_util::Future;
#[cfg(not(target_arch = "wasm32"))]
use tokio::runtime::{Builder, Handle, Runtime};

#[cfg(target_arch = "wasm32")]
mod wasm;

// TODO: use LazyLock when MSRV will be at 1.80.0
#[cfg(not(target_arch = "wasm32"))]
static RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// Thread Error
#[derive(Debug)]
pub enum Error {
    #[cfg(not(target_arch = "wasm32"))]
    IO(std::io::Error),
    /// Join Error
    JoinError,
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::IO(e) => write!(f, "{e}"),
            Self::JoinError => write!(f, "impossible to join thread"),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::IO(e)
    }
}

/// Join Handle
pub enum JoinHandle<T> {
    /// Std
    #[cfg(not(target_arch = "wasm32"))]
    Std(std::thread::JoinHandle<T>),
    /// Tokio
    #[cfg(not(target_arch = "wasm32"))]
    Tokio(tokio::task::JoinHandle<T>),
    /// Wasm
    #[cfg(target_arch = "wasm32")]
    Wasm(self::wasm::JoinHandle<T>),
}

impl<T> JoinHandle<T> {
    /// Join
    pub async fn join(self) -> Result<T, Error> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Std(handle) => handle.join().map_err(|_| Error::JoinError),
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
    if is_tokio_context() {
        let handle = tokio::task::spawn(future);
        JoinHandle::Tokio(handle)
    } else {
        let handle = std::thread::spawn(move || runtime().block_on(future));
        JoinHandle::Std(handle)
    }
}

/// Spawn a new thread
#[cfg(target_arch = "wasm32")]
pub fn spawn<T>(future: T) -> Result<JoinHandle<T::Output>, Error>
where
    T: Future + 'static,
{
    let handle = self::wasm::spawn(future);
    Ok(JoinHandle::Wasm(handle))
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
pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let handle = if is_tokio_context() {
        tokio::task::spawn_blocking(f)
    } else {
        runtime().spawn_blocking(f)
    };
    JoinHandle::Tokio(handle)
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

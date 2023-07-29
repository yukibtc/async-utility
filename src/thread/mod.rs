// Copyright (c) 2022-2023 Yuki Kishimoto
// Distributed under the MIT software license

//! Thread

use core::fmt;
use core::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use crossbeam_channel::{bounded, RecvError};
use futures_util::stream::{AbortHandle, Abortable};
use futures_util::Future;
#[cfg(feature = "blocking")]
use tokio::runtime::{Builder, Runtime};

#[cfg(target_arch = "wasm32")]
mod wasm;

type Result<T, E = Box<dyn std::error::Error>> = core::result::Result<T, E>;

#[cfg(feature = "blocking")]
fn new_current_thread() -> Result<Runtime> {
    Ok(Builder::new_current_thread().enable_all().build()?)
}

/// Thread Error
#[derive(Debug)]
pub enum Error {
    /// RecvError
    #[cfg(not(target_arch = "wasm32"))]
    Recv(RecvError),
    /// Join Error
    JoinError,
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Recv(e) => write!(f, "impossible to recv: {e}"),
            Self::JoinError => write!(f, "impossible to join thread"),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<RecvError> for Error {
    fn from(e: RecvError) -> Self {
        Self::Recv(e)
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

#[cfg(not(target_arch = "wasm32"))]
impl<T> JoinHandle<T>
where
    T: Send + 'static,
{
    /// Join (can be execute in async context)
    pub fn join_blocking(self) -> Result<T, Error> {
        let (tx, rx) = bounded(1);
        spawn(async move {
            tx.send(self.join().await).unwrap();
        });
        rx.recv()?
    }
}

/// Spawn
#[cfg(not(target_arch = "wasm32"))]
pub fn spawn<T>(future: T) -> Option<JoinHandle<T::Output>>
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    #[cfg(feature = "blocking")]
    match new_current_thread() {
        Ok(rt) => {
            let handle = std::thread::spawn(move || {
                let res = rt.block_on(future);
                rt.shutdown_timeout(Duration::from_millis(100));
                res
            });
            Some(JoinHandle::Std(handle))
        }
        Err(_) => None,
    }

    #[cfg(not(feature = "blocking"))]
    {
        let handle = tokio::task::spawn(future);
        Some(JoinHandle::Tokio(handle))
    }
}

/// Spawn
#[cfg(target_arch = "wasm32")]
pub fn spawn<T>(future: T) -> Option<JoinHandle<T::Output>>
where
    T: Future + 'static,
{
    let handle = self::wasm::spawn(future);
    Some(JoinHandle::Wasm(handle))
}

/// Spawn abortable thread
#[cfg(not(target_arch = "wasm32"))]
pub fn abortable<T>(future: T) -> AbortHandle
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    let (abort_handle, abort_registration) = AbortHandle::new_pair();
    spawn(Abortable::new(future, abort_registration));
    abort_handle
}

/// Spawn abortable thread
#[cfg(target_arch = "wasm32")]
pub fn abortable<T>(future: T) -> AbortHandle
where
    T: Future + 'static,
{
    let (abort_handle, abort_registration) = AbortHandle::new_pair();
    spawn(Abortable::new(future, abort_registration));
    abort_handle
}

/// Sleep
pub async fn sleep(duration: Duration) {
    #[cfg(not(target_arch = "wasm32"))]
    tokio::time::sleep(duration).await;
    #[cfg(target_arch = "wasm32")]
    gloo_timers::future::sleep(duration).await;
}

// Copyright (c) 2022-2023 Yuki Kishimoto
// Distributed under the MIT software license

//! Task

use core::fmt;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[cfg(not(target_arch = "wasm32"))]
use tokio::task::JoinHandle as TokioJoinHandle;

#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(not(target_arch = "wasm32"))]
use crate::runtime;

/// Task error
#[derive(Debug)]
pub struct JoinError;

impl std::error::Error for JoinError {}

impl fmt::Display for JoinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("impossible to join task")
    }
}

enum Inner<T> {
    /// Tokio
    #[cfg(not(target_arch = "wasm32"))]
    Tokio(TokioJoinHandle<T>),
    /// Wasm
    #[cfg(target_arch = "wasm32")]
    Wasm(self::wasm::JoinHandle<T>),
}

/// A handle for awaiting a spawned task or requesting its cancellation.
///
/// Dropping the handle detaches the task, allowing it to keep running.
pub struct JoinHandle<T>(Inner<T>);

impl<T> JoinHandle<T> {
    /// Request cancellation of the task.
    ///
    /// Await the handle to wait for cancellation and release of the task's
    /// resources. Cancellation returns [`JoinError`]; a task that has
    /// already completed retains its result. Repeated calls are harmless.
    #[inline]
    pub fn abort(&self) {
        match &self.0 {
            #[cfg(not(target_arch = "wasm32"))]
            Inner::Tokio(handle) => handle.abort(),
            #[cfg(target_arch = "wasm32")]
            Inner::Wasm(handle) => handle.abort(),
        }
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, JoinError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let handle: &mut JoinHandle<T> = self.get_mut();

        match &mut handle.0 {
            #[cfg(not(target_arch = "wasm32"))]
            Inner::Tokio(handle) => Pin::new(handle).poll(cx).map_err(|_| JoinError),
            #[cfg(target_arch = "wasm32")]
            Inner::Wasm(handle) => Pin::new(handle).poll(cx),
        }
    }
}

/// Spawn new task
#[inline]
#[cfg(not(target_arch = "wasm32"))]
pub fn spawn<T>(future: T) -> JoinHandle<T::Output>
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    let handle = runtime::handle().spawn(future);
    JoinHandle(Inner::Tokio(handle))
}

/// Spawn a new task
#[cfg(target_arch = "wasm32")]
pub fn spawn<T>(future: T) -> JoinHandle<T::Output>
where
    T: Future + 'static,
{
    let handle = self::wasm::spawn(future);
    JoinHandle(Inner::Wasm(handle))
}

#[inline]
#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_blocking<F, R>(f: F) -> TokioJoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    runtime::handle().spawn_blocking(f)
}

pub struct AbortOnDropHandle<T>(JoinHandle<T>);

impl<T> fmt::Debug for AbortOnDropHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AbortOnDropHandle").finish()
    }
}

impl<T> AbortOnDropHandle<T> {
    #[inline]
    pub fn new(handle: JoinHandle<T>) -> Self {
        Self(handle)
    }

    #[inline]
    pub fn abort(&self) {
        self.0.abort();
    }
}

impl<T> Drop for AbortOnDropHandle<T> {
    #[inline]
    fn drop(&mut self) {
        self.abort();
    }
}

#[cfg(test)]
mod tests {
    #![cfg_attr(target_arch = "wasm32", allow(unexpected_cfgs))]

    use core::future::Future;
    use core::marker::PhantomPinned;
    use core::pin::Pin;
    use core::task::{Context, Poll};

    use tokio::sync::oneshot;

    use super::*;
    #[cfg(not(target_arch = "wasm32"))]
    use crate::runtime;

    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    struct PendingTask {
        started: Option<oneshot::Sender<()>>,
        dropped: Option<oneshot::Sender<()>>,
    }

    impl Future for PendingTask {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            if let Some(started) = self.started.take() {
                let _ = started.send(());
            }
            Poll::Pending
        }
    }

    impl Drop for PendingTask {
        fn drop(&mut self) {
            if let Some(dropped) = self.dropped.take() {
                let _ = dropped.send(());
            }
        }
    }

    fn pending_task() -> (JoinHandle<()>, oneshot::Receiver<()>, oneshot::Receiver<()>) {
        let (started_tx, started_rx) = oneshot::channel();
        let (dropped_tx, dropped_rx) = oneshot::channel();
        let handle = spawn(PendingTask {
            started: Some(started_tx),
            dropped: Some(dropped_tx),
        });
        (handle, started_rx, dropped_rx)
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_is_tokio_context_macros() {
        assert!(runtime::is_tokio_context());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[async_std::test]
    async fn test_is_tokio_context_in_async_std() {
        let handle = runtime::handle();
        let _guard = handle.enter();
        assert!(runtime::is_tokio_context());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_is_tokio_context_once_lock() {
        let handle = runtime::handle();
        let _guard = handle.enter();
        assert!(runtime::is_tokio_context());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[async_std::test]
    async fn test_spawn_in_async_std() {
        let future = async { 42 };
        let handle = spawn(future);
        let result = handle.await.unwrap();
        assert_eq!(result, 42);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_spawn_in_smol() {
        smol::block_on(async {
            let future = async { 42 };
            let handle = spawn(future);
            let result = handle.await.unwrap();
            assert_eq!(result, 42);
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_spawn_outside_tokio_ctx() {
        let future = async { 42 };
        let _handle = spawn(future);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_spawn_blocking() {
        let handle = spawn_blocking(|| 42);
        let result = handle.await.unwrap();
        assert_eq!(result, 42);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_spawn_blocking_outside_tokio_ctx() {
        let _handle = spawn_blocking(|| 42);
    }

    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    async fn await_returns_output() {
        assert_eq!(spawn(async { 42 }).await.unwrap(), 42);
    }

    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    async fn non_unpin_output() {
        struct Output {
            value: u8,
            _pin: PhantomPinned,
        }

        let make_output = || async {
            Output {
                value: 42,
                _pin: PhantomPinned,
            }
        };

        let mut handle = spawn(make_output());
        assert_eq!((&mut handle).await.unwrap().value, 42);
        assert_eq!(spawn(make_output()).await.unwrap().value, 42);
    }

    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    async fn abort_before_first_poll() {
        let (handle, mut started, mut dropped) = pending_task();

        handle.abort();

        assert!(matches!(handle.await, Err(JoinError)));
        assert_eq!(
            started.try_recv(),
            Err(oneshot::error::TryRecvError::Closed)
        );
        assert_eq!(dropped.try_recv(), Ok(()));
    }

    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    async fn abort_running_task() {
        let (handle, started, mut dropped) = pending_task();
        started.await.unwrap();

        handle.abort();

        assert!(matches!(handle.await, Err(JoinError)));
        assert_eq!(dropped.try_recv(), Ok(()));
    }

    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    async fn repeated_abort() {
        let (handle, started, mut dropped) = pending_task();
        started.await.unwrap();

        handle.abort();
        handle.abort();

        assert!(matches!(handle.await, Err(JoinError)));
        assert_eq!(dropped.try_recv(), Ok(()));
    }

    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    async fn abort_completed_task_preserves_output() {
        let (completed_tx, completed_rx) = oneshot::channel();
        let handle = spawn(async move {
            completed_tx.send(()).unwrap();
            42
        });
        // Both test executors run on one thread. The task returns its output
        // before this test can resume after receiving the completion signal.
        completed_rx.await.unwrap();

        handle.abort();

        assert_eq!(handle.await.unwrap(), 42);
    }

    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    async fn dropping_handle_detaches_task() {
        let (resume_tx, resume_rx) = oneshot::channel();
        let (completed_tx, completed_rx) = oneshot::channel();
        let handle = spawn(async move {
            resume_rx.await.unwrap();
            completed_tx.send(42).unwrap();
        });

        drop(handle);
        resume_tx.send(()).unwrap();

        assert_eq!(completed_rx.await.unwrap(), 42);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn panic_returns_join_error() {
        let handle = spawn(async { panic!("task panic") });

        assert!(matches!(handle.await, Err(JoinError)));
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn non_send_future_and_output() {
        use std::cell::Cell;
        use std::rc::Rc;

        let value = Rc::new(Cell::new(0));
        let task_value = value.clone();
        let (resume_tx, resume_rx) = oneshot::channel();
        let handle = spawn(async move {
            resume_rx.await.unwrap();
            task_value.set(42);
            task_value
        });
        resume_tx.send(()).unwrap();

        let output = handle.await.unwrap();
        assert!(Rc::ptr_eq(&value, &output));
        assert_eq!(output.get(), 42);
    }
}

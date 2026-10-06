// Copyright (c) 2022-2023 Yuki Kishimoto
// Distributed under the MIT software license

use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::{Context, Poll};
use std::sync::Arc;

use futures_util::future::{AbortHandle, Abortable};
use tokio::sync::oneshot::{self, Receiver};
use wasm_bindgen_futures::spawn_local;

use super::JoinError;

struct CompletionGuard(Arc<AtomicBool>);

impl Drop for CompletionGuard {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

pub(super) struct JoinHandle<T> {
    rx: Receiver<T>,
    abort_handle: AbortHandle,
    finished: Arc<AtomicBool>,
}

impl<T> JoinHandle<T> {
    #[inline]
    pub(super) fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Acquire)
    }

    #[inline]
    pub(super) fn abort(&self) {
        self.abort_handle.abort();
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, JoinError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.get_mut().rx)
            .poll(cx)
            .map_err(|_| JoinError)
    }
}

pub(super) fn spawn<T>(f: T) -> JoinHandle<T::Output>
where
    T: Future + 'static,
    T::Output: 'static,
{
    let (abort_handle, abort_registration) = AbortHandle::new_pair();
    let (tx, rx) = oneshot::channel();
    let finished: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let completion = CompletionGuard(finished.clone());

    spawn_local(async move {
        // Capture the future before the guard, so it is also dropped first if
        // the spawned future is released before its first poll.
        let future: T = f;
        let _completion: CompletionGuard = completion;

        if let Ok(output) = Abortable::new(future, abort_registration).await {
            let _ = tx.send(output);
        } else {
            drop(tx);
        }
    });

    JoinHandle {
        rx,
        abort_handle,
        finished,
    }
}

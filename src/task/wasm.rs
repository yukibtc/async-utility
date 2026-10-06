// Copyright (c) 2022-2023 Yuki Kishimoto
// Distributed under the MIT software license

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use futures_util::future::{AbortHandle, Abortable};
use tokio::sync::oneshot::{self, Receiver};
use wasm_bindgen_futures::spawn_local;

use super::JoinError;

pub(super) struct JoinHandle<T> {
    rx: Receiver<T>,
    abort_handle: AbortHandle,
}

impl<T> JoinHandle<T> {
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

    spawn_local(async move {
        if let Ok(output) = Abortable::new(f, abort_registration).await {
            let _ = tx.send(output);
        }
    });

    JoinHandle { rx, abort_handle }
}

// Copyright (c) 2022-2023 Yuki Kishimoto
// Distributed under the MIT software license

use core::fmt;

use futures_util::Future;
use tokio::sync::oneshot::{self, Receiver};
use wasm_bindgen_futures::spawn_local;

use super::Error;

pub struct JoinHandle<T>(Receiver<T>);

impl<T> JoinHandle<T> {
    #[inline]
    pub async fn join(self) -> Result<T, Error> {
        self.0.await.map_err(|_| Error::JoinError)
    }
}

impl<T> fmt::Debug for JoinHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("JoinHandle { .. }")
    }
}

pub fn spawn<T>(f: T) -> JoinHandle<T::Output>
where
    T: Future + 'static,
    T::Output: 'static,
{
    let (sender, receiver) = oneshot::channel();

    spawn_local(async {
        let res = f.await;
        sender.send(res).ok();
    });

    JoinHandle(receiver)
}

pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + 'static,
    R: 'static,
{
    let (sender, receiver) = oneshot::channel();

    spawn_local(async {
        let res: R = f();
        sender.send(res).ok();
    });

    JoinHandle(receiver)
}

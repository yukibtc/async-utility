// Copyright (c) 2022-2023 Yuki Kishimoto
// Distributed under the MIT software license

//! Async to blocking conversion

use core::future::Future;

use crate::runtime;

/// Async to blocking conversion
pub trait Async2Blocking {
    type Output;

    /// Executes the asynchronous task and blocks the current thread until it completes.
    fn blocking(self) -> Self::Output;
}

impl<T> Async2Blocking for T
where
    T: Future,
{
    type Output = T::Output;

    #[inline]
    fn blocking(self) -> Self::Output {
        runtime::runtime().block_on(self)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::time;

    #[test]
    fn test_blocking() {
        let future = async {
            time::sleep(Duration::from_secs(5)).await;
            42
        };
        let result = future.blocking();
        assert_eq!(result, 42);
    }
}

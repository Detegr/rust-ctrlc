//! Helper routines for the asynchronous functions

use std::future::Future;
use std::time::Duration;

/// Spawns a background thread using which ever asynchronous runtime the library is built with
pub fn spawn<T>(future: T)
where T: Future + Send + 'static, T::Output: Send + 'static,
{
    #[cfg(feature = "tokio")]
    if tokio::runtime::Handle::try_current().is_ok() {
        tokio::spawn(future);
    } else {
        std::thread::Builder::new()
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                rt.block_on(future);
            })
            .expect("failed to spawn thread");
    }

    #[cfg(feature = "async-std")]
    if async_std::task::try_current().is_some() {
        async_std::task::spawn(future);
    } else {
        std::thread::Builder::new()
            .spawn(move || {
                async_std::task::block_on(future);
            })
            .expect("failed to spawn thread");
    }
}

/// Safely executing blocking code using which ever asynchronous runtime the library is built with
pub fn spawn_block_on<F>(mut task: F)
where F: FnMut() -> () + 'static + Send,
{
    #[cfg(feature = "tokio")]
    if tokio::runtime::Handle::try_current().is_ok() {
        tokio::task::spawn_blocking(move || {
            task();
        });
        return;
    }

    #[cfg(feature = "async-std")]
    if async_std::task::try_current().is_some() {
        std::thread::Builder::new()
            .spawn(task)
            .expect("failed to spawn thread");
        return;
    }

    task();
}

/// Executes a future with a specific timeout using which ever asynchronous runtime the library is built with
#[cfg(test)]
#[must_use = "this `Option` should be handled"]
pub async fn timeout<F>(duration: Duration, future: F) -> Option<F::Output>
where F: Future
{
    #[cfg(feature = "tokio")]
    return match tokio::time::timeout(duration, future).await {
        Ok(a) => Some(a),
        Err(_) => None
    };

    #[cfg(feature = "async-std")]
    return match async_std::future::timeout(duration, future).await {
        Ok(a) => Some(a),
        Err(_) => None
    };
}

/// Sleeps for a fixed period of time (without blocking asynchronous runtimes)
pub async fn sleep(duration: Duration)
{
    #[cfg(feature = "tokio")]
    tokio::time::sleep(duration).await;

    #[cfg(feature = "async-std")]
    async_std::task::sleep(duration).await;
}
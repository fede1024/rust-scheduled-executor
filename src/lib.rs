//! This library provides a series of utilities for scheduling and executing tasks.
//!
//! A simple executor for scheduled tasks. Tasks can be executed at fixed interval or at
//! fixed rates, and can be executed in the main executor thread, or using a thread pool.
#[macro_use] extern crate log;
extern crate futures;
extern crate tokio_core;
extern crate futures_cpupool;

pub mod executor;
pub mod task_group;

pub use executor::CoreExecutor;
pub use task_group::TaskGroup;
pub use futures_cpupool::CpuPool;
pub use tokio_core::reactor::Handle;

use futures_cpupool::Builder;

pub fn thread_pool(threads: usize, prefix: &str) -> CpuPool {
    Builder::new()
        .pool_size(threads)
        .name_prefix(prefix)
        .create()
}

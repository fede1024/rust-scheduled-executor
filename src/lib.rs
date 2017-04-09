extern crate futures;
extern crate tokio_core;
extern crate futures_cpupool;

pub mod executor;
pub mod task_group;

pub use executor::Executor;
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

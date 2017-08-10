//! ## The library
//!
//! This library provides a series of utilities for scheduling and executing tasks (functions and
//! closures). Tasks can be executed at fixed interval or at fixed rates, and can be executed
//! sequentially in the main executor thread or in parallel using a thread pool.
//!
//! ### Executors
//!
//! - [`CoreExecutor`]: schedule and execute tasks on a single thread, ideal for short running tasks.
//! - [`ThreadPoolExecutor`]: schedule and execute tasks on a thread pool. Can be used for long
//! running tasks.
//!
//! [`CoreExecutor`]: https://fede1024.github.io/rust-scheduled-executor/scheduled_executor/executor/struct.CoreExecutor.html
//! [`ThreadPoolExecutor`]: https://fede1024.github.io/rust-scheduled-executor/scheduled_executor/executor/struct.ThreadPoolExecutor.html
//!
//! ### Task group
//! The scheduled-executor crate also provides an abstraction for the execution of groups of tasks
//! called [`TaskGroup`]. A `TaskGroup` requires a method for the generation of the collection of
//! tasks, which will be executed at the beginning of each cycle, and a method for the execution of
//! individual task, which will be executed for each task.
//!
//! To see a task group in action, check out the [`task_group.rs`] example.
//!
//! [`TaskGroup`]: https://fede1024.github.io/rust-scheduled-executor/scheduled_executor/executor/struct.ThreadPoolExecutor.html
//! [`task_group.rs`]: https://github.com/fede1024/rust-scheduled-executor/blob/master/examples/task_group.rs
//!
//! ### Documentation
//!
//! - [Current master branch](https://fede1024.github.io/rust-scheduled-executor/)
//! - [Latest release](https://docs.rs/scheduled-executor/)
//!
//! ### Examples
//!
//! Scheduling periodic task is very simple. Here is an example using a thread pool:
//!
//! ```rust,ignore
//! // Starts a new thread-pool based executor with 4 threads
//! let executor = ThreadPoolExecutor::new(4)?;
//!
//! executor.schedule_fixed_rate(
//!     Duration::from_secs(2),  // Wait 2 seconds before scheduling the first task
//!     Duration::from_secs(5),  // and schedule every following task at 5 seconds intervals
//!     |remote| {
//!         // Code to be scheduled. The code will run on one of the threads in the thread pool.
//!         // The `remote` handle can be used to schedule additional work on the event loop,
//!         // if needed.
//!     },
//! );
//! ```
//!
#[macro_use] extern crate log;
extern crate futures;
extern crate tokio_core;
extern crate futures_cpupool;

pub mod executor;
pub mod task_group;

pub use executor::{CoreExecutor, ThreadPoolExecutor};
pub use task_group::{TaskGroup, TaskGroupScheduler};

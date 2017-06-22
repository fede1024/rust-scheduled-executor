//! ## The library
//!
//! This library provides a series of utilities for scheduling and executing tasks.
//! Tasks can be executed at fixed interval or at fixed rates, and can be executed in the main
//! executor thread or using a thread pool.
//!
//! ### Documentation
//!
//! - [Current master branch](https://fede1024.github.io/rust-scheduled-executor/)
//!
#[macro_use] extern crate log;
extern crate futures;
extern crate tokio_core;
extern crate futures_cpupool;

pub mod executor;
pub mod task_group;

pub use executor::{CoreExecutor, ThreadPoolExecutor};
pub use task_group::{TaskGroup, TaskGroupScheduler};
pub use tokio_core::reactor::{Handle, Remote};

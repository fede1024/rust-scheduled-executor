use futures::future::Future;
use futures::sync::oneshot::{channel, Sender};
use futures_cpupool::{Builder, CpuPool};
use tokio_core::reactor::{Core, Handle, Remote};
use tokio_core::reactor::Timeout;

use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Instant, Duration};
use std::io;


fn fixed_interval_loop<F>(scheduled_fn: F, interval: Duration, handle: &Handle)
    where F: Fn(&Handle) + Send + 'static
{
    let start_time = Instant::now();
    scheduled_fn(&handle);
    let execution = start_time.elapsed();
    let next_iter_wait = if execution >= interval {
        Duration::from_secs(0)
    } else {
        interval - execution
    };
    let handle_clone = handle.clone();
    let t = Timeout::new(next_iter_wait, handle).unwrap()
        .then(move |_| {
            fixed_interval_loop(scheduled_fn, interval, &handle_clone);
            Ok::<(), ()>(())
        });
    handle.spawn(t);
}

fn calculate_delay(interval: Duration, execution: Duration, delay: Duration) -> (Duration, Duration) {
    if execution >= interval {
        (Duration::from_secs(0), delay + execution - interval)
    } else {
        let wait_gap = interval - execution;
        if delay == Duration::from_secs(0) {
            (wait_gap, Duration::from_secs(0))
        } else {
            if delay < wait_gap {
                (wait_gap - delay, Duration::from_secs(0))
            } else {
                (Duration::from_secs(0), delay - wait_gap)
            }
        }
    }
}

fn fixed_rate_loop<F>(scheduled_fn: F, interval: Duration, handle: &Handle, delay: Duration)
    where F: Fn(&Handle) + Send + 'static
{
    let start_time = Instant::now();
    scheduled_fn(&handle);
    let execution = start_time.elapsed();
    let (next_iter_wait, updated_delay) = calculate_delay(interval, execution, delay);
    let handle_clone = handle.clone();
    let t = Timeout::new(next_iter_wait, handle).unwrap()
        .then(move |_| {
            fixed_rate_loop(scheduled_fn, interval, &handle_clone, updated_delay);
            Ok::<(), ()>(())
        });
    handle.spawn(t);
}


struct CoreExecutorInner {
    remote: Remote,
    termination_sender: Option<Sender<()>>,
    thread_handle: Option<JoinHandle<()>>,
}

impl Drop for CoreExecutorInner {
    fn drop(&mut self) {
        let _ = self.termination_sender.take().unwrap().send(());
        let _ = self.thread_handle.take().unwrap().join();
    }
}

pub struct CoreExecutor {
    inner: Arc<CoreExecutorInner>
}

impl Clone for CoreExecutor {
    fn clone(&self) -> Self {
        CoreExecutor { inner: Arc::clone(&self.inner) }
    }
}

impl CoreExecutor {
    pub fn new() -> Result<CoreExecutor, io::Error> {
        CoreExecutor::with_name("core_executor")
    }

    pub fn with_name(thread_name: &str) -> Result<CoreExecutor, io::Error> {
        let (termination_tx, termination_rx) = channel();
        let (core_tx, core_rx) = channel();
        let thread_handle = thread::Builder::new()
            .name(thread_name.to_owned())
            .spawn(move || {
                debug!("Core starting");
                let mut core = Core::new().expect("Failed to start core");
                let _ = core_tx.send(core.remote());
                match core.run(termination_rx) {
                    Ok(v) => debug!("Core terminated correctly {:?}", v),
                    Err(e) => debug!("Core terminated with error: {:?}", e),
                }
            })?;
        let inner = CoreExecutorInner {
            remote: core_rx.wait().expect("Failed to receive remote"),
            termination_sender: Some(termination_tx),
            thread_handle: Some(thread_handle),
        };
        let executor = CoreExecutor {
            inner: Arc::new(inner)
        };
        debug!("Executor created");
        Ok(executor)
    }

    pub fn schedule_fixed_interval<F>(&self, interval: Duration, scheduled_fn: F)
        where F: Fn(&Handle) + Send + 'static
    {
        self.inner.remote.spawn(move |handle| {
            fixed_interval_loop(scheduled_fn, interval, handle);
            Ok::<(), ()>(())
        });
    }

    pub fn schedule_fixed_rate<F>(&self, interval: Duration, scheduled_fn: F)
        where F: Fn(&Handle) + Send + 'static
    {
        self.inner.remote.spawn(move |handle| {
            fixed_rate_loop(scheduled_fn, interval, handle, Duration::from_secs(0));
            Ok::<(), ()>(())
        });
    }
}


pub struct ThreadPoolExecutor {
    executor: CoreExecutor,
    pool: CpuPool
}

impl ThreadPoolExecutor {
    pub fn new(threads: usize, prefix: &str) -> Result<ThreadPoolExecutor, io::Error> {
        let new_executor = CoreExecutor::with_name(&format!("{}executor", prefix))?;
        Ok(ThreadPoolExecutor::with_executor(threads, prefix, new_executor))
    }

    pub fn with_executor(threads: usize, prefix: &str, executor: CoreExecutor) -> ThreadPoolExecutor {
        let pool = Builder::new()
            .pool_size(threads)
            .name_prefix(prefix)
            .create();
        ThreadPoolExecutor { pool, executor }
    }

    pub fn schedule_fixed_rate<F>(&self, interval: Duration, scheduled_fn: F)
        where F: Fn(&Remote) + Send + Sync + 'static
    {
        let pool_clone = self.pool.clone();
        let arc_fn = Arc::new(scheduled_fn);
        self.executor.schedule_fixed_rate(interval, move |handle| {
            let arc_fn_clone = arc_fn.clone();
            let remote = handle.remote().clone();
            let t = pool_clone.spawn_fn(move || {
                arc_fn_clone(&remote);
                Ok::<(),()>(())
            });
            handle.spawn(t);
        });
    }

    // TODO: make pub(crate)
    pub fn pool(&self) -> &CpuPool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, RwLock};
    use std::thread;
    use std::time::{Duration, Instant};

    use super::{CoreExecutor, ThreadPoolExecutor, calculate_delay};

    #[test]
    fn fixed_interval_test() {
        let timings = Arc::new(RwLock::new(Vec::new()));
        {
            let executor = CoreExecutor::new().unwrap();
            let timings_clone = Arc::clone(&timings);
            executor.schedule_fixed_rate(Duration::from_secs(1), move |_handle| {
                timings_clone.write().unwrap().push(Instant::now());
            });
            thread::sleep(Duration::from_millis(5500));
        }

        let timings = timings.read().unwrap();
        assert!(timings.len() == 6);
        for i in 1..6 {
            let execution_interval = timings[i] - timings[i-1];
            assert!(execution_interval < Duration::from_millis(1020));
            assert!(execution_interval > Duration::from_millis(980));
        }
    }

    #[test]
    fn fixed_interval_slow_task_test() {
        let counter = Arc::new(RwLock::new(0));
        let counter_clone = Arc::clone(&counter);
        {
            let executor = CoreExecutor::new().unwrap();
            executor.schedule_fixed_interval(Duration::from_secs(1), move |_handle| {
                // TODO: use atomic int when available
                let counter = {
                    let mut counter = counter_clone.write().unwrap();
                    (*counter) += 1;
                    *counter
                };
                if counter == 1 {
                    thread::sleep(Duration::from_secs(3));
                }
            });
            thread::sleep(Duration::from_millis(5500));
        }
        assert_eq!(*counter.read().unwrap(), 4);
    }

    #[test]
    fn calculate_delay_test() {
        fn s(n: u64) -> Duration { Duration::from_secs(n) };
        assert_eq!(calculate_delay(s(10), s(3), s(0)), (s(7), s(0)));
        assert_eq!(calculate_delay(s(10), s(11), s(0)), (s(0), s(1)));
        assert_eq!(calculate_delay(s(10), s(3), s(3)), (s(4), s(0)));
        assert_eq!(calculate_delay(s(10), s(3), s(9)), (s(0), s(2)));
        assert_eq!(calculate_delay(s(10), s(12), s(15)), (s(0), s(17)));
    }

    #[test]
    fn fixed_rate_test() {
        let counter = Arc::new(RwLock::new(0));
        let counter_clone = Arc::clone(&counter);
        {
            let executor = CoreExecutor::new().unwrap();
            executor.schedule_fixed_rate(Duration::from_secs(1), move |_handle| {
                let mut counter = counter_clone.write().unwrap();
                (*counter) += 1;
            });
            thread::sleep(Duration::from_millis(5500));
        }
        assert_eq!(*counter.read().unwrap(), 6);
    }

    #[test]
    fn fixed_rate_slow_task_test() {
        let counter = Arc::new(RwLock::new(0));
        let counter_clone = Arc::clone(&counter);
        {
            let executor = CoreExecutor::new().unwrap();
            executor.schedule_fixed_rate(Duration::from_secs(1), move |_handle| {
                // TODO: use atomic int when available
                let counter = {
                    let mut counter = counter_clone.write().unwrap();
                    (*counter) += 1;
                    *counter
                };
                if counter == 1 {
                    thread::sleep(Duration::from_secs(3));
                }
            });
            thread::sleep(Duration::from_millis(5500));
        }
        assert_eq!(*counter.read().unwrap(), 6);
    }

    #[test]
    fn fixed_rate_slow_task_test_pool() {
        let counter = Arc::new(RwLock::new(0));
        let counter_clone = Arc::clone(&counter);
        {
            let executor = ThreadPoolExecutor::new(20, "pool-").unwrap();
            executor.schedule_fixed_rate(Duration::from_secs(1), move |_remote| {
                // TODO: use atomic int when available
                let counter = {
                    let mut counter = counter_clone.write().unwrap();
                    (*counter) += 1;
                    *counter
                };
                if counter == 1 {
                    thread::sleep(Duration::from_secs(3));
                }
            });
            thread::sleep(Duration::from_millis(5500));
        }
        assert_eq!(*counter.read().unwrap(), 6);
    }
}

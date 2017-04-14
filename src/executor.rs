use futures::future::Future;
use futures::sync::oneshot::{channel, Sender};
use tokio_core::reactor::{Core, Handle, Remote};
use tokio_core::reactor::Timeout;

use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use std::io;


fn timer_loop<F>(scheduled_fn: Arc<F>, interval: Duration, handle: &Handle)
    where F: Fn(&Handle) + Send + 'static
{
    let handle_clone = handle.clone();
    let scheduled_fn_clone = scheduled_fn.clone();
    let t = Timeout::new(interval, handle).unwrap()
        .then(move |_| {
            timer_loop(scheduled_fn_clone, interval, &handle_clone);
            Ok::<(), ()>(())
        });
    handle.spawn(t);
    scheduled_fn(&handle);
}

pub struct Executor {
    remote: Remote,
    termination_sender: Sender<()>,
    thread_handle: JoinHandle<()>,
}

impl Executor {
    pub fn new(thread_name: &str) -> Result<Executor, io::Error> {
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
        let executor = Executor {
            remote: core_rx.wait().expect("Failed to receive remote"),
            termination_sender: termination_tx,
            thread_handle: thread_handle,
        };
        debug!("Executor created");
        Ok(executor)
    }

    pub fn stop(self) {
        let _ = self.termination_sender.send(());
        let _ = self.thread_handle.join();
    }

    pub fn schedule_fixed_rate<F>(&self, interval: Duration, scheduled_fn: F)
        where F: Fn(&Handle) + Send + 'static
    {
        self.remote.spawn(move |handle| {
            timer_loop(Arc::new(scheduled_fn), interval, handle);
            Ok::<(), ()>(())
        });
    }
}


#[cfg(test)]
mod tests {
    use std::sync::{Arc, RwLock};
    use std::thread;
    use std::time::{Duration, Instant};
    use Executor;

    #[test]
    fn fixed_rate_test() {
        let timings = Arc::new(RwLock::new(Vec::new()));
        let executor = Executor::new("executor").unwrap();
        let timings_clone = Arc::clone(&timings);
        executor.schedule_fixed_rate(Duration::from_secs(1), move |_handle| {
            timings_clone.write().unwrap().push(Instant::now());
        });
        thread::sleep(Duration::from_millis(5200));
        executor.stop();

        let timings = timings.read().unwrap();
        assert!(timings.len() == 6);
        for i in 1..6 {
            let execution_interval = timings[i] - timings[i-1];
            assert!(execution_interval < Duration::from_millis(1020));
            assert!(execution_interval > Duration::from_millis(980));
        }
    }
}

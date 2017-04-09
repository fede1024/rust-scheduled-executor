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
    _thread_handle: JoinHandle<()>,
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
                    Ok(v) => println!("Core terminated correctly {:?}", v),
                    Err(e) => println!("Core terminated with error: {:?}", e),
                }
            })?;
        let executor = Executor {
            remote: core_rx.wait().expect("Failed to receive remote"),
            termination_sender: termination_tx,
            _thread_handle: thread_handle,
        };
        debug!("Executor created");
        Ok(executor)
    }

    pub fn stop(self) {
        let _ = self.termination_sender.send(());
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


// TODO: write proper tests
#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;
    use Executor;

    #[test]
    fn fixed_rate_test() {
        let executor = Executor::new("executor").unwrap();
        println!("Started");
        let i = 0;
        executor.schedule_fixed_rate(Duration::from_secs(1), move |_handle| {
            println!("> LOOOLL {}", i);
        });
        thread::sleep(Duration::from_secs(5));
        println!("Terminating core");
        executor.stop();
        thread::sleep(Duration::from_secs(1));
        println!("The end");
    }
}

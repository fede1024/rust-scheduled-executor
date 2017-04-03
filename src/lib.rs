extern crate tokio_timer;
extern crate futures;
extern crate tokio_core;
extern crate futures_cpupool;
extern crate rand;

use futures::future::Future;
use futures::stream::Stream;
use futures::sync::oneshot::{channel, Receiver, Sender};
use futures_cpupool::{Builder, CpuPool};
use tokio_core::reactor::{Core, Handle, Remote};
use tokio_timer::Timer;
use tokio_core::reactor::Timeout;

use std::cell::Cell;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};


pub struct ExecutorCore {
    pub remote: Remote,
    termination_sender: Sender<()>,
}

impl ExecutorCore {
    fn new(pool_size: usize) -> ExecutorCore {
        let (remote, termination_sender) = ExecutorCore::start_core_thread();
        ExecutorCore {
            remote: remote,
            termination_sender: termination_sender,
        }
    }

    fn start_core_thread() -> (Remote, Sender<()>) {
        let (termination_tx, termination_rx) = channel();
        let (core_tx, core_rx) = channel();
        thread::spawn(move || {
            println!("Core starting");
            let mut core = Core::new().expect("Failed to start core");
            core_tx.complete(core.remote());
            match core.run(termination_rx) {
                Ok(v) => println!("Core terminated correctly {:?}", v),
                Err(e) => println!("Core terminated with error: {:?}", e),
            }
        });
        println!("Waiting for remote");
        let remote = core_rx.wait().expect("Failed to receive remote");
        println!("Remote received");
        (remote, termination_tx)
    }

    pub fn stop(self) {
        self.termination_sender.complete(());
    }

    fn timeouts(&self) {
        let remote = self.remote.clone();
        let f = self.remote.spawn(move |handle| {
            let t = Timeout::new(Duration::from_secs(1), handle).unwrap()
                .then(move |_| {
                    println!("YOOOO");
                    let h = remote.handle().unwrap();
                    for i in 0..3 {
                        let j = i;
                        let p = Timeout::new(Duration::from_secs(i), &h).unwrap()
                            .then(move |_| {
                                println!(">> {}", i);
                                Ok::<(),()>(())
                            });
                        h.spawn(p);
                    }
                    Ok::<(),()>(())
                });
            handle.spawn(t);
            Ok::<(),()>(())
        });
    }

}

#[cfg(test)]
mod tests {

    use std::thread;
    use ExecutorCore;

    #[test]
    fn core_test() {
        let core = ExecutorCore::new(4);
        core.timeouts();
        println!("Started");
        thread::sleep_ms(5000);
        println!("Terminating core");
        core.stop();
        thread::sleep_ms(2000);
        println!("The end");
    }
}

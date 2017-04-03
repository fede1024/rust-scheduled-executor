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

use std::cell::Cell;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};


#[derive(Debug)]
enum CoreSignal {
    Terminate
}

struct ExecutorCoreInner {
    remote: Remote,
    cpu_pool: CpuPool,
    timer: Timer,
    termination_sender: Sender<CoreSignal>,
}

pub struct ExecutorCore {
    inner: Arc<ExecutorCoreInner>
}

impl ExecutorCore {
    fn new(pool_size: usize) -> ExecutorCore {
        let (remote, termination_sender) = ExecutorCore::start_core_thread();
        let inner = ExecutorCoreInner {
            remote: remote,
            cpu_pool: Builder::new().pool_size(pool_size).name_prefix("pool").create(),
            timer: Timer::default(),
            termination_sender: termination_sender,
        };
        ExecutorCore { inner: Arc::new(inner) }
    }

    fn start_core_thread() -> (Remote, Sender<CoreSignal>) {
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

    fn task_group<F, V>(&self, total_time: Duration, func: &'static F) -> TaskGroup<F, V>
            where F: Fn(&V) + Send + Sync + 'static, V: Send + Sync + 'static {
        TaskGroup {
            inner_core: self.inner.clone(),
            total_time: total_time,
            func: func,
            _p: PhantomData
        }
    }
}

pub struct TaskGroup<F, V> where F: Fn(&V) + Send + Sync + 'static, V: Send + Sync + 'static {
    inner_core: Arc<ExecutorCoreInner>,
    total_time: Duration,
    func: &'static F,
    _p: PhantomData<V>
    //values: Vec<(V, Instant)>,
}

impl<F, V> TaskGroup<F, V> where F: Fn(&V) + Send + Sync + 'static, V: Send + Sync + 'static {
    fn add_task(&mut self, value: V) {
        //let rand_delay = Duration::from_secs(rand::random::<f32>() % Duration::as;
        TaskGroup::reschedule(self.func, value, Duration::from_secs((1)), &self.inner_core.remote, self.inner_core.timer.clone());
        // self.values.push((value, Instant::now()));
    }

    fn reschedule(func: &'static F, value: V, delay: Duration, remote: &Remote, timer: Timer) {
        let timer_clone = timer.clone();
        remote.spawn(move |_| {
            let sleep = timer_clone.sleep(delay)
                .then(move |_| {
                    println!("LOL");
                    func(&value);
                    // TaskGroup::reschedule(func, value, delay, remote, timer);
                    Ok::<(), ()>(())
                });
            sleep
        });
    }
}

#[cfg(test)]
mod tests {
    use tokio_timer::*;
    use futures::*;
    use std::time::*;
    use tokio_core::reactor::{Core, Handle};
    use futures_cpupool::Builder;
    use std::thread;
    use futures::sync::oneshot::{channel, Receiver, Sender};

    use ExecutorCore;
    use TaskGroup;


    #[test]
    fn tasks_test() {
        let core = ExecutorCore::new(4);
        let mut task_group = core.task_group(Duration::from_secs(1), |v| println!(">> {:?}", v));

        println!("Adding task");
        task_group.add_task("AAAA");
        println!("Ending");

        thread::sleep_ms(30000);
    }

    #[test]
    fn timer_test() {
        // let timer = Timer::default();

        // // let mut refresh = timer.interval(Duration::from_secs(3));

        // // loop {
        // //     let mut tick = timer.interval(Duration::from_secs(1));
        // //     for x in tick.select(refresh).wait() {
        // //         println!(">> {:?}", x);
        // //     }
        // // }

        // let mut core = Core::new().unwrap();
        // let cpu_pool = Builder::new().pool_size(4).name_prefix("sched_pool").create();

        // let handle = core.handle();
        // let ticker = timer.interval(Duration::from_secs(1))
        //     .map_err(|_| ())
        //     .for_each(move |_| {
        //         println!("TICK");
        //         let f = cpu_pool.spawn_fn(|| {
        //             println!("  in {:?}", thread::current().name());
        //             thread::sleep_ms(1450);
        //             println!("  out {:?}", thread::current().name());
        //             Ok::<(),()>(())
        //         });
        //         handle.spawn(f);
        //         Err(())
        //         // Ok(())
        //     });
        // core.handle().spawn(ticker);

        // let sleep = timer.sleep(Duration::from_secs(5));

        // core.run(sleep).unwrap();
        // println!("DONE");
        // for tick in ticks.wait() {
        //     println!("TICK");
        // }

        // let sleep = timer.sleep(Duration::from_secs(10))
        //     .map(|_| (0, timer.interval(Duration::from_secs(3))))
        //     .map_err(|_| ());

        //loop {
            // let tick_future = tick.into_future()
            //     .map(|(_, p)| (0, p))
            //     .map_err(|_| ());
            // let refresh_future = refresh.into_future()
            //     .map(|(_, p)| (1, p))
            //     .map_err(|_| ());
            //let i : i32 = tick.select(refresh).wait();
            // if let Ok(((n, r), _)) = tick.select(refresh).wait() {
            //     println!("TOCK {}", n);
            // } else {
            //     break;
            // }
            //let (_, ticks) = stream_future.wait().unwrap();
        //}
    }
}

extern crate tokio_timer;
extern crate futures;
extern crate tokio_core;
extern crate futures_cpupool;

use tokio_core::reactor::{Core, Handle};
use futures_cpupool::{Builder, CpuPool};
use tokio_timer::Timer;
use futures::stream::Stream;
use futures::sync::oneshot::{channel, Receiver, Sender};

use std::time::{Duration, Instant};
use std::cell::Cell;
use std::sync::atomic::AtomicUsize;
use std::thread;
use std::cell::RefCell;

pub struct ExecutorCore {
    handle: Handle,
    cpu_pool: CpuPool,
    timer: Timer,
    termination_sender: Sender<()>,
}

impl ExecutorCore {
    fn new(pool_size: usize) -> ExecutorCore {
        let (tx, rx) = channel();
        let mut core = Core::new().unwrap();
        let remote = core.remote();
        let handle = core.handle();
        thread::spawn(move || {
            println!("Core starting");
            core.run(rx).unwrap();
            println!("Core terminated");
        });
        ExecutorCore {
            handle: handle,
            cpu_pool: Builder::new().pool_size(pool_size).name_prefix("pool").create(),
            timer: Timer::default(),
            termination_sender: tx,
        }
    }

    fn task_group<F, V>(&self, total_time: Duration, func: F) -> TaskGroup<F, V>
            where F: Fn(V) + 'static, V: Send + 'static {
        TaskGroup {
            total_time: total_time,
            values: Vec::new(),
            func: func,
            last_run: Cell::new(Instant::now()),
            generation_id: AtomicUsize::new(0),
        }
    }
}

pub struct TaskGroup<F, V> where F: Fn(V), V: Send + 'static {
    total_time: Duration,
    func: F,
    values: Vec<V>,
    last_run: Cell<Instant>,
    generation_id: AtomicUsize,
}

impl<F, V> TaskGroup<F, V> where F: Fn(V) + 'static, V: Send + 'static {
    // fn add_task(&mut self, value: V) {
    //     self.values.push(value);
    //     let interval = self.total_time / (self.values.len() as u32);
    //     let ticker = self.executor_inner.timer.interval(interval)
    //         .map_err(|_| ())
    //         .for_each(move |_| {
    //             println!("TICK");
    //             // Err(())
    //             Ok(())
    //         });
    //     self.executor_inner.handle.spawn(ticker);
    //     // thread::sleep_ms(10000);
    // }
}

#[cfg(test)]
mod tests {
    use tokio_timer::*;
    use futures::*;
    use std::time::*;
    use tokio_core::reactor::{Core, Handle};
    use futures_cpupool::Builder;
    use std::thread;

    use ExecutorCore;
    use TaskGroup;

    fn schedule(timer: Timer, handle: Handle) {
        let timer_clone = timer.clone();
        let handle_clone = handle.clone();
        let sleep = timer.sleep(Duration::from_secs(1))
            .then(move |_| {
                println!("BLOOP");
                schedule(timer_clone, handle_clone);
                Ok(())
            });
        handle.spawn(sleep);
    }

    #[test]
    fn sleep_loop() {
        let timer = Timer::default();
        let mut core = Core::new().unwrap();

        schedule(timer.clone(), core.handle());

        core.run(timer.sleep(Duration::from_secs(5)));
        println!("PUFF");
    }


    #[test]
    fn tasks_test() {
        // let core = ExecutorCore::new(4);
        // let mut task_group = core.task_group(Duration::from_secs(1), |v| println!(">> {:?}", v));

        // task_group.add_task("AAAA");
        // core.run();
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

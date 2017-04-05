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

fn schedule_tasks<G: TaskGroup>(task_group: Arc<G>, interval: Duration, remote: &Remote) {
    let tasks = task_group.get_tasks();
    if tasks.is_empty() {
        return
    }
    let gap = interval / tasks.len() as u32;
    remote.spawn(move |handle| {
        for (i, task) in tasks.into_iter().enumerate() {
            // Use loop and single timeout every time instead, to iterate over the loop
            let task_group_clone = task_group.clone();
            let t = Timeout::new(gap * i as u32, handle).unwrap()
                .then(move |_| {
                    task_group_clone.execute(task);
                    Ok::<(), ()>(())
                });
            handle.spawn(t);
        }
        Ok::<(), ()>(())
    });
}

fn schedule_group_refresh<G: TaskGroup>(task_group: Arc<G>, interval: Duration, remote: &Remote) {
    schedule_tasks(task_group.clone(), interval, remote);
    let remote_clone = remote.clone();
    remote.spawn(move |handle| {
        // Re-schedule itself with delay
        Timeout::new(interval, handle).unwrap()
            .then(move |_| {
                schedule_group_refresh(task_group, interval, &remote_clone);
                Ok::<(), ()>(())
            })
    });
}

pub trait TaskGroup: Send + Sync + 'static {
    type TaskId: Send;
    fn get_tasks(&self) -> Vec<Self::TaskId>;
    fn execute(&self, Self::TaskId);
}

pub struct TaskGroupExecutor {
    pub remote: Remote,
    termination_sender: Sender<()>,
}

impl TaskGroupExecutor {
    fn new(pool_size: usize) -> TaskGroupExecutor {
        let (remote, termination_sender) = start_core_thread();
        TaskGroupExecutor {
            remote: remote,
            termination_sender: termination_sender,
        }
    }

    pub fn stop(self) {
        self.termination_sender.complete(());
    }

    fn run_task_group<G: TaskGroup>(&self, task_group: G, interval: Duration) {
        schedule_group_refresh(Arc::new(task_group), interval, &self.remote);
    }
}


#[cfg(test)]
mod tests {

    use std::thread;
    use std::time::Duration;
    use TaskGroupExecutor;
    use TaskGroup;

    struct MyGroup;

    impl TaskGroup for MyGroup {
        type TaskId = i32;

        fn get_tasks(&self) -> Vec<i32> {
            println!("get tasks");
            vec![0, 1, 2, 3, 4]
        }

        fn execute(&self, task_id: i32) {
            println!("EXECUTE: {}", task_id);
        }
    }

    #[test]
    fn core_test() {
        let executor = TaskGroupExecutor::new(5);
        let group = MyGroup;
        executor.run_task_group(group, Duration::from_secs(2));
        println!("Started");
        thread::sleep_ms(20000);
        println!("Terminating core");
        executor.stop();
        thread::sleep_ms(1000);
        println!("The end");
    }
}

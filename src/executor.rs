use futures::future::Future;
use futures::sync::oneshot::{channel, Sender};
use futures_cpupool::CpuPool;
use tokio_core::reactor::{Core, Handle, Remote};
use tokio_core::reactor::Timeout;

use task_group::{TaskGroup, schedule_tasks};

use std::sync::Arc;
use std::thread;
use std::time::Duration;


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
}

impl Executor {
    // use logging
    pub fn new() -> Executor {
        let (termination_tx, termination_rx) = channel();
        let (core_tx, core_rx) = channel();
        thread::spawn(move || {
            println!("Core starting");
            let mut core = Core::new().expect("Failed to start core");
            let _ = core_tx.send(core.remote());
            match core.run(termination_rx) {
                Ok(v) => println!("Core terminated correctly {:?}", v),
                Err(e) => println!("Core terminated with error: {:?}", e),
            }
        });
        println!("Waiting for remote");
        let remote = core_rx.wait().expect("Failed to receive remote");
        println!("Remote received");
        Executor {
            remote: remote,
            termination_sender: termination_tx,
        }
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

    pub fn run_task_group<G: TaskGroup>(&self, task_group: G, interval: Duration, cpu_pool: Option<CpuPool>) {
        let task_group = Arc::new(task_group);
        self.schedule_fixed_rate(interval, move |handle| {
            schedule_tasks(task_group.clone(), interval, handle, cpu_pool.clone());
        });
    }
}


// TODO: write proper tests
#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;
    use tokio_core::reactor::Handle;
    use futures_cpupool::Builder;
    use Executor;
    use task_group::TaskGroup;

    #[test]
    fn fixed_rate_test() {
        let executor = Executor::new();
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

    struct MyGroup;

    impl TaskGroup for MyGroup {
        type TaskId = i32;

        fn get_tasks(&self) -> Vec<i32> {
            println!("get tasks");
            vec![0, 1, 2, 3, 4]
        }

        fn execute(&self, task_id: i32, handle: Option<Handle>) {
            println!("TASK: {} {:?} {:?}", task_id, thread::current().name(), handle);
        }
    }

    #[test]
    fn task_group_test() {
        let executor = Executor::new();
        let group = MyGroup;
        let cpu_pool = Builder::new().name_prefix("lol-").pool_size(4).create();
        executor.run_task_group(group, Duration::from_secs(1), Some(cpu_pool));
        println!("Started");
        thread::sleep(Duration::from_secs(5));
        println!("Terminating core");
        executor.stop();
        thread::sleep(Duration::from_secs(1));
        println!("The end");
    }
}

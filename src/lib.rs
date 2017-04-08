extern crate futures;
extern crate tokio_core;
extern crate futures_cpupool;

use futures::future::Future;
use futures::sync::oneshot::{channel, Sender};
use futures_cpupool::CpuPool;
use tokio_core::reactor::{Core, Handle, Remote};
use tokio_core::reactor::Timeout;

use std::sync::Arc;
use std::thread;
use std::time::Duration;


fn start_core_thread() -> (Remote, Sender<()>) {
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
    (remote, termination_tx)
}

fn schedule_tasks<G: TaskGroup>(task_group: Arc<G>, interval: Duration, handle: &Handle, pool: Option<CpuPool>) {
    let tasks = task_group.get_tasks();
    if tasks.is_empty() {
        return
    }
    let task_interval = interval / tasks.len() as u32;
    for (i, task) in tasks.into_iter().enumerate() {
        let task_group_clone = task_group.clone();

        match pool {
            Some(ref cpu_pool) => {
                let t = Timeout::new(task_interval * i as u32, handle).unwrap()
                    .then(move |_| {
                        task_group_clone.execute(task, None);
                        Ok::<(), ()>(())
                    });
                handle.spawn(cpu_pool.spawn(t));
            }
            None => {
                let handle_clone = handle.clone();
                let t = Timeout::new(task_interval * i as u32, handle).unwrap()
                    .then(move |_| {
                        task_group_clone.execute(task, Some(handle_clone));
                        Ok::<(), ()>(())
                    });
                handle.spawn(t);
            }
        }
    }
}

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

pub trait TaskGroup: Send + Sync + 'static {
    type TaskId: Send;
    fn get_tasks(&self) -> Vec<Self::TaskId>;
    fn execute(&self, Self::TaskId, Option<Handle>);
}

pub struct TaskGroupExecutor {
    remote: Remote,
    termination_sender: Sender<()>,
}

impl TaskGroupExecutor {
    pub fn new() -> TaskGroupExecutor {
        let (remote, termination_sender) = start_core_thread();
        TaskGroupExecutor {
            remote: remote,
            termination_sender: termination_sender,
        }
    }

    pub fn stop(self) {
        let _ = self.termination_sender.send(());
    }

    pub fn run_task_group<G: TaskGroup>(&self, task_group: G, interval: Duration, cpu_pool: Option<CpuPool>) {
        let task_group = Arc::new(task_group);
        self.schedule_rate(interval, move |handle| {
            schedule_tasks(task_group.clone(), interval, handle, cpu_pool.clone());
        });
    }

    pub fn schedule_rate<F>(&self, interval: Duration, scheduled_fn: F)
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

    use std::thread;
    use std::time::Duration;
    use tokio_core::reactor::Handle;
    use futures_cpupool::Builder;
    use TaskGroupExecutor;
    use TaskGroup;

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
        let executor = TaskGroupExecutor::new();
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

    #[test]
    fn fixed_rate_test() {
        let executor = TaskGroupExecutor::new();
        println!("Started");
        let i = 0;
        executor.schedule_rate(Duration::from_secs(1), move |_handle| {
            println!("> LOOOLL {}", i);
        });
        thread::sleep(Duration::from_secs(5));
        println!("Terminating core");
        executor.stop();
        thread::sleep(Duration::from_secs(1));
        println!("The end");
    }
}

use futures::future::Future;
use futures_cpupool::CpuPool;
use tokio_core::reactor::Handle;
use tokio_core::reactor::Timeout;

use executor::Executor;

use std::sync::Arc;
use std::time::Duration;

pub trait TaskGroup: Send + Sync + Sized + 'static {
    type TaskId: Send;

    fn get_tasks(&self) -> Vec<Self::TaskId>;
    fn execute(&self, Self::TaskId, Option<Handle>);

    fn schedule(self, interval: Duration, executor: &Executor, cpu_pool: Option<CpuPool>) -> Arc<Self> {
        let task_group = Arc::new(self);
        let task_group_clone = task_group.clone();
        executor.schedule_fixed_rate(interval, move |handle| {
            Self::schedule_tasks(&task_group_clone, interval, handle, &cpu_pool);
        });
        task_group
    }

    fn schedule_tasks(task_group: &Arc<Self>, interval: Duration, handle: &Handle, pool: &Option<CpuPool>) {
        let tasks = task_group.get_tasks();
        if tasks.is_empty() {
            return
        }
        let task_interval = interval / tasks.len() as u32;
        for (i, task) in tasks.into_iter().enumerate() {
            let task_group_clone = task_group.clone();

            match pool {
                &Some(ref cpu_pool) => {
                    let t = Timeout::new(task_interval * i as u32, handle).unwrap()
                        .then(move |_| {
                            task_group_clone.execute(task, None);
                            Ok::<(), ()>(())
                        });
                    handle.spawn(cpu_pool.spawn(t));
                }
                &None => {
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
        group.schedule(Duration::from_secs(1), &executor, Some(cpu_pool));
        println!("Started");
        thread::sleep(Duration::from_secs(5));
        println!("Terminating core");
        executor.stop();
        thread::sleep(Duration::from_secs(1));
        println!("The end");
    }
}

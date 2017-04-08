use futures::future::Future;
use futures_cpupool::CpuPool;
use tokio_core::reactor::Handle;
use tokio_core::reactor::Timeout;

use std::sync::Arc;
use std::time::Duration;

pub trait TaskGroup: Send + Sync + 'static {
    type TaskId: Send;
    fn get_tasks(&self) -> Vec<Self::TaskId>;
    fn execute(&self, Self::TaskId, Option<Handle>);
}

pub fn schedule_tasks<G: TaskGroup>(task_group: Arc<G>, interval: Duration, handle: &Handle, pool: Option<CpuPool>) {
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

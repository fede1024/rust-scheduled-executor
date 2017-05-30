use futures::future::Future;
use futures_cpupool::CpuPool;
use tokio_core::reactor::Handle;
use tokio_core::reactor::Timeout;

use executor::CoreExecutor;

use std::sync::Arc;
use std::time::Duration;

pub trait TaskGroup: Send + Sync + Sized + 'static {
    type TaskId: Send;

    fn get_tasks(&self) -> Vec<Self::TaskId>;

    fn execute(&self, Self::TaskId, Option<Handle>);

    fn schedule(self, interval: Duration, executor: &CoreExecutor, cpu_pool: Option<CpuPool>) -> Arc<Self> {
        let task_group = Arc::new(self);
        let task_group_clone = task_group.clone();
        executor.schedule_fixed_interval(interval, move |handle| {
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

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::{Duration, Instant};
    use std::sync::{Arc, RwLock};
    use tokio_core::reactor::Handle;
    use futures_cpupool::Builder;
    use task_group::TaskGroup;

    use CoreExecutor;

    type TaskExecutions = Vec<Vec<Instant>>;
    struct TestGroup {
        executions_lock: Arc<RwLock<TaskExecutions>>,
    }

    impl TestGroup {
        fn new() -> TestGroup {
            let executions = (0..5).map(|_| Vec::new()).collect::<Vec<_>>();
            TestGroup {
                executions_lock : Arc::new(RwLock::new(executions))
            }
        }

        fn executions_lock(&self) -> Arc<RwLock<TaskExecutions>> {
            self.executions_lock.clone()
        }
    }

    impl TaskGroup for TestGroup {
        type TaskId = usize;

        fn get_tasks(&self) -> Vec<usize> {
            vec![0, 1, 2, 3, 4]
        }

        fn execute(&self, task_id: usize, handle: Option<Handle>) {
            let mut executions = self.executions_lock.write().unwrap();
            executions[task_id].push(Instant::now());
            assert!(handle.is_none());
        }
    }

    #[test]
    fn task_group_test() {
        let group = TestGroup::new();
        let executions_lock = group.executions_lock();
        {
            let executor = CoreExecutor::new().unwrap();
            let cpu_pool = Builder::new().name_prefix("pool-thread-").pool_size(4).create();
            group.schedule(Duration::from_secs(4), &executor, Some(cpu_pool));
            thread::sleep(Duration::from_millis(11800));
        }

        let executions = &executions_lock.read().unwrap();
        // There were 5 tasks
        assert!(executions.len() == 5);
        for task in 0..5 {
            // each of them executed 3 times
            assert!(executions[task].len() == 3);
            for run in 1..3 {
                // with 4 seconds between each of them
                let task_interval = executions[task][run] - executions[task][run-1];
                assert!(task_interval < Duration::from_millis(4500));
                assert!(task_interval > Duration::from_millis(500));
            }
        }
        for i in 1..15 {
            let task = i % 5;
            let run = i / 5;
            let task_prev = (i - 1) % 5;
            let run_prev = (i - 1) / 5;
            let inter_task_interval = executions[task][run] - executions[task_prev][run_prev];
            assert!(inter_task_interval < Duration::from_millis(1500));
            assert!(inter_task_interval > Duration::from_millis(500));
        }
    }
}

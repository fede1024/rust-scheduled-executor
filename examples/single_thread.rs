extern crate scheduled_executor;

use scheduled_executor::CoreExecutor;

use std::time::{Duration, Instant};
use std::thread;

fn task_body(task_id: i32, start_time: Instant) {
    println!("> Task {} is being executed", task_id);
    println!("  time elapsed from from start: {:?} seconds", start_time.elapsed().as_secs());
    println!("  thread: {}", thread::current().name().unwrap());
}

fn main() {
    let executor = CoreExecutor::new()
        .expect("Core creation failed");

    let start_time = Instant::now();

    let task_1 = executor.schedule_fixed_rate(
        Duration::from_secs(2),
        Duration::from_secs(5),
        move |_| { task_body(1, start_time) }
    );
    let _task_2 = executor.schedule_fixed_rate(
        Duration::from_secs(3),
        Duration::from_secs(3),
        move |_| { task_body(2, start_time) }
    );

    println!("Tasks have been scheduled");
    thread::sleep(Duration::from_secs(20));
    task_1.stop();
    println!("Task1 has been stopped");
    thread::sleep(Duration::from_secs(10));
    println!("Terminating");
}
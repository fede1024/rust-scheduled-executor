extern crate scheduled_executor;

use scheduled_executor::ThreadPoolExecutor;

use std::time::{Duration, Instant};
use std::thread;

fn main() {
    let executor = ThreadPoolExecutor::new(4)
        .expect("Thread pool creation failed");

    let start_time = Instant::now();

    executor.schedule_fixed_rate(
        Duration::from_secs(2),
        Duration::from_secs(5),
        move |_| {
            println!("> Task is being executed");
            println!("  time elapsed from from start: {:?} seconds", start_time.elapsed().as_secs());
            println!("  thread: {}", thread::current().name().unwrap());
            // Emulate an expensive task
            thread::sleep(Duration::from_secs(10));
        },
    );

    println!("Task has been scheduled");
    thread::sleep(Duration::from_secs(20));
    println!("Terminating");
}
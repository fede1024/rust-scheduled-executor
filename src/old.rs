
fn calc_next_run(curr_time: Instant, last_time: Instant, task_n: u32,
                 total_tasks: u32, total_time: Duration)
                 -> Instant {
    let elapsed = curr_time - last_time;
    let absolute_run = total_time + (total_time / total_tasks) * task_n;
    return curr_time + absolute_run - elapsed;
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};
    use calc_next_run;

    #[test]
    fn calc_next_run_test() {
        let t_0 = Instant::now();
        let t_20 = Instant::now() + Duration::from_secs(20);
        let t_30 = Instant::now() + Duration::from_secs(30);
        println!(">> {:?}", t_0);
        let next_run =  calc_next_run(t_0, t_0, 0, 3, Duration::from_secs(60));
        println!(">> {:?} {:?}", next_run, next_run - t_0);
        let next_run =  calc_next_run(t_20, t_0, 1, 3, Duration::from_secs(60));
        println!(">> {:?} {:?}", next_run, next_run - t_0);
        let next_run =  calc_next_run(t_30, t_20, 2, 3, Duration::from_secs(60));
        println!(">> {:?} {:?}", next_run, next_run - t_0);
    }
}

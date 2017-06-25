extern crate scheduled_executor;
extern crate rand;

use scheduled_executor::{TaskGroup, TaskGroupScheduler, ThreadPoolExecutor};
use rand::random;

use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;


#[derive(Debug)]
enum ServerStatus {
    Up,
    Maintenance,
    Down
}

fn run_healthcheck(server_id: &ServerId) -> ServerStatus {
    println!("Service health: checking server: {:?}", server_id);
    thread::sleep(Duration::from_millis(500)); // emulate expensive operation
    match random::<u8>() % 3 {
        0 => ServerStatus::Up,
        1 => ServerStatus::Maintenance,
        _ => ServerStatus::Down,
    }
}


type ServerId = Ipv4Addr;

#[derive(Clone)]
struct ServiceHealth {
    servers: Arc<Mutex<HashMap<ServerId, ServerStatus>>>
}

impl ServiceHealth {
    fn new() -> ServiceHealth {
        ServiceHealth {
            servers: Arc::new(Mutex::new(HashMap::new()))
        }
    }
}

impl TaskGroup for ServiceHealth {
    type TaskId = ServerId;

    fn get_tasks(&self) -> Vec<ServerId> {
        println!("Service health: getting list of servers");
        thread::sleep(Duration::from_millis(250)); // emulate expensive operation
        vec![
            // TODO: try ServerId instead
            ServerId::from_str("127.0.0.1").unwrap(),
            ServerId::from_str("127.0.0.2").unwrap(),
            ServerId::from_str("127.0.0.3").unwrap(),
            ServerId::from_str("127.0.0.4").unwrap(),
            ServerId::from_str("127.0.0.5").unwrap(),
        ]
    }

    fn execute(&self, server_id: ServerId) {
        let status = run_healthcheck(&server_id);
        let mut servers = self.servers.lock().unwrap();
        (*servers).insert(server_id, status);
    }
}


fn main() {
    let executor = ThreadPoolExecutor::new(4)
        .expect("Thread pool creation failed");

    let service_health = ServiceHealth::new();

    let service_health_monitoring = service_health.clone();
    executor.schedule_fixed_rate(
        Duration::from_secs(5),
        Duration::from_secs(2),
        move |_| {
            println!("Monitoring: {:?}", *service_health_monitoring.servers.lock().unwrap())
        }
    );

    executor.schedule(
        service_health,
        Duration::from_secs(0),
        Duration::from_secs(5),
    );

    println!("Task group has been scheduled");
    thread::sleep(Duration::from_secs(20));
    println!("Terminating");
}
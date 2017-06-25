//! Task groups are used for the periodic executions of different tasks. THe task groups is defined
//! by a `get_task` function, which provides the list of task ids, and the `execute` method,
//! which will receive the task id as argument.
//!
//! This example code simulates the execution of an health check on all the servers of a service.
//! At the beginning of every cycle, the task group will fetch the list of available servers, and
//! it will then health check all of them in turn.
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


/// The status that a health check will return.
#[derive(Debug)]
enum ServerStatus {
    Up,
    Maintenance,
    Down
}

/// Server identifier
type ServerId = Ipv4Addr;

/// Execute the health check on the given server.
fn run_healthcheck(server_id: &ServerId) -> ServerStatus {
    println!("Service health: checking server: {:?}", server_id);
    thread::sleep(Duration::from_millis(500)); // emulate expensive operation
    match random::<u8>() % 3 {
        0 => ServerStatus::Up,
        1 => ServerStatus::Maintenance,
        _ => ServerStatus::Down,
    }
}


/// Stores the health of the service
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

    /// Return the list of task ids. In this example, the list of tasks corresponds to the list
    /// of servers in the service.
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

    /// Executes the code on a particular task. In this example the code will run the health check
    /// on the server.
    fn execute(&self, server_id: ServerId) {
        let status = run_healthcheck(&server_id);
        let mut servers = self.servers.lock().unwrap();
        (*servers).insert(server_id, status);
    }
}


fn main() {
    /// Create a multi threaded executor
    let executor = ThreadPoolExecutor::new(4)
        .expect("Thread pool creation failed");

    let service_health = ServiceHealth::new();

    // Use another reference to the service health to periodically print the status of the
    // service.
    let service_health_monitoring = service_health.clone();
    executor.schedule_fixed_rate(
        Duration::from_secs(5),
        Duration::from_secs(2),
        move |_| {
            println!("Monitoring: {:?}", *service_health_monitoring.servers.lock().unwrap())
        }
    );

    // Use the executor to schedule the task group.
    executor.schedule(
        service_health,
        Duration::from_secs(0),
        Duration::from_secs(5),
    );

    println!("Task group has been scheduled");
    thread::sleep(Duration::from_secs(20));
    println!("Terminating");
}
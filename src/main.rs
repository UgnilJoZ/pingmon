use std::process::Command;
use std::thread;
extern crate systemd;
//use systemd::daemon;

#[derive(Debug)]
enum PingError {
	ErrProcess,
	ErrThread,
	ErrPing(std::process::Output),
}

fn ping(host: &str, wait_secs: u16) -> Result<(), PingError> {
	let output = match Command::new("ping").args(&[host, "-c1", &format!("-w{}", wait_secs)]).output() {
		Err(_) => return Err(PingError::ErrProcess),
		Ok(output) => output,
	};

	if output.status.success() {
		Ok(())
	} else {
		Err(PingError::ErrPing(output))
	}
}

fn ping_many(hosts: &[String], wait_secs: u16) -> Vec<Result<(), PingError>> {
	// Spawn each ping in it's own thread
	let mut children = vec![];
	for host in hosts {
		let host = host.clone();
		children.push(thread::spawn(move || -> Result<(), PingError>  { ping(&host, wait_secs) }));
	}

	// Collect the status of the pings in the right order
	children
		.into_iter()
		.map(|c| c.join().unwrap_or(Err(PingError::ErrThread)))
		.collect()
}

fn main() {
	let hosts = ["1.1.1.1".to_string(), "8.8.8.8".to_string(), "8.7.6.4".to_string()];
	println!("{:?}", ping_many(&hosts, 1));
}

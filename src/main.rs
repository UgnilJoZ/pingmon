use std::{env,thread};
use std::process::{self,Command};
use std::time::Duration;
use std::fs::File;
extern crate systemd;
use systemd::{daemon,journal};
use std::io::{BufRead,BufReader};
use std::collections::HashMap;

fn ping(host: &str, wait_secs: u16) -> String {
	match Command::new("ping").args(&[host, "-c1", &format!("-w{}", wait_secs)]).output() {
		Err(_) => "THREAD_ERROR",
		Ok(output) => {
			if output.status.success() {
				"UP"
			} else {
				// If ping exit code is 2, the argument was unresolvable
				if output.status.code() == Some(2)
					{ "UNRESOLVABLE" }
				else
					{ "DOWN" }
			}
		}
	}.to_string()
}

fn ping_many(hosts: &[String], wait_secs: u16, resultmap: &mut HashMap<String, String>) {
	// Spawn each ping in it's own thread
	let mut children = vec![];
	for host in hosts {
		let host_cloned = host.clone();
		children.push((host, thread::spawn(move || ping(&host_cloned, wait_secs) )));
	}

	// Collect the status of the pings
	while let Some((host, thread)) = children.pop() {
		let result = thread.join().unwrap_or("THREAD_ERROR".to_string());
		match resultmap.insert(host.clone(), result.clone()) {
			// None: The value was not in the map before.
			None =>	{
				let message = format!("Host {} starts as {}", host, result);
				journal::send(&[
					      &format!("MESSAGE={}", message),
					      &format!("HOST={}", host),
					      &format!("HOST_STATUS={}", result),
					      &"EVENT=FIRSTPING"]);
			},
			// Some: the value was in there
			Some(old_value) => 
				if result != old_value {
					let message = format!("Host {} turns {}", host, result);
					journal::send(&[
						      &format!("MESSAGE={}", message),
						      &format!("HOST={}", host),
						      &format!("HOST_STATUS={}", result),
						      &"EVENT=STATUSCHANGE"]);
				},
		}
	}
}

fn main() {
	// Read whitespace separated $PINGMON_HOSTS or the lines of $PINGMON_HOSTSFILE
	let mut hosts: Vec<String> = match env::var_os("PINGMON_HOSTS") {
		Some(val) => val
			.into_string().expect("$PINGMON_HOSTS has to be valid UTF-8")
			.split_whitespace().map(|l| String::from(l)).collect(),
		None => match env::var_os("PINGMON_HOSTSFILE") {
			Some(file) => {
				let file = File::open(file).expect("I could not read the file you gave me as PINGMON_HOSTSFILE.");
				BufReader::new(file).lines().map(|l| l.unwrap()).collect()
			},
			None => {
				println!("You have to give me the environment variables PINGMON_HOSTS xor PINGMON_HOSTSFILE!");
				process::exit(1);
			}
		},
	};
	hosts.reverse();
	// Other params
	let sleep_time = match env::var_os("PINGMON_SLEEP") {
		Some(val) => val.into_string().expect("$PINGMON_SLEEP has to be valid UTF-8").parse().expect("PINGMON_SLEEP has to be a number"),
		None => 10,
	};
	let deadline_secs = match env::var_os("PINGMON_TIMEOUT") {
		Some(val) => val.into_string().expect("$PINGMON_TIMEOUT has to be valid UTF-8").parse().expect("PINGMON_TIMEOUT has to be a number"),
		None => 1,
	};

	// Initialize ping status storage with current state
	let mut map = HashMap::new();
	ping_many(&hosts, deadline_secs, &mut map);

	// Notify systemd
	let notify_fields: HashMap<&str, &str> = [("READY", "1")].iter().cloned().collect();
	if daemon::notify(false, notify_fields).is_err() {
		println!("Startup complete");
	}
	
	// Main loop
	loop {
		thread::sleep(Duration::from_secs(sleep_time));
		ping_many(&hosts, deadline_secs, &mut map);
	}
}

use std::process;
use std::process::Command;
use std::thread;
extern crate systemd;
//use systemd::daemon;
use std::env;
use std::io::BufReader;
use std::io::BufRead;
use std::fs::File;

#[derive(Debug)]
enum PingError {
	Unresolvable,
	Down,
	Thread,
}

fn pingresult_to_str(val: Result<(), PingError>) -> String {
	match val {
		Ok(_) => "UP",
		Err(e) => match e {
			PingError::Unresolvable => "UNRESOLVABLE",
			PingError::Down => "DOWN",
			PingError::Thread => "THREAD_ERROR",
		},
	}.to_string()
}

fn ping(host: &str, wait_secs: u16) -> Result<(), PingError> {
	let output = match Command::new("ping").args(&[host, "-c1", &format!("-w{}", wait_secs)]).output() {
		Err(_) => return Err(PingError::Down),
		Ok(output) => output,
	};

	if output.status.success() {
		Ok(())
	} else {
		// If ping exit code is 2, the argument was unresolvable
		if output.status.code() == Some(2)
			{ Err(PingError::Unresolvable) }
		else
			{ Err(PingError::Down) }
	}
}

fn ping_many(hosts: &[String], wait_secs: u16) -> Vec<String> {
	// Spawn each ping in it's own thread
	let mut children = vec![];
	for host in hosts {
		let host = host.clone();
		children.push(thread::spawn(move || ping(&host, wait_secs) ));
	}

	// Collect the status of the pings in the right order
	children
		.into_iter()
		.map(|c| c.join().unwrap_or(Err(PingError::Thread)))
		.map(|p| pingresult_to_str(p))
		.collect()
}

fn main() {
	// Read whitespace separated $PINGMON_HOSTS or the lines of $PINGMON_HOSTSFILE
	let hosts: Vec<String> = match env::var_os("PINGMON_HOSTS") {
		Some(val) => val.into_string().expect("$PINGMON_HOSTS has to be valid UTF-8").split_whitespace().map(|l| String::from(l)).collect(),
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
	println!("{:?}", ping_many(&hosts, 1));
}

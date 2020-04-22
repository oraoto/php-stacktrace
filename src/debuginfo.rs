use std;
use std::fs;
use std::fs::File;
use std::io::{self, BufRead};
use std::process;
use std::process::{Command, Stdio};
use regex::Regex;
use read_process_memory::*;

pub fn get_executor_globals_address<Pid>(pid: Pid) -> usize
where
    Pid: TryIntoProcessHandle + std::fmt::Display + Copy,
{
    get_maps_address(pid) + get_nm_address(pid)
}

fn get_nm_address<Pid>(pid: Pid) -> usize
where
    Pid: TryIntoProcessHandle + std::fmt::Display,
{
    let nm_command = Command::new("nm")
        .arg("-D")
        .arg(format!("/proc/{}/exe", pid))
        .stdout(Stdio::piped())
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {}", e));
    if !nm_command.status.success() {
        panic!(
            "failed to execute process: {}",
            String::from_utf8(nm_command.stderr).unwrap()
        )
    }

    let nm_output = String::from_utf8(nm_command.stdout).unwrap();
    let re = Regex::new(r"(\w+) [B] executor_globals").unwrap();
    let cap = re.captures(&nm_output).unwrap_or_else(|| {
        println!("Cannot find executor_globals in php process");
        process::exit(1)
    });
    let address_str = cap.get(1).unwrap().as_str();
    usize::from_str_radix(address_str, 16).unwrap()
}

fn get_maps_address<Pid>(pid: Pid) -> usize
where
    Pid: TryIntoProcessHandle + std::fmt::Display,
{
    let map_path = format!("/proc/{}/maps", pid);
    let exe_path = fs::read_link(format!("/proc/{}/exe", pid)).unwrap().to_string_lossy().to_string();

    let file = File::open(map_path).unwrap();
    for line in io::BufReader::new(file).lines() {
        let line = line.unwrap();
        if line.contains(&exe_path) {
	    let address_str = line.split("-").collect::<Vec<&str>>()[0];
            return usize::from_str_radix(address_str, 16).unwrap();
        }
    }
    0
}

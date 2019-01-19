


use std;
use std::fs;
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
    let cat_command = Command::new("cat")
        .arg(format!("/proc/{}/maps", pid))
        .stdout(Stdio::piped())
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {}", e));
    if !cat_command.status.success() {
        panic!(
            "failed to execute process: {}",
            String::from_utf8(cat_command.stderr).unwrap()
        )
    }

    let exe_path = fs::read_link(format!("/proc/{}/exe", pid)).unwrap();
    let exe_path = exe_path.to_string_lossy();
    let regex = String::from(r"(\w+).+p.+?") + &exe_path;

    let output = String::from_utf8(cat_command.stdout).unwrap();
    let re = Regex::new(&regex).unwrap();
    let cap = re.captures(&output).unwrap();
    let address_str = cap.get(1).unwrap().as_str();
    usize::from_str_radix(address_str, 16).unwrap()
}

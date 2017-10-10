#![feature(try_from)]

pub mod dwarf;
extern crate byteorder;
extern crate clap;
extern crate libc;
extern crate read_process_memory;
extern crate regex;

use std::convert::TryFrom;
use dwarf::{parse_dwarf_file, CStruct, CUnion, DwarfLookup};
use libc::*;
use std::process;
use regex::Regex;
use read_process_memory::*;
use std::process::{Command, Stdio};
use std::io::Cursor;
use std::thread;
use std::time::Duration;
use std::collections::{HashMap, HashSet};
use byteorder::{NativeEndian, ReadBytesExt};
use clap::{App, Arg, ArgMatches};

fn parse_args() -> ArgMatches<'static> {
    App::new("php-stacktrace")
        .version("0.1")
        .about("Sampling profiler for PHP programs")
        .arg(
            Arg::with_name("COMMAND")
                .help("trace or top or oneshot")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("DEBUGINFO")
                .help("Path to php debuginfo")
                .required(true)
                .index(2),
        )
        .arg(
            Arg::with_name("PID")
                .help("PID of the PHP process you want to profile")
                .required(true)
                .index(3),
        )
        .get_matches()
}

fn main() {
    let matches = parse_args();
    let pid: pid_t = matches.value_of("PID").unwrap().parse().unwrap();
    let path: String = matches.value_of("DEBUGINFO").unwrap().parse().unwrap();
    let command = matches.value_of("COMMAND").unwrap();

    let dwarf = parse_dwarf_file(path);
    let source = pid.try_into_process_handle().unwrap();
    let debug_info = get_debug_info(pid, dwarf);

    let mut method_stats = HashMap::new();
    let mut method_own_time_stats = HashMap::new();
    let mut j = 0;

    match command {
        "top" => {
            loop {
                j += 1;
                let trace = get_stack_trace(&source, &debug_info);
                let mut seen = HashSet::new();
                for item in &trace {
                    if !seen.contains(&item.clone()) {
                        let counter = method_stats.entry(item.clone()).or_insert(0);
                        *counter += 1;
                    }
                    seen.insert(item.clone());
                }
                {
                    if trace.len() > 0 {
                        let counter2 = method_own_time_stats.entry(trace[0].clone()).or_insert(0);
                        *counter2 += 1;
                    }
                }
                if j % 100 == 0 {
                    print_method_stats(&method_stats, &method_own_time_stats, 30);
                    method_stats = HashMap::new();
                    method_own_time_stats = HashMap::new();
                }
                thread::sleep(Duration::from_millis(10));
            }
        },
        "trace" => {
            loop {
                let trace = get_stack_trace(&source, &debug_info);
                if trace.len() > 0 {
                    for item in &trace {
                        println!("{}", item);
                    }
                    println!("{}", 1);
                }
                thread::sleep(Duration::from_millis(10));
            }
        },
        "oneshot" => {
            loop {
                let trace = get_stack_trace(&source, &debug_info);
                if trace.len() > 0 {
                    for item in &trace {
                        println!("{}", item);
                    }
                    break;
                } else {
                    thread::sleep(Duration::from_millis(10));
                }
            }
        }
        _ => {
            println!("COMMAND must be trace/top/oneshot");
        }
    }
}

fn get_debug_info(pid: pid_t, dwarf: DwarfLookup) -> DebugInfo {
    let zend_executor_globals = dwarf
        .find_struct(String::from("_zend_executor_globals"))
        .unwrap();
    let current_execute_data_offset = zend_executor_globals
        .find_member(String::from("current_execute_data"))
        .unwrap()
        .byte_offset;
    let zend_execute_data = dwarf
        .find_struct(String::from("_zend_execute_data"))
        .unwrap();
    let func_offset = zend_execute_data
        .find_member(String::from("func"))
        .unwrap()
        .byte_offset;
    let this_offset = zend_execute_data
        .find_member(String::from("This"))
        .unwrap()
        .byte_offset;
    let prev_offset = zend_execute_data
        .find_member(String::from("prev_execute_data"))
        .unwrap()
        .byte_offset;

    let zend_function = dwarf.find_union(String::from("_zend_function")).unwrap();
    let member_common = zend_function.find_member(String::from("common")).unwrap();
    let common = dwarf.find_struct_by_id(member_common.type_id).unwrap();
    let function_name_offset = common
        .find_member(String::from("function_name"))
        .unwrap()
        .byte_offset;

    let zend_string = dwarf.find_struct(String::from("_zend_string")).unwrap();
    let zend_string_len_offset = zend_string
        .find_member(String::from("len"))
        .unwrap()
        .byte_offset;
    let zend_string_val_offset = zend_string
        .find_member(String::from("val"))
        .unwrap()
        .byte_offset;

    DebugInfo {
        executor_globals_address: get_executor_globals_address(pid),
        zend_executor_globals: zend_executor_globals.clone(),
        zend_execute_data: zend_execute_data.clone(),
        zval: dwarf
            .find_struct(String::from("_zval_struct"))
            .unwrap()
            .clone(),
        zend_value: dwarf
            .find_union(String::from("_zend_value"))
            .unwrap()
            .clone(),
        zend_function: zend_function.clone(),
        zend_string: zend_string.clone(),
        zend_class_entry: dwarf
            .find_struct(String::from("_zend_class_entry"))
            .unwrap()
            .clone(),
        current_execute_data_offset: current_execute_data_offset,
        func_offset: func_offset,
        this_value_offset: this_offset, // zend_value is the field of zval
        prev_execute_data_offset: prev_offset,
        function_name_offset: function_name_offset,
        zend_string_len_offset: zend_string_len_offset,
        zend_string_val_offset: zend_string_val_offset,
    }
}

#[derive(Debug, Clone)]
struct DebugInfo {
    executor_globals_address: usize,
    zend_executor_globals: CStruct,
    zend_execute_data: CStruct,
    zval: CStruct,
    zend_value: CUnion,
    zend_function: CUnion,
    zend_string: CStruct,
    zend_class_entry: CStruct,
    current_execute_data_offset: usize,
    this_value_offset: usize,
    func_offset: usize,
    prev_execute_data_offset: usize,
    function_name_offset: usize,
    zend_string_len_offset: usize,
    zend_string_val_offset: usize,
}

fn get_pointer_address(vec: &[u8]) -> usize {
    let mut rdr = Cursor::new(vec);
    usize::try_from(rdr.read_u64::<NativeEndian>().unwrap()).unwrap()
}

fn get_usize(vec: &[u8]) -> usize {
    let mut rdr = Cursor::new(vec);
    usize::try_from(rdr.read_u64::<NativeEndian>().unwrap()).unwrap()
}

fn copy_address_raw<T>(addr: *const c_void, length: usize, source: &T) -> Option<Vec<u8>>
where
    T: CopyAddress,
{
    if (length > 10240) {
        return None;
    }
    let mut copy = vec![0; length];
    match source.copy_address(addr as usize, &mut copy) {
        Ok(_) => Some(copy),
        Err(_) => None
    }
}

fn get_executor_globals_address(pid: pid_t) -> usize {
    get_maps_address(pid) + get_nm_address(pid)
}

fn get_nm_address(pid: pid_t) -> usize {
    let nm_command = Command::new("nm")
        .arg("-D")
        .arg((format!("/proc/{}/exe", pid)))
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
    let addr = usize::from_str_radix(address_str, 16).unwrap();
    addr
}

fn get_maps_address(pid: pid_t) -> usize {
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

    let output = String::from_utf8(cat_command.stdout).unwrap();
    let re = Regex::new(r"(\w+).+xp.+?bin/php").unwrap();
    let cap = re.captures(&output).unwrap();
    let address_str = cap.get(1).unwrap().as_str();
    let addr = usize::from_str_radix(address_str, 16).unwrap();
    addr
}

fn get_current_execute_data_address<T>(source: &T, info: &DebugInfo) -> Option<usize>
where
    T: CopyAddress,
{
    let pointer_addr = info.executor_globals_address + info.current_execute_data_offset;
    let data = copy_address_raw(pointer_addr as *const c_void, 8, source);
    match data {
        Some(d) => Some(get_pointer_address(&d)),
        None => None
    }
}

fn read_execute_data<T>(addr: usize, source: &T, info: &DebugInfo) -> Option<Vec<u8>>
where
    T: CopyAddress,
{
    let size = info.zend_execute_data.byte_size;
    copy_address_raw(addr as *const c_void, size, source)
}

fn get_func_address(execute_data: &Vec<u8>, info: &DebugInfo) -> usize {
    let mut rdr = Cursor::new(execute_data);
    rdr.set_position(u64::try_from(info.func_offset).unwrap());
    usize::try_from(rdr.read_u64::<NativeEndian>().unwrap()).unwrap()
}

fn read_function_name_address<T>(func_address: usize, source: &T, info: &DebugInfo) -> Option<usize>
where
    T: CopyAddress,
{
    let addr = func_address + info.function_name_offset;
    let mdata = copy_address_raw(addr as *const c_void, 8, source);
    match mdata {
        Some(d) => Some(get_pointer_address(&d)),
        None => None
    }
}

fn get_prev_execute_data_address(execute_data: &Vec<u8>, info: &DebugInfo) -> usize {
    let mut rdr = Cursor::new(execute_data);
    rdr.set_position(u64::try_from(info.prev_execute_data_offset).unwrap());
    usize::try_from(rdr.read_u64::<NativeEndian>().unwrap()).unwrap()
}

fn read_zend_string<T>(addr: usize, source: &T, info: &DebugInfo) -> Option<String>
where
    T: CopyAddress,
{
    let len_addr = addr + info.zend_string_len_offset;
    let val_addr = addr + info.zend_string_val_offset;
    let len_data = copy_address_raw(len_addr as *const c_void, 8, source);
    if len_data.is_none() {
        return None;
    }
    let len = get_usize(&len_data.unwrap());
    let data = copy_address_raw(val_addr as *const c_void, len, source);
    if data.is_none() {
        return None;
    }
    String::from_utf8(data.unwrap()).ok()
}

fn get_stack_trace<T>(source: &T, info: &DebugInfo) -> Vec<String>
where
    T: CopyAddress,
{
    let maddr = get_current_execute_data_address(source, info);
    let mut stack_trace = vec![];

    if maddr.is_none() {
        return stack_trace;
    }

    let mut addr = maddr.unwrap();

    while addr != 0 {
        let mexecute_data = read_execute_data(addr, source, info);
        if mexecute_data.is_none() {
            return stack_trace;
        }
        let execute_data = mexecute_data.unwrap();

        let func_addr = get_func_address(&execute_data, info);
        let mut trace = String::new();
        if func_addr == 0 {
            trace.push_str("???");
        } else {
            let mfunction_name_addr = read_function_name_address(func_addr, source, info);
            if mfunction_name_addr.is_none() {
                return stack_trace;
            }
            let function_name_addr = mfunction_name_addr.unwrap();

            if function_name_addr != 0 {
                let function_name = read_zend_string(function_name_addr, source, info);
                if function_name.is_some() {
                    trace.push_str(function_name.unwrap().as_str());
                }
            } else {
                trace.push_str("main");
            }
        }
        stack_trace.push(trace);
        let prev_execute_data_addr = get_prev_execute_data_address(&execute_data, info);
        addr = prev_execute_data_addr;
    }
    return stack_trace;
}

// modify from ruby-stacktrace[https://github.com/jvns/ruby-stacktrace/blob/master/src/lib.rs#L173]
pub fn print_method_stats(method_stats: &HashMap<String, u32>,
method_own_time_stats: &HashMap<String, u32>,
                      n_terminal_lines: usize) {

    let mut count_vec: Vec<_> = method_own_time_stats.iter().collect();
    count_vec.sort_by(|a, b| b.1.cmp(a.1));
    let self_sum: u32 = method_own_time_stats.values().fold(0, std::ops::Add::add);
    let stotal_sum: Option<&u32> = method_stats.values().max();
    if stotal_sum.is_none() {
        return;
    }
    let total_sum: u32 = *method_stats.values().max().unwrap();
    if count_vec.len() == 0 {
        return;
    }
    println!("[{}c", 27 as char); // clear the screen
    println!(" {:4} | {:4} | {}", "self", "tot", "method");
    for &(method, count) in count_vec.iter().take(n_terminal_lines - 1) {
        let total_count = method_stats.get(&method[..]).unwrap();
        println!(" {:02.1}% | {:02.1}% | {}",
                 100.0 * (*count as f32) / (self_sum as f32),
                 100.0 * (*total_count as f32) / (total_sum as f32),
                 method);
    }
}

#![feature(try_from)]
#[macro_use]
extern crate serde_derive;

mod debuginfo;
mod dwarf;

extern crate byteorder;
extern crate clap;
extern crate libc;
extern crate read_process_memory;
extern crate regex;

use std::convert::TryFrom;
use libc::*;
use read_process_memory::*;
use std::io::Cursor;
use std::thread;
use std::time::Duration;
use byteorder::{NativeEndian, ReadBytesExt};
use clap::{App, Arg, ArgMatches};
use debuginfo::*;
use dwarf::*;


fn main()
where
    Pid: TryIntoProcessHandle + std::fmt::Display + std::str::FromStr + Copy,
{
    let matches = parse_args();

    let pid: Pid = matches.value_of("PID").unwrap().parse().unwrap();

    let debuginfo = get_debug_info(pid, &matches);

    if matches.is_present("WRITE_CONFIG") {
        write_debuginfo_to_conif(&debuginfo, matches.value_of("WRITE_CONFIG").unwrap().parse().unwrap())
    }

    let source = pid.try_into_process_handle().unwrap();

    println!("{:?}", debuginfo);

    loop {
        let trace = get_stack_trace(&source, &debuginfo);
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

fn parse_args() -> ArgMatches<'static> {
    App::new("php-stacktrace")
        .version("0.1")
        .about("Read stacktrace from outside PHP process")
        .arg(
            Arg::with_name("DEBUGINFO")
                .value_name("debuginfo_path")
                .short("d")
                .help("Path to php debuginfo")
                .required(false),
        )
        .arg(
            Arg::with_name("WRITE_CONFIG")
                .short("w")
                .value_name("write_config_path")
                .help("Write config to file, requires -d")
                .requires("DEBUGINFO")
                .required(false),
        )
        .arg(
            Arg::with_name("CONFIG")
                .short("c")
                .value_name("config_path")
                .help("Path to config file, conflicts with -d, -w")
                .conflicts_with_all(&["DEBUGINFO", "WRITE_CONFIG"])
                .required(false),
        )
        .arg(
            Arg::with_name("PID")
                .help("PID of the PHP process")
                .required(true)
                .index(1),
        )
        .get_matches()
}

fn get_debug_info<Pid>(pid: Pid, matches: &ArgMatches)-> DebugInfo
where Pid: TryIntoProcessHandle + std::fmt::Display + std::str::FromStr + Copy,
{
    if matches.is_present("CONFIG") {
        let config_path: String = matches.value_of("CONFIG").unwrap().parse().unwrap();
        get_debug_info_from_config(pid, config_path).unwrap()
    } else {
        let dwarf_path: String = matches.value_of("DEBUGINFO").unwrap().parse().unwrap();
        let dwarf = parse_dwarf_file(dwarf_path);
        let debuginfo = get_debug_info_from_dwarf(pid, dwarf);
        debuginfo
    }
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
    if length > 10240 {
        return None;
    }
    let mut copy = vec![0; length];
    match source.copy_address(addr as usize, &mut copy) {
        Ok(_) => Some(copy),
        Err(_) => None,
    }
}

fn get_current_execute_data_address<T>(source: &T, info: &DebugInfo) -> Option<usize>
where
    T: CopyAddress,
{
    let pointer_addr = info.executor_globals_address + info.eg_current_execute_data_offset;
    let data = copy_address_raw(pointer_addr as *const c_void, 8, source);
    match data {
        Some(d) => Some(get_pointer_address(&d)),
        None => None,
    }
}

fn read_execute_data<T>(addr: usize, source: &T, info: &DebugInfo) -> Option<Vec<u8>>
where
    T: CopyAddress,
{
    let size = info.zend_execute_data_byte_size;
    copy_address_raw(addr as *const c_void, size, source)
}

fn get_func_address(execute_data: &Vec<u8>, info: &DebugInfo) -> usize {
    let mut rdr = Cursor::new(execute_data);
    rdr.set_position(u64::try_from(info.ed_func_offset).unwrap());
    usize::try_from(rdr.read_u64::<NativeEndian>().unwrap()).unwrap()
}

fn read_function_name_address<T>(func_address: usize, source: &T, info: &DebugInfo) -> Option<usize>
where
    T: CopyAddress,
{
    let addr = func_address + info.fu_function_name_offset;
    let mdata = copy_address_raw(addr as *const c_void, 8, source);
    match mdata {
        Some(d) => Some(get_pointer_address(&d)),
        None => None,
    }
}

fn get_prev_execute_data_address(execute_data: &Vec<u8>, info: &DebugInfo) -> usize {
    let mut rdr = Cursor::new(execute_data);
    rdr.set_position(u64::try_from(info.ed_prev_execute_data_offset).unwrap());
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

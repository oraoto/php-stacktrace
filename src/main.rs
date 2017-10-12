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
use std::io;
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
        write_debuginfo_to_conif(
            &debuginfo,
            matches.value_of("WRITE_CONFIG").unwrap().parse().unwrap(),
        )
    }

    let source = pid.try_into_process_handle().unwrap();

    loop {
        match get_stack_trace(&source, &debuginfo) {
            Err(_) => (),
            Ok(trace) => if trace.len() > 0 {
                for item in &trace {
                    println!("{}", item);
                }
                break;
            }
        }
        thread::sleep(Duration::from_millis(10));
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

fn get_debug_info<Pid>(pid: Pid, matches: &ArgMatches) -> DebugInfo
where
    Pid: TryIntoProcessHandle + std::fmt::Display + std::str::FromStr + Copy,
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

fn copy_address_raw<T>(addr: *const c_void, length: usize, source: &T) -> io::Result<Vec<u8>>
where
    T: CopyAddress,
{
    if length > 512 * 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Can't copy more than 512KB",
        ));
    }
    let mut copy = vec![0; length];
    try!(source.copy_address(addr as usize, &mut copy));
    Ok(copy)
}

fn get_current_execute_data_address<T>(source: &T, info: &DebugInfo) -> io::Result<usize>
where
    T: CopyAddress,
{
    let pointer_addr = info.executor_globals_address + info.eg_current_execute_data_offset;
    let data = try!(copy_address_raw(pointer_addr as *const c_void, 8, source));
    Ok(get_pointer_address(&data))
}

fn read_execute_data<T>(addr: usize, source: &T, info: &DebugInfo) -> io::Result<Vec<u8>>
where
    T: CopyAddress,
{
    let size = info.ed_byte_size;
    copy_address_raw(addr as *const c_void, size, source)
}

fn get_func_address(execute_data: &Vec<u8>, info: &DebugInfo) -> usize {
    let mut rdr = Cursor::new(execute_data);
    rdr.set_position(u64::try_from(info.ed_func_offset).unwrap());
    usize::try_from(rdr.read_u64::<NativeEndian>().unwrap()).unwrap()
}

fn read_function_name_address<T>(
    func_address: usize,
    source: &T,
    info: &DebugInfo,
) -> io::Result<usize>
where
    T: CopyAddress,
{
    let addr = func_address + info.fu_function_name_offset;
    let data = try!(copy_address_raw(addr as *const c_void, 8, source));
    Ok(get_pointer_address(&data))
}

fn read_vm_stack<T>(source: &T, info: &DebugInfo) -> std::io::Result<(usize, usize, Vec<u8>)>
where
    T: CopyAddress,
{
    panic!("")
}


fn get_prev_execute_data_address(execute_data: &Vec<u8>, info: &DebugInfo) -> usize {
    let mut rdr = Cursor::new(execute_data);
    rdr.set_position(u64::try_from(info.ed_prev_execute_data_offset).unwrap());
    usize::try_from(rdr.read_u64::<NativeEndian>().unwrap()).unwrap()
}

fn read_zend_string<T>(addr: usize, source: &T, info: &DebugInfo) -> io::Result<String>
where
    T: CopyAddress,
{
    let len_addr = addr + info.zend_string_len_offset;
    let val_addr = addr + info.zend_string_val_offset;
    let len_data = try!(copy_address_raw(len_addr as *const c_void, 8, source));
    let len = get_usize(&len_data);
    let data = try!(copy_address_raw(val_addr as *const c_void, len, source));
    Ok(String::from_utf8(data).unwrap())
}

fn get_stack_trace<T>(source: &T, info: &DebugInfo) -> io::Result<Vec<String>>
where
    T: CopyAddress,
{
    let mut addr = try!(get_current_execute_data_address(source, info));
    let mut stack_trace = vec![];

    while addr != 0 {
        let execute_data = try!(read_execute_data(addr, source, info));

        let func_addr = get_func_address(&execute_data, info);
        let mut trace = String::new();
        if func_addr == 0 {
            trace.push_str("???");
        } else {
            let function_name_addr = try!(read_function_name_address(func_addr, source, info));
            if function_name_addr != 0 {
                let function_name = try!(read_zend_string(function_name_addr, source, info));
                trace.push_str(function_name.as_str());
            } else {
                trace.push_str("main");
            }
        }
        stack_trace.push(trace);
        let prev_execute_data_addr = get_prev_execute_data_address(&execute_data, info);
        addr = prev_execute_data_addr;
    }
    Ok(stack_trace)
}

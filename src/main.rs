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
        let vm_stack = get_vm_stack(&source, &debuginfo);
        if vm_stack.is_err() {
            thread::sleep(Duration::from_millis(10));
            continue;
        }
        match get_stack_trace(&source, &debuginfo, &vm_stack.unwrap()) {
            Err(_) => (),
            Ok(trace) => if trace.len() > 0 {
                for item in trace {
                    if item.scope.is_some() {
                        println!("{}->{}()", item.scope.unwrap(), item.name);
                    } else {
                        println!("{}()", item.name);
                    }
                }
                break;
            },
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

fn read_pointer_address(vec: &[u8]) -> usize {
    let mut rdr = Cursor::new(vec);
    usize::try_from(rdr.read_u64::<NativeEndian>().unwrap()).unwrap()
}

fn get_usize(vec: &[u8]) -> usize {
    let mut rdr = Cursor::new(vec);
    usize::try_from(rdr.read_u64::<NativeEndian>().unwrap()).unwrap()
}

fn copy_address_checked<T>(addr: usize, length: usize, source: &T) -> io::Result<Vec<u8>>
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
    try!(source.copy_address(addr, &mut copy));
    Ok(copy)
}

fn get_function_name_address<T>(
    func_address: usize,
    source: &T,
    info: &DebugInfo,
) -> io::Result<usize>
where
    T: CopyAddress,
{
    let addr = func_address + info.fu_function_name_offset;
    let data = try!(copy_address_checked(addr, 8, source));
    Ok(read_pointer_address(&data))
}

fn get_function_scope_address<T>(
    func_address: usize,
    source: &T,
    info: &DebugInfo,
) -> io::Result<usize>
where
    T: CopyAddress,
{
    let addr = func_address + info.fu_scope_offset;
    let data = try!(copy_address_checked(addr, 8, source));
    Ok(read_pointer_address(&data))
}


#[derive(Debug, Clone)]
struct VMStack {
    low: usize,
    high: usize,
    data: Vec<u8>,
    current_ed_addr: usize,
}

#[derive(Debug, Clone)]
struct FunctionInfo {
    name: String,
    scope: Option<String>,
}

fn get_vm_stack<T>(source: &T, info: &DebugInfo) -> std::io::Result<VMStack>
where
    T: CopyAddress,
{
    let eg = try!(copy_address_checked(
        info.executor_globals_address,
        info.eg_byte_size,
        source
    ));
    let mut eg_rdr = Cursor::new(&eg);
    eg_rdr.set_position(usize_to_u64(info.eg_vm_stack_top_offset));
    let eg_vm_stack_top = u64_to_usize(eg_rdr.read_u64::<NativeEndian>().unwrap());
    // eg_rdr.set_position(usize_to_u64(info.eg_vm_stack_end_offset));
    // let eg_vm_stack_end = u64_to_usize(eg_rdr.read_u64::<NativeEndian>().unwrap());
    eg_rdr.set_position(usize_to_u64(info.eg_current_execute_data_offset));
    let current_ed_addr = u64_to_usize(eg_rdr.read_u64::<NativeEndian>().unwrap());


    eg_rdr.set_position(usize_to_u64(info.eg_vm_stack_offset));
    let vm_stack_addr = u64_to_usize(eg_rdr.read_u64::<NativeEndian>().unwrap());
    let vm_stack = try!(copy_address_checked(
        vm_stack_addr,
        info.stack_byte_size,
        source
    ));

    let mut stack_rdr = Cursor::new(&vm_stack);
    let vm_stack_top = u64_to_usize(stack_rdr.read_u64::<NativeEndian>().unwrap());
    stack_rdr.set_position(usize_to_u64(info.stack_end_offset));

    // let vm_stack_end  = u64_to_usize(stack_rdr.read_u64::<NativeEndian>().unwrap());
    // Todo: check it's a single stack

    let size = eg_vm_stack_top - vm_stack_top;
    let data = try!(copy_address_checked(vm_stack_top, size, source));

    Ok(VMStack {
        low: vm_stack_top,
        high: eg_vm_stack_top,
        data: data,
        current_ed_addr: current_ed_addr,
    })
}

fn get_stack_trace<T>(
    source: &T,
    info: &DebugInfo,
    stack: &VMStack,
) -> io::Result<Vec<FunctionInfo>>
where
    T: CopyAddress,
{
    if stack.current_ed_addr < stack.low {
        return Err(io::Error::new(io::ErrorKind::Other, "Not execution"));
    }

    let mut ed_offset = stack.current_ed_addr - stack.low;
    let mut trace = vec![];

    loop {
        let ed_func_offset = ed_offset + info.ed_func_offset;
        let ed_func_prev_offset = ed_offset + info.ed_prev_execute_data_offset;

        let mut rdr = Cursor::new(&stack.data);
        let mut function = FunctionInfo {
            name: String::from("???"),
            scope: None,
        };

        rdr.set_position(usize_to_u64(ed_func_offset));
        let func_addr = u64_to_usize(rdr.read_u64::<NativeEndian>().unwrap());
        function.name = try!(get_function_name(func_addr, source, info));

        match get_function_scope(func_addr, ed_offset, source, info, stack) {
            Ok(s) => function.scope = Some(s),
            Err(_) => (),
        };

        trace.push(function);

        rdr.set_position(usize_to_u64(ed_func_prev_offset));
        let prev_addr = u64_to_usize(rdr.read_u64::<NativeEndian>().unwrap());
        if prev_addr == 0 {
            break;
        }
        ed_offset = prev_addr - stack.low;
    }
    Ok(trace)
}

fn get_function_name<T>(func_addr: usize, source: &T, info: &DebugInfo) -> io::Result<String>
where
    T: CopyAddress,
{
    if func_addr == 0 {
        return Ok(String::from("???"));
    }

    let function_name_addr = try!(get_function_name_address(func_addr, source, info));
    if function_name_addr == 0 {
        Ok(String::from("main"))
    } else {
        let function_name = try!(read_zend_string(function_name_addr, source, info));
        Ok(function_name)
    }
}

fn get_function_scope<T>(
    func_addr: usize,
    ed_offset: usize,
    source: &T,
    info: &DebugInfo,
    stack: &VMStack,
) -> io::Result<String>
where
    T: CopyAddress,
{
    let this_offset = ed_offset + info.ed_this_offset;
    let mut rdr = Cursor::new(&stack.data);
    rdr.set_position(usize_to_u64(this_offset));
    let this_data = rdr.read_u64::<NativeEndian>().unwrap();

    let is_obj = this_data != 0;
    if is_obj {
        let scope_addr = try!(get_function_scope_address(func_addr, source, info));
        if scope_addr != 0 {
            return get_class_entry_name(scope_addr, source, info);
        } else {
            return Err(io::Error::new(io::ErrorKind::Other, "Not implemented"));
        }
    } else {
        let scope_addr = try!(get_function_scope_address(func_addr, source, info));
        return get_class_entry_name(scope_addr, source, info);
    }
}

fn get_class_entry_name<T>(ce_addr: usize, source: &T, info: &DebugInfo) -> io::Result<String>
where
    T: CopyAddress,
{
    if ce_addr != 0 {
        let addr = read_pointer_address(&try!(copy_address_checked(
            ce_addr + info.ce_name_offset,
            8,
            source
        )));
        return read_zend_string(addr, source, info);
    } else {
        return Err(io::Error::new(io::ErrorKind::Other, "Failed to read class entry name"));
    }
}

fn read_zend_string<T>(addr: usize, source: &T, info: &DebugInfo) -> io::Result<String>
where
    T: CopyAddress,
{
    let len_addr = addr + info.zend_string_len_offset;
    let val_addr = addr + info.zend_string_val_offset;
    let len_data = try!(copy_address_checked(len_addr, 8, source));
    let len = get_usize(&len_data);
    let data = try!(copy_address_checked(val_addr, len, source));
    Ok(String::from_utf8(data).unwrap())
}

fn u64_to_usize(u: u64) -> usize {
    usize::try_from(u).unwrap()
}
fn usize_to_u64(u: usize) -> u64 {
    u64::try_from(u).unwrap()
}

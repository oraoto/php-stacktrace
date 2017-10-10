#![feature(try_from)]
#[macro_use] extern crate log;

pub mod dwarf;
extern crate libc;
extern crate regex;
extern crate read_process_memory;
extern crate byteorder;

use std::convert::TryFrom;
use dwarf::{parse_dwarf_file, CStruct, CUnion, DwarfLookup};
use libc::*;
use std::process;
use regex::Regex;
use read_process_memory::*;
use std::process::{Command, Stdio};
use std::io::Cursor;
use byteorder::{NativeEndian,ReadBytesExt};

fn main() {
    let dwarf = parse_dwarf_file(String::from("./ref/php.dwz"));
    let pid: pid_t = "8759".parse().unwrap();
    let source = pid.try_into_process_handle().unwrap();
    let debug_info = get_debug_info(pid, dwarf);
    let stack_trace = get_stack_trace(&source, &debug_info);
    println!("{:?}", stack_trace);
}

fn get_debug_info(pid: pid_t, dwarf: DwarfLookup) -> DebugInfo
{
    let zend_executor_globals = dwarf.find_struct(String::from("_zend_executor_globals")).unwrap();
    let current_execute_data_offset = zend_executor_globals.find_member(String::from("current_execute_data")).unwrap().byte_offset;
    let zend_execute_data =  dwarf.find_struct(String::from("_zend_execute_data")).unwrap();
    let func_offset = zend_execute_data.find_member(String::from("func")).unwrap().byte_offset;
    let this_offset = zend_execute_data.find_member(String::from("This")).unwrap().byte_offset;
    let prev_offset = zend_execute_data.find_member(String::from("prev_execute_data")).unwrap().byte_offset;

    let zend_function = dwarf.find_union(String::from("_zend_function")).unwrap();
    let member_common = zend_function.find_member(String::from("common")).unwrap();
    let common = dwarf.find_struct_by_id(member_common.type_id).unwrap();
    let function_name_offset = common.find_member(String::from("function_name")).unwrap().byte_offset;

    let zend_string = dwarf.find_struct(String::from("_zend_string")).unwrap();
    let zend_string_len_offset = zend_string.find_member(String::from("len")).unwrap().byte_offset;
    let zend_string_val_offset = zend_string.find_member(String::from("val")).unwrap().byte_offset;

    DebugInfo {
        executor_globals_address: get_executor_globals_address(pid),
        zend_executor_globals: zend_executor_globals.clone(),
        zend_execute_data: zend_execute_data.clone(),
        zval: dwarf.find_struct(String::from("_zval_struct")).unwrap().clone(),
        zend_value: dwarf.find_union(String::from("_zend_value")).unwrap().clone(),
        zend_function: zend_function.clone(),
        zend_string: zend_string.clone(),
        zend_class_entry: dwarf.find_struct(String::from("_zend_class_entry")).unwrap().clone(),
        current_execute_data_offset: current_execute_data_offset,
        func_offset: func_offset,
        this_value_offset: this_offset, // zend_value is the field of zval
        prev_execute_data_offset: prev_offset,
        function_name_offset: function_name_offset,
        zend_string_len_offset: zend_string_len_offset,
        zend_string_val_offset: zend_string_val_offset
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

fn copy_address_raw<T>(addr: *const c_void, length: usize, source: &T) -> Vec<u8>
    where T: CopyAddress
{
    debug!("copy_address_raw: addr: {:x}", addr as usize);
    let mut copy = vec![0; length];
    match source.copy_address(addr as usize, &mut copy) {
        Ok(_) => {}
        Err(e) => warn!("copy_address failed for {:p}: {:?}", addr, e),
    }
    copy
}

fn get_executor_globals_address(pid: pid_t)->usize {
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
        panic!("failed to execute process: {}", String::from_utf8(nm_command.stderr).unwrap())
    }

    let nm_output = String::from_utf8(nm_command.stdout).unwrap();
    let re = Regex::new(r"(\w+) [B] executor_globals").unwrap();
    let cap = re.captures(&nm_output).unwrap_or_else(|| {
        println!("Cannot find executor_globals in php process");
        process::exit(1)
    });
    let address_str = cap.get(1).unwrap().as_str();
    let addr = usize::from_str_radix(address_str, 16).unwrap();
    debug!("get_nm_address: {:x}", addr);
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
        panic!("failed to execute process: {}", String::from_utf8(cat_command.stderr).unwrap())
    }

    let output = String::from_utf8(cat_command.stdout).unwrap();
    let re = Regex::new(r"(\w+).+xp.+?bin/php").unwrap();
    let cap = re.captures(&output).unwrap();
    let address_str = cap.get(1).unwrap().as_str();
    let addr = usize::from_str_radix(address_str, 16).unwrap();
    debug!("get_maps_address: {:x}", addr);
    addr
}

fn get_current_execute_data_address<T>(source: &T, info: &DebugInfo) -> usize
    where T: CopyAddress
{
    let pointer_addr = info.executor_globals_address + info.current_execute_data_offset;
    get_pointer_address(&copy_address_raw(pointer_addr as *const c_void, 8, source))
}

fn read_execute_data<T>(addr: usize, source: &T, info:&DebugInfo) -> Vec<u8>
    where T: CopyAddress
{
    let size = info.zend_execute_data.byte_size;
    copy_address_raw(addr as *const c_void, size, source)
}

fn get_func_address(execute_data: &Vec<u8>, info: &DebugInfo) -> usize
{
    let mut rdr = Cursor::new(execute_data);
    rdr.set_position(u64::try_from(info.func_offset).unwrap());
    usize::try_from(rdr.read_u64::<NativeEndian>().unwrap()).unwrap()
}

fn read_function_name_address<T>(func_address: usize, source: &T, info: &DebugInfo) -> usize
    where T: CopyAddress
{
    let addr = func_address + info.function_name_offset;
    get_pointer_address(&copy_address_raw(addr as *const c_void, 8, source))
}

fn get_prev_execute_data_address(execute_data: &Vec<u8>, info: &DebugInfo) -> usize
{
    let mut rdr = Cursor::new(execute_data);
    rdr.set_position(u64::try_from(info.prev_execute_data_offset).unwrap());
    usize::try_from(rdr.read_u64::<NativeEndian>().unwrap()).unwrap()
}

fn read_zend_string<T>(addr: usize, source: &T, info: &DebugInfo) -> String
    where T: CopyAddress
{
    let len_addr = addr + info.zend_string_len_offset;
    let val_addr = addr + info.zend_string_val_offset;
    let len = get_usize(&copy_address_raw(len_addr as *const c_void, 8, source));
    let data = copy_address_raw(val_addr as *const c_void, len, source);
    String::from_utf8(data).unwrap()
}

fn get_stack_trace<T>(source: &T, info: &DebugInfo) -> Vec<String>
    where T: CopyAddress
{
    let mut addr = get_current_execute_data_address(source, info);
    let mut stack_trace = vec!();

    while addr != 0 {
        let execute_data = read_execute_data(addr, source, info);
        let func_addr = get_func_address(&execute_data, info);
        if func_addr == 0 {
           stack_trace.push(String::from("???"));
        } else {
            let function_name_addr = read_function_name_address(func_addr, source, info);
            if function_name_addr != 0 {
                let function_name = read_zend_string(function_name_addr, source, info);
                stack_trace.push(function_name);
            } else {
                stack_trace.push(String::from("main"));
            }
        }
        let prev_execute_data_addr = get_prev_execute_data_address(&execute_data, info);
        addr = prev_execute_data_addr;
    }
    return stack_trace;
}
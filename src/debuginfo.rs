extern crate serde;
extern crate serde_json;

use std;
use std::error::Error;
use std::io::prelude::*;
use std::fs::File;
use std::path::Path;
use std::process;
use std::process::{Command, Stdio};
use regex::Regex;
use read_process_memory::*;
use dwarf::DwarfLookup;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DebugInfo {
    #[serde(default)]
    #[serde(skip_serializing)]
    pub executor_globals_address: usize,

    // _zend_executor_globals
    pub eg_byte_size: usize,
    pub eg_current_execute_data_offset: usize,
    pub eg_vm_stack_top_offset: usize,
    pub eg_vm_stack_end_offset: usize,
    pub eg_vm_stack_offset: usize,

    // _zend_execute_data
    pub ed_byte_size: usize,
    pub ed_this_offset: usize,
    pub ed_func_offset: usize,
    pub ed_prev_execute_data_offset: usize,

    // _zend_function
    pub fu_function_name_offset: usize,
    pub fu_scope_offset: usize,

    // _zend_string
    pub zend_string_len_offset: usize,
    pub zend_string_val_offset: usize,

    // _zend_vm_stack
    pub stack_byte_size: usize,
    pub stack_end_offset: usize,

    // _zend_class_entry
    pub ce_name_offset: usize,
}

pub fn get_debug_info_from_config<Pid>(pid: Pid, path: &str) -> std::io::Result<DebugInfo>
where
    Pid: TryIntoProcessHandle + std::fmt::Display + Copy,
{
    let path = Path::new(path);
    let mut s = String::new();
    let mut file = try!(File::open(&path));
    try!(file.read_to_string(&mut s));
    let mut info: DebugInfo = serde_json::from_str(&s).unwrap();
    info.executor_globals_address = get_executor_globals_address(pid);
    Ok(info)
}

pub fn write_debuginfo_to_conif(debuginfo: &DebugInfo, path: &str) {
    let json = serde_json::to_string_pretty(debuginfo).unwrap();
    let path = Path::new(path);
    let display = path.display();
    let mut file = match File::create(&path) {
        Err(why) => panic!("Couldn't create {}: {}", display, why.description()),
        Ok(file) => file,
    };
    match file.write_all(json.as_bytes()) {
        Err(why) => panic!("Couldn't write to {}: {}", display, why.description()),
        Ok(_) => println!("Successfully wrote to {}", display),
    }
}

pub fn get_debug_info_from_dwarf<Pid>(pid: Pid, dwarf: &DwarfLookup) -> DebugInfo
where
    Pid: TryIntoProcessHandle + std::fmt::Display + Copy,
{
    let zend_executor_globals = dwarf.find_struct("_zend_executor_globals").unwrap();
    let current_execute_data_offset = zend_executor_globals
        .find_member("current_execute_data")
        .unwrap()
        .byte_offset;
    let zend_execute_data = dwarf.find_struct("_zend_execute_data").unwrap();
    let func_offset = zend_execute_data.find_member("func").unwrap().byte_offset;
    let this_offset = zend_execute_data.find_member("This").unwrap().byte_offset;
    let prev_offset = zend_execute_data
        .find_member("prev_execute_data")
        .unwrap()
        .byte_offset;

    let zend_function = dwarf.find_union("_zend_function").unwrap();
    let member_common = zend_function.find_member("common").unwrap();
    let common = dwarf.find_struct_by_id(member_common.type_id).unwrap();
    let function_name_offset = common.find_member("function_name").unwrap().byte_offset;
    let scope_offset = common.find_member("scope").unwrap().byte_offset;

    let zend_string = dwarf.find_struct("_zend_string").unwrap();
    let zend_string_len_offset = zend_string.find_member("len").unwrap().byte_offset;
    let zend_string_val_offset = zend_string.find_member("val").unwrap().byte_offset;

    let zend_vm_stack = dwarf.find_struct("_zend_vm_stack").unwrap();

    let zend_class_entry = dwarf.find_struct("_zend_class_entry").unwrap();

    DebugInfo {
        executor_globals_address: get_executor_globals_address(pid),
        eg_byte_size: zend_executor_globals.byte_size,
        ed_byte_size: zend_execute_data.byte_size,
        eg_current_execute_data_offset: current_execute_data_offset,
        ed_func_offset: func_offset,
        ed_this_offset: this_offset, // zend_value is the field of zval
        ed_prev_execute_data_offset: prev_offset,
        fu_function_name_offset: function_name_offset,
        eg_vm_stack_top_offset: zend_executor_globals
            .find_member("vm_stack_top")
            .unwrap()
            .byte_offset,
        eg_vm_stack_end_offset: zend_executor_globals
            .find_member("vm_stack_end")
            .unwrap()
            .byte_offset,
        eg_vm_stack_offset: zend_executor_globals
            .find_member("vm_stack")
            .unwrap()
            .byte_offset,
        zend_string_len_offset: zend_string_len_offset,
        zend_string_val_offset: zend_string_val_offset,
        stack_byte_size: zend_vm_stack.byte_size,
        stack_end_offset: zend_vm_stack.find_member("end").unwrap().byte_offset,
        fu_scope_offset: scope_offset,
        ce_name_offset: zend_class_entry.find_member("name").unwrap().byte_offset,
    }
}

fn get_executor_globals_address<Pid>(pid: Pid) -> usize
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

    let output = String::from_utf8(cat_command.stdout).unwrap();
    let re = Regex::new(r"(\w+).+xp.+?php").unwrap();
    let cap = re.captures(&output).unwrap();
    let address_str = cap.get(1).unwrap().as_str();
    usize::from_str_radix(address_str, 16).unwrap()
}

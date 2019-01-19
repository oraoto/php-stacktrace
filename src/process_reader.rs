#[allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

extern crate read_process_memory;

use crate::php73;
use crate::php72;
use crate::php56;

use read_process_memory::{copy_address, ProcessHandle};
use std::mem::{size_of, transmute};

pub trait ProcessReader {
    fn read(&self, addr: usize) -> Trace;
}

pub struct Trace {

}

pub struct PHP730 { pub source: ProcessHandle }
pub struct PHP720 { pub source: ProcessHandle }
pub struct PHP560 { pub source: ProcessHandle }

impl PHP730 {

    fn get_executor_global(&self, addr: usize) -> php73::zend_executor_globals
    {
        read_memory::<php73::zend_executor_globals>(&self.source, addr)
    }

    fn get_execute_data(&self, addr: usize) -> php73::zend_execute_data
    {
        read_memory::<php73::zend_execute_data>(&self.source, addr)
    }

    fn get_function(&self, addr: usize) -> php73::zend_function
    {
        read_memory::<php73::zend_function>(&self.source, addr)
    }

    fn get_string(&self, addr: usize) -> String
    {
        let zend_str = read_memory::<php73::zend_string>(&self.source, addr);
        let offset = unsafe { &(*(::std::ptr::null::<php73::zend_string>())).val as *const _ as usize };

        let val = copy_address(addr + offset, zend_str.len, &self.source).unwrap();
        unsafe { String::from_utf8_unchecked(val) }
    }
}

impl ProcessReader for PHP730 {

    fn read(&self, addr: usize) -> Trace
    {
        let eg = self.get_executor_global(addr);
        let mut ex_addr = eg.current_execute_data as usize;

        let mut output = String::new();

        while ex_addr != 0 {
            let ex = self.get_execute_data(ex_addr);

            let func_addr = ex.func as usize;
            if func_addr == 0 {
                break
            };

            let func = self.get_function(func_addr);

            unsafe {
                if func.common.scope as usize != 0 {
                    let ce = read_memory::<php73::zend_class_entry>(&self.source, func.common.scope as usize);
                    let scope = self.get_string(ce.name as usize);
                    output = output + &scope + "::";
                }
            }

            let function_name_addr = unsafe { func.common.function_name as usize };
            if function_name_addr == 0 {
                output = output + "main()";
            } else {
                let name = self.get_string(function_name_addr);
                output = output + &name + "()\n";
            }

            ex_addr = ex.prev_execute_data as usize;
        }
        println!("{}", output);
        Trace{}
    }
}

impl PHP720 {
    fn get_executor_global(&self, addr: usize) -> php72::zend_executor_globals
    {
        read_memory::<php72::zend_executor_globals>(&self.source, addr)
    }

    fn get_execute_data(&self, addr: usize) -> php72::zend_execute_data
    {
        read_memory::<php72::zend_execute_data>(&self.source, addr)
    }

    fn get_function(&self, addr: usize) -> php72::zend_function
    {
        read_memory::<php72::zend_function>(&self.source, addr)
    }

    fn get_string(&self, addr: usize) -> String
    {
        let zend_str = read_memory::<php72::zend_string>(&self.source, addr);
        let offset = unsafe { &(*(::std::ptr::null::<php72::zend_string>())).val as *const _ as usize };

        let val = copy_address(addr + offset, zend_str.len, &self.source).unwrap();
        unsafe { String::from_utf8_unchecked(val) }
    }
}

impl ProcessReader for PHP720 {

    fn read(&self, addr: usize) -> Trace
    {
        let eg = self.get_executor_global(addr);
        let mut ex_addr = eg.current_execute_data as usize;

        let mut output = String::new();

        while ex_addr != 0 {
            let ex = self.get_execute_data(ex_addr);

            let func_addr = ex.func as usize;
            if func_addr == 0 {
                break
            };

            let func = self.get_function(func_addr);

            unsafe {
                if func.common.scope as usize != 0 {
                    let ce = read_memory::<php73::zend_class_entry>(&self.source, func.common.scope as usize);
                    let scope = self.get_string(ce.name as usize);
                    output = output + &scope + "::";
                }
            }

            let function_name_addr = unsafe { func.common.function_name as usize };
            if function_name_addr == 0 {
                output = output + "main()";
            } else {
                let name = self.get_string(function_name_addr);
                output = output + &name + "()\n";
            }
            ex_addr = ex.prev_execute_data as usize;
        }
        println!("{}", output);
        Trace{}
    }
}

impl PHP560 {

    fn get_executor_global(&self, addr: usize) -> php56::zend_executor_globals
    {
        read_memory::<php56::zend_executor_globals>(&self.source, addr)
    }

    fn get_execute_data(&self, addr: usize) -> php56::zend_execute_data
    {
        read_memory::<php56::zend_execute_data>(&self.source, addr)
    }

    fn get_function(&self, addr: usize) -> php56::zend_function
    {
        read_memory::<php56::zend_function>(&self.source, addr)
    }

    fn get_string(&self, addr: usize) -> String
    {
        read_cstr(&self.source, addr)
    }
}

impl ProcessReader for PHP560 {

    fn read(&self, addr: usize) -> Trace
    {
        let eg = self.get_executor_global(addr);
        let mut ex_addr = eg.current_execute_data as usize;

        let mut output = String::new();

        while ex_addr != 0 {
            let ex = self.get_execute_data(ex_addr);

            let func_addr = ex.function_state.function as usize;
            if func_addr == 0 {
                break
            };

            let func = self.get_function(func_addr);

            unsafe {
                if func.common.scope as usize != 0 {
                    let ce = read_memory::<php56::zend_class_entry>(&self.source, func.common.scope as usize);
                    let scope = read_cstr(&self.source, ce.name as usize);
                    output = output + &scope + "::";
                }
            }

            let function_name_addr = unsafe { func.common.function_name as usize };
            if function_name_addr == 0 {
                output = output + "main()";
            } else {
                let name = self.get_string(function_name_addr);
                output = output + &name + "()\n";
            }
            ex_addr = ex.prev_execute_data as usize;
        }
        print!("{}", output);
        Trace{}
    }
}

fn read_memory<R>(source: &ProcessHandle, addr: usize) -> R
where R: Copy
{
    let size = size_of::<R>();
    let bytes = copy_address(addr, size, source).unwrap();
    let bytes_ptr: *mut R = unsafe { transmute(bytes.as_ptr()) };
    unsafe { (*bytes_ptr) }
}

fn read_cstr(source: &ProcessHandle, addr: usize) -> String
{
    let mut result = String::new();
    let mut i = 0;
    loop {
        let c = copy_address(addr + i, 1, source).unwrap()[0];
        if c == 0 {
            return result;
        } else {
            result.push(c as char);
            i = i + 1;
        }
    }
}
extern crate read_process_memory;

use php73;
use php72;

use read_process_memory::*;
use std::mem::*;

// To stupid
pub trait ProcessReader<EG, ED, F, S> {
    fn get_executor_global<T>(&self, source: &T, addr: usize) -> EG where T:CopyAddress;
    fn get_execute_data<T>(&self, source: &T, addr: usize) -> ED where T:CopyAddress;
    fn get_function<T>(&self, source: &T, addr: usize) -> F where T:CopyAddress;
    fn get_string<T>(&self, source: &T, addr: usize) -> String where T:CopyAddress;
}

pub struct PHP730 { }
pub struct PHP720 { }

impl ProcessReader<
    php73::zend_executor_globals,
    php73::zend_execute_data,
    php73::zend_function,
    php73::zend_string> for PHP730 {
    fn get_executor_global<T>(&self, source: &T, addr: usize) -> php73::zend_executor_globals
    where T: CopyAddress
    {
        read_memory::<T, php73::zend_executor_globals>(source, addr)
    }

    fn get_execute_data<T>(&self, source: &T, addr: usize) -> php73::zend_execute_data
    where T: CopyAddress
    {
        read_memory::<T, php73::zend_execute_data>(source, addr)
    }

    fn get_function<T>(&self, source: &T, addr: usize) -> php73::zend_function
    where T: CopyAddress
    {
        read_memory::<T, php73::zend_function>(source, addr)
    }

    fn get_string<T>(&self, source: &T, addr: usize) -> String
    where T: CopyAddress
    {
        let zend_str = read_memory::<T, php73::zend_string>(source, addr);
        let offset = unsafe { &(*(::std::ptr::null::<php73::zend_string>())).val as *const _ as usize };

        let val = copy_address(addr + offset, zend_str.len, source).unwrap();
        unsafe { String::from_utf8_unchecked(val) }
    }
}


impl ProcessReader<
    php72::zend_executor_globals,
    php72::zend_execute_data,
    php72::zend_function,
    php72::zend_string> for PHP720 {
    fn get_executor_global<T>(&self, source: &T, addr: usize) -> php72::zend_executor_globals
    where T: CopyAddress
    {
        read_memory::<T, php72::zend_executor_globals>(source, addr)
    }

    fn get_execute_data<T>(&self, source: &T, addr: usize) -> php72::zend_execute_data
    where T: CopyAddress
    {
        read_memory::<T, php72::zend_execute_data>(source, addr)
    }

    fn get_function<T>(&self, source: &T, addr: usize) -> php72::zend_function
    where T: CopyAddress
    {
        read_memory::<T, php72::zend_function>(source, addr)
    }

    fn get_string<T>(&self, source: &T, addr: usize) -> String
    where T: CopyAddress
    {
        let zend_str = read_memory::<T, php72::zend_string>(source, addr);
        let offset = unsafe { &(*(::std::ptr::null::<php72::zend_string>())).val as *const _ as usize };

        let val = copy_address(addr + offset, zend_str.len, source).unwrap();
        unsafe { String::from_utf8_unchecked(val) }
    }
}


fn read_memory<T, R>(source: &T, addr: usize) -> R
where T: CopyAddress, R: Copy
{
    let size = size_of::<R>();
    let bytes = copy_address(addr, size, source).unwrap();
    let bytes_ptr: *mut R = unsafe { transmute(bytes.as_ptr()) };
    return unsafe { (*bytes_ptr) };
}

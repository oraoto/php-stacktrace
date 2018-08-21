#![feature(duration_as_u128)]
#[macro_use]
extern crate serde_derive;

mod debuginfo;
mod dwarf;
mod attach;
mod process_reader;
mod php73;
mod php72;

extern crate byteorder;
extern crate clap;
extern crate libc;
extern crate read_process_memory;
extern crate regex;


use read_process_memory::*;
use std::thread;
use std::time::Duration;
use std::time;
use clap::{App, Arg, ArgMatches};
use debuginfo::*;
use process_reader::*;

fn main()
{
    let matches = parse_args();

    let pid: Pid = matches.value_of("PID").unwrap().parse().unwrap();

    let source = pid.try_into_process_handle().unwrap();

    let addr = get_executor_globals_address(source);

    let start_time  = time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();

    let php_version = matches.value_of("PHP Version").unwrap();

    let php = process_reader::PHP720{};

    attach::attach(pid);

    let eg = php.get_executor_global(&source, addr);

    let mut ex_addr = eg.current_execute_data as usize;

    while ex_addr != 0 {

        let ex = php.get_execute_data(&source, ex_addr);

        let func_addr = ex.func as usize;
        if func_addr == 0 {
            break
        };

        let func = php.get_function(&source, func_addr);

        let function_name_addr = unsafe { func.common.function_name as usize };
        if function_name_addr == 0 {
            println!("main()");
        } else {
            let name = php.get_string(&source, function_name_addr);
            println!("{}", name);
        }
        ex_addr = ex.prev_execute_data as usize;
    }

    attach::detach(pid);

    let end_time = time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();

    let dur = end_time - start_time;
    let dur_ns = dur.as_nanos();
    let dur_ms = dur_ns as f32 / 1000_000.0;
    println!("Time {:?}  {:?}", end_time, start_time);
    println!("Time {} ns {} ms", dur_ns, dur_ms);

}

fn parse_args() -> ArgMatches<'static> {
    App::new("php-stacktrace")
        .version("0.2.0")
        .about("Read stacktrace from outside PHP process")
        .arg(
            Arg::with_name("PHP Version")
                .value_name("php_version")
                .short("v")
                .help("PHP Version (720, 730)")
                .default_value("720")
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

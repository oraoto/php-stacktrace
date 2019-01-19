
#[macro_use]
extern crate serde_derive;

mod debuginfo;
mod attach;
mod process_reader;
mod php73;
mod php72;
mod php56;








use read_process_memory::*;
use std::time;
use clap::{App, Arg, ArgMatches};
use crate::debuginfo::*;
use crate::process_reader::ProcessReader;

fn create_reader(version: &str, source: ProcessHandle) -> Box<dyn ProcessReader>
{
    if version == "5.6" {
        Box::new(process_reader::PHP560{source})
    } else if version == "7.2" {
        Box::new(process_reader::PHP720{source})
    } else {
        Box::new(process_reader::PHP730{source})
    }
}

fn main()
{
    let matches = parse_args();

    let pid: Pid = matches.value_of("PID").unwrap().parse().unwrap();

    let source = pid.try_into_process_handle().unwrap();

    let addr = get_executor_globals_address(source);
  
    let php_version = matches.value_of("PHP Version").unwrap();
    let php = create_reader(php_version, source);

    let start_time  = time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();

    attach::attach(pid);

    php.read(addr);

    attach::detach(pid);

    let end_time = time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();
    let dur = end_time - start_time;
    println!("Time {:?}", dur);
}

fn parse_args() -> ArgMatches<'static> {
    App::new("php-stacktrace")
        .version("0.2.0")
        .about("Read stacktrace from outside PHP process")
        .arg(
            Arg::with_name("PHP Version")
                .value_name("php_version")
                .short("v")
                .help("PHP Version (5.6, 7.2, 7.3)")
                .default_value("7.3")
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

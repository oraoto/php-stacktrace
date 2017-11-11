extern crate libc;
extern crate read_process_memory;

use libc::{c_int, c_long};

#[cfg(target_os = "linux")]
extern {
    // addr and data should be c_void?
    fn ptrace(request: c_int, pid: libc::pid_t, addr: usize, data: usize) -> c_long;
}

#[cfg(target_os = "linux")]
pub fn attach(pid: read_process_memory::Pid) {
    unsafe {
        ptrace(0x4206, pid, 0, 0); // PTRACE_SEIZE
        ptrace(0x4207, pid, 0, 0); // PTRACE_INTERRUPT
    }
}

#[cfg(target_os = "linux")]
pub fn detach(pid: read_process_memory::Pid) {
    unsafe {
        ptrace(17, pid, 0, 0); // PTRACE_DETACH
    }
}

#[cfg(not(target_os = "linux"))]
#[allow(dead_code)]
pub fn detach(_: read_process_memory::Pid) {

}

#[cfg(not(target_os = "linux"))]
#[allow(dead_code)]
pub fn attach(_: read_process_memory::Pid) {

}

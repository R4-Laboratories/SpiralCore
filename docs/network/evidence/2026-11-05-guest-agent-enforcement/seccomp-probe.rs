// This direct Linux workload inspects the filter it inherited. It installs none.
use std::{
    io,
    os::fd::{FromRawFd, OwnedFd},
};

unsafe extern "C" {
    fn socket(domain: i32, kind: i32, protocol: i32) -> i32;
}

fn open_socket(family: i32) -> io::Result<OwnedFd> {
    // SAFETY: socket takes integer values and returns a newly owned descriptor.
    let fd = unsafe { socket(family, 1 | 0x80000, 0) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful socket returned a new descriptor owned by this process.
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

fn main() {
    if std::env::args_os().len() != 1 {
        std::process::exit(20);
    }
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else {
        std::process::exit(21);
    };
    let filtered = status.lines().any(|line| {
        line.strip_prefix("Seccomp:")
            .is_some_and(|mode| mode.trim() == "2")
    });
    if !filtered {
        std::process::exit(22);
    }
    if !matches!(open_socket(2), Err(error) if error.raw_os_error() == Some(1)) {
        std::process::exit(23);
    }
    if open_socket(40).is_err() {
        std::process::exit(24);
    }
    println!(r#"{{"seccomp":2,"af_inet":"EPERM","af_vsock":"created"}}"#);
}

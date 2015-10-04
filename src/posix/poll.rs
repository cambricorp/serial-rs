#![allow(non_camel_case_types,dead_code)]

extern crate libc;
extern crate time;

use std::io;

use time::Duration;

use self::libc::{c_int,c_short};


#[cfg(target_os = "linux")]
type nfds_t = libc::c_ulong;

#[cfg(not(target_os = "linux"))]
type nfds_t = libc::c_uint;

#[derive(Debug)]
#[repr(C)]
struct PollFd {
    fd: c_int,
    events: c_short,
    revents: c_short
}

const POLLIN:   c_short = 0x0001;
const POLLPRI:  c_short = 0x0002;
const POLLOUT:  c_short = 0x0004;

const POLLERR:  c_short = 0x0008;
const POLLHUP:  c_short = 0x0010;
const POLLNVAL: c_short = 0x0020;

pub fn wait_read_fd(fd: c_int, timeout: Duration) -> io::Result<()> {
    wait_fd(fd, POLLIN, timeout)
}

pub fn wait_write_fd(fd: c_int, timeout: Duration) -> io::Result<()> {
    wait_fd(fd, POLLOUT, timeout)
}

fn wait_fd(fd: c_int, events: c_short, timeout: Duration) -> io::Result<()> {
    use self::libc::{EINTR,EPIPE,EIO};

    let mut fds = vec!(PollFd { fd: fd, events: events, revents: 0 });

    let wait = do_poll(&mut fds, timeout);

    if wait < 0 {
        let errno = super::error::errno();

        let kind = match errno {
            EINTR => io::ErrorKind::Interrupted,
            _ => io::ErrorKind::Other
        };

        return Err(io::Error::new(kind, super::error::error_string(errno)));
    }

    if wait == 0 {
        return Err(io::Error::new(io::ErrorKind::TimedOut, "Operation timed out"));
    }

    if fds[0].revents & events != 0 {
        return Ok(());
    }

    if fds[0].revents & (POLLHUP | POLLNVAL) != 0 {
        return Err(io::Error::new(io::ErrorKind::BrokenPipe, super::error::error_string(EPIPE)));
    }

    Err(io::Error::new(io::ErrorKind::Other, super::error::error_string(EIO)))
}

#[cfg(target_os = "linux")]
#[inline]
fn do_poll(fds: &mut Vec<PollFd>, timeout: Duration) -> c_int {
    use std::ptr;

    #[repr(C)]
    struct sigset_t;

    extern "C" {
        fn ppoll(fds: *mut PollFd, nfds: nfds_t, timeout_ts: *mut self::libc::timespec, sigmask: *const sigset_t) -> c_int;
    }

    let seconds     = timeout.num_seconds();
    let nanoseconds = ((timeout - Duration::seconds(seconds)) * 1_000_000_000).num_seconds();

    let mut timeout_ts = self::libc::timespec {
        tv_sec: seconds as libc::time_t,
        tv_nsec: nanoseconds as libc::c_long
    };

    unsafe {
        ppoll((&mut fds[..]).as_mut_ptr(),
              fds.len() as nfds_t,
              &mut timeout_ts,
              ptr::null())
    }
}

#[cfg(not(target_os = "linux"))]
#[inline]
fn do_poll(fds: &mut Vec<PollFd>, timeout: Duration) -> c_int {
    extern "C" {
        fn poll(fds: *mut PollFd, nfds: nfds_t, timeout: c_int) -> c_int;
    }

    unsafe {
        poll((&mut fds[..]).as_mut_ptr(),
             fds.len() as nfds_t,
             timeout.num_milliseconds() as c_int)
    }
}
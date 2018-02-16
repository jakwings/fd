use std::io::{self, Read, Write};
use std::os::unix::io::AsRawFd;
use std::sync::Arc;
use std::sync::atomic::{self, AtomicBool};
use std::thread;
use std::time;

use super::nix::errno::Errno;
use super::nix::libc;

const BUF_SIZE: usize = 512;
const INTERVAL: u32 = 500 * 1000; // 500 microseconds
const MAX_CNT: u32 = 500;

fn loop_counter(counter: &mut u32) {
    *counter = if *counter < MAX_CNT { *counter + 1 } else { 1 };
}

unsafe fn get_flag<T: AsRawFd>(obj: &T) -> libc::c_int {
    libc::fcntl(obj.as_raw_fd(), libc::F_GETFL, 0 /* ignored */)
}

unsafe fn set_flags<T: AsRawFd>(obj: &T, flags: libc::c_int) -> bool {
    libc::fcntl(obj.as_raw_fd(), libc::F_SETFL, flags) != -1
}

// Return: whether it was nonblocking
pub unsafe fn set_nonblocking<T: AsRawFd>(obj: &T) -> Result<bool, &str> {
    let flags = get_flag(obj);

    if (flags & libc::O_NONBLOCK) != 0 {
        Ok(true)
    } else {
        match set_flags(obj, flags | libc::O_NONBLOCK) {
            true => Ok(false),
            false => Err(Errno::last().desc()),
        }
    }
}

// Return: whether it was blocking
pub unsafe fn set_blocking<T: AsRawFd>(obj: &T) -> Result<bool, &str> {
    let flags = get_flag(obj);

    if (flags & libc::O_NONBLOCK) == 0 {
        Ok(true)
    } else {
        match set_flags(obj, flags & !libc::O_NONBLOCK) {
            true => Ok(false),
            false => Err(Errno::last().desc()),
        }
    }
}

fn load_bool(atom: &Arc<AtomicBool>) -> bool {
    atom.load(atomic::Ordering::Relaxed)
}

pub fn try_read_to_end<R: Read>(
    atom: &Arc<AtomicBool>,
    reader: &mut R,
    content: &mut Vec<u8>,
) -> io::Result<Option<usize>> {
    let mut total = 0;
    let mut buffer = [0; BUF_SIZE];
    let mut counter = 0;
    let interval = time::Duration::new(0, INTERVAL);

    loop {
        if counter >= MAX_CNT && load_bool(&atom) {
            return Ok(None);
        } else {
            loop_counter(&mut counter);
        }
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(size) => total += content.write(&buffer[0..size]).unwrap(),
            Err(err) => {
                if err.kind() != io::ErrorKind::WouldBlock {
                    return Err(err);
                }
                thread::sleep(interval);
            }
        }
    }

    Ok(Some(total))
}

pub fn try_write_all<W: Write>(
    atom: &Arc<AtomicBool>,
    writer: &mut W,
    content: &Vec<u8>,
) -> io::Result<Option<()>> {
    let mut total = 0;
    let length = content.len();
    let mut counter = 0;
    let interval = time::Duration::new(0, INTERVAL);

    loop {
        if counter >= MAX_CNT && load_bool(&atom) {
            return Ok(None);
        } else {
            loop_counter(&mut counter);
        }

        let range = total..(BUF_SIZE + total).min(length);

        match writer.write(&content[range]) {
            Ok(0) => break,
            Ok(size) => total += size,
            Err(err) => {
                if err.kind() != io::ErrorKind::WouldBlock {
                    return Err(err);
                }
                thread::sleep(interval);
            }
        }
    }

    Ok(Some(()))
}

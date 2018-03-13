use std::io::{self, Read, Write};
use std::os::unix::io::RawFd;
use std::sync::Arc;
use std::sync::atomic::{self, AtomicBool};

use super::nix::Error;
use super::nix::errno::Errno;
use super::nix::sys::select;
use super::nix::sys::time::{TimeVal, TimeValLike};

const BUF_SIZE: usize = 512;
const INTERVAL: i64 = 500 * 1000; // 500 microseconds
const MAX_CNT: u32 = 500;

fn loop_counter(counter: &mut u32) {
    *counter = if *counter < MAX_CNT { *counter + 1 } else { 1 };
}

fn load_bool(atom: &Arc<AtomicBool>) -> bool {
    atom.load(atomic::Ordering::Relaxed)
}

pub fn select_read_to_end<R: Read>(
    atom: &Arc<AtomicBool>,
    fd: RawFd,
    reader: &mut R,
    content: &mut Vec<u8>,
) -> io::Result<Option<usize>> {
    let mut total = 0;
    let mut buffer = [0; BUF_SIZE];
    let mut counter = 0;
    let mut fdset = select::FdSet::new();

    loop {
        if counter >= MAX_CNT && load_bool(&atom) {
            return Ok(None);
        } else {
            loop_counter(&mut counter);
        }

        let mut interval = TimeVal::nanoseconds(INTERVAL);

        fdset.insert(fd);

        match select::select(
            Some(fd + 1),
            Some(&mut fdset),
            None,
            None,
            Some(&mut interval),
        ) {
            Ok(0) => (), // timeout
            Ok(_) => {
                if fdset.contains(fd) {
                    match reader.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(size) => total += content.write(&buffer[0..size]).unwrap(),
                        Err(err) => {
                            if err.kind() != io::ErrorKind::WouldBlock {
                                return Err(err);
                            }
                        }
                    }
                } else {
                    unreachable!("[Error] unknown bugs about select(2)");
                }
            }
            Err(Error::Sys(Errno::EINTR)) => (),
            Err(err) => {
                use std::error::Error;

                return Err(io::Error::new(io::ErrorKind::Other, err.description()));
            }
        }
    }

    Ok(Some(total))
}

pub fn select_write_all<W: Write>(
    atom: &Arc<AtomicBool>,
    fd: RawFd,
    writer: &mut W,
    content: &Vec<u8>,
) -> io::Result<Option<()>> {
    let mut total = 0;
    let length = content.len();
    let mut counter = 0;
    let mut fdset = select::FdSet::new();

    loop {
        if counter >= MAX_CNT && load_bool(&atom) {
            return Ok(None);
        } else {
            loop_counter(&mut counter);
        }

        let range = total..(BUF_SIZE + total).min(length);
        let mut interval = TimeVal::nanoseconds(INTERVAL);

        fdset.insert(fd);

        match select::select(
            Some(fd + 1),
            None,
            Some(&mut fdset),
            None,
            Some(&mut interval),
        ) {
            Ok(0) => (), // timeout
            Ok(_) => {
                if fdset.contains(fd) {
                    match writer.write(&content[range]) {
                        Ok(0) => break,
                        Ok(size) => total += size,
                        Err(err) => {
                            if err.kind() != io::ErrorKind::WouldBlock {
                                return Err(err);
                            }
                        }
                    }
                } else {
                    unreachable!("[Error] unknown bugs about select(2)");
                }
            }
            Err(Error::Sys(Errno::EINTR)) => (),
            Err(err) => {
                use std::error::Error;

                return Err(io::Error::new(io::ErrorKind::Other, err.description()));
            }
        }
    }

    Ok(Some(()))
}

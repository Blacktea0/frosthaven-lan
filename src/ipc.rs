#![allow(dead_code)]
//! Process-lifetime memory IPC; no status, command, or log files are written.
use crate::win::*;
use std::{
    io, ptr,
    sync::atomic::{AtomicU32, Ordering},
};
pub const STARTUP: usize = 0;
pub const REQUEST: usize = 16384;
pub const RESPONSE: usize = 32768;
const SLOT: usize = 16384;
const SIZE: usize = 65536;
pub struct Channel {
    mapping: Handle,
    view: *mut u8,
}
impl Channel {
    fn map(mapping: Handle) -> io::Result<Self> {
        if mapping.is_null() {
            return Err(io::Error::last_os_error());
        }
        let view = unsafe { MapViewOfFile(mapping, 0xf001f, 0, 0, SIZE) };
        if view.is_null() {
            let error = io::Error::last_os_error();
            unsafe {
                CloseHandle(mapping);
            }
            return Err(error);
        }
        Ok(Self {
            mapping,
            view: view.cast(),
        })
    }
    pub fn create(pid: u32) -> io::Result<Self> {
        Self::map(unsafe {
            CreateFileMappingW(
                -1isize as Handle,
                ptr::null_mut(),
                4,
                0,
                SIZE as u32,
                wide(&format!("Local\\FrosthavenLan-{pid}")).as_ptr(),
            )
        })
    }
    pub fn open(pid: u32) -> io::Result<Self> {
        Self::map(unsafe {
            OpenFileMappingW(
                0xf001f,
                0,
                wide(&format!("Local\\FrosthavenLan-{pid}")).as_ptr(),
            )
        })
    }
    pub fn write(&self, slot: usize, text: &str) {
        assert!(matches!(slot, STARTUP | REQUEST | RESPONSE));
        let bytes = text.as_bytes();
        let count = bytes.len().min(SLOT - 8);
        unsafe {
            let seq = &*self.view.add(slot).cast::<AtomicU32>();
            let before = seq.load(Ordering::SeqCst) & !1;
            seq.store(before.wrapping_add(1), Ordering::SeqCst);
            ptr::copy_nonoverlapping(bytes.as_ptr(), self.view.add(slot + 8), count);
            ptr::write_volatile(self.view.add(slot + 4).cast::<u32>(), count as u32);
            seq.store(before.wrapping_add(2), Ordering::SeqCst);
        }
    }
    pub fn read(&self, slot: usize) -> Option<String> {
        assert!(matches!(slot, STARTUP | REQUEST | RESPONSE));
        unsafe {
            let seq = &*self.view.add(slot).cast::<AtomicU32>();
            for _ in 0..8 {
                let before = seq.load(Ordering::SeqCst);
                if before == 0 || before & 1 != 0 {
                    continue;
                }
                let count = ptr::read_volatile(self.view.add(slot + 4).cast::<u32>()) as usize;
                if count > SLOT - 8 {
                    return None;
                }
                let mut bytes = vec![0; count];
                ptr::copy_nonoverlapping(self.view.add(slot + 8), bytes.as_mut_ptr(), count);
                std::sync::atomic::fence(Ordering::SeqCst);
                if seq.load(Ordering::SeqCst) == before {
                    return Some(String::from_utf8_lossy(&bytes).into_owned());
                }
            }
            None
        }
    }
}
impl Drop for Channel {
    fn drop(&mut self) {
        unsafe {
            UnmapViewOfFile(self.view.cast());
            CloseHandle(self.mapping);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_slots_preserve_unicode_and_command_responses() {
        let pid = unsafe { GetCurrentProcessId() };
        let writer = Channel::create(pid).unwrap();
        let reader = Channel::open(pid).unwrap();
        writer.write(STARTUP, "READY 网络就绪");
        reader.write(REQUEST, "token|status");
        writer.write(RESPONSE, "token|后端 ENet");
        assert_eq!(reader.read(STARTUP).unwrap(), "READY 网络就绪");
        assert_eq!(writer.read(REQUEST).unwrap(), "token|status");
        assert_eq!(reader.read(RESPONSE).unwrap(), "token|后端 ENet");
    }
}

use std::io::BufRead;
use std::sync::OnceLock;
use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{FILE_TYPE_CHAR, FILE_TYPE_DISK, FILE_TYPE_PIPE, GetFileType, ReadFile};
use windows_sys::Win32::System::Console::{
    CTRL_BREAK_EVENT, CTRL_C_EVENT, GetNumberOfConsoleInputEvents, GetStdHandle, INPUT_RECORD, KEY_EVENT, PeekConsoleInputW, STD_INPUT_HANDLE, SetConsoleCtrlHandler,
};
use windows_sys::Win32::System::Pipes::PeekNamedPipe;
use windows_sys::core::BOOL;

const VK_RETURN: u16 = 0x0D;

static HANDLER: OnceLock<fn()> = OnceLock::new();

unsafe extern "system" fn on_ctrl(kind: u32) -> BOOL {
    if kind != CTRL_C_EVENT && kind != CTRL_BREAK_EVENT {
        return 0;
    }
    if let Some(handler) = HANDLER.get() {
        handler();
    }
    1
}

pub fn install_interrupt_handler(handler: fn()) -> bool {
    let _ = HANDLER.set(handler);
    unsafe { SetConsoleCtrlHandler(Some(on_ctrl), 1) != 0 }
}

pub fn read_available(buffer: &mut [u8]) -> Option<usize> {
    let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return None;
    }
    match unsafe { GetFileType(handle) } {
        FILE_TYPE_CHAR => read_console(handle, buffer),
        FILE_TYPE_PIPE => {
            let mut available = 0u32;
            if unsafe { PeekNamedPipe(handle, std::ptr::null_mut(), 0, std::ptr::null_mut(), &mut available, std::ptr::null_mut()) } == 0 {
                return None;
            }
            if available == 0 {
                return Some(0);
            }
            let length = buffer.len().min(available as usize);
            read_file(handle, &mut buffer[..length])
        }
        FILE_TYPE_DISK => read_file(handle, buffer),
        _ => None,
    }
}

fn read_file(handle: HANDLE, buffer: &mut [u8]) -> Option<usize> {
    let mut read = 0u32;
    let ok = unsafe { ReadFile(handle, buffer.as_mut_ptr(), buffer.len() as u32, &mut read, std::ptr::null_mut()) };
    if ok == 0 || read == 0 { None } else { Some(read as usize) }
}

fn read_console(handle: HANDLE, buffer: &mut [u8]) -> Option<usize> {
    let mut count = 0u32;
    if unsafe { GetNumberOfConsoleInputEvents(handle, &mut count) } == 0 {
        return None;
    }
    if count == 0 {
        return Some(0);
    }
    let mut records: Vec<INPUT_RECORD> = vec![unsafe { std::mem::zeroed() }; count as usize];
    let mut peeked = 0u32;
    if unsafe { PeekConsoleInputW(handle, records.as_mut_ptr(), count, &mut peeked) } == 0 {
        return None;
    }
    let line_ready = records[..peeked as usize].iter().any(|record| {
        record.EventType as u32 == KEY_EVENT && {
            let key = unsafe { record.Event.KeyEvent };
            key.bKeyDown != 0 && key.wVirtualKeyCode == VK_RETURN
        }
    });
    if !line_ready {
        return Some(0);
    }

    let mut line = String::new();
    match std::io::stdin().lock().read_line(&mut line) {
        Ok(0) | Err(_) => None,
        Ok(_) => {
            if !line.ends_with('\n') {
                line.push('\n');
            }
            let length = line.len().min(buffer.len());
            buffer[..length].copy_from_slice(&line.as_bytes()[..length]);
            Some(length)
        }
    }
}

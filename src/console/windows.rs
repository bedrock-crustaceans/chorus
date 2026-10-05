use std::os::windows::io::AsRawHandle;
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::Storage::FileSystem::{FILE_TYPE_DISK, FILE_TYPE_PIPE, GetFileType, ReadFile};
use windows_sys::Win32::System::Pipes::PeekNamedPipe;

pub fn read_available(buffer: &mut [u8]) -> Option<usize> {
    let handle = std::io::stdin().as_raw_handle() as HANDLE;
    if handle.is_null() {
        return None;
    }
    match unsafe { GetFileType(handle) } {
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

use std::io;

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_INVALID_PARAMETER, GetLastError,
};
use windows_sys::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
    TerminateProcess, WaitForSingleObject,
};

const SYNCHRONIZE: u32 = 0x0010_0000;

pub fn stop_process(pid: u32) -> Result<bool, io::Error> {
    // First attempt tree termination to cleanly stop child subprocesses
    let _ = std::process::Command::new("taskkill")
        .args(["/F", "/T", "/PID", &pid.to_string()])
        .output();

    let handle = unsafe { OpenProcess(PROCESS_TERMINATE, 0, pid) };

    if handle.is_null() {
        let error = unsafe { GetLastError() };

        return match error {
            ERROR_INVALID_PARAMETER => Ok(false),
            _ => Err(io::Error::from_raw_os_error(error as i32)),
        };
    }

    let result = unsafe { TerminateProcess(handle, 1) };
    let error = if result == 0 {
        Some(unsafe { GetLastError() })
    } else {
        None
    };

    unsafe {
        CloseHandle(handle);
    }

    match error {
        None => Ok(true),
        Some(code) => Err(io::Error::from_raw_os_error(code as i32)),
    }
}

pub fn is_process_active(pid: u32) -> Result<bool, io::Error> {
    let handle = unsafe { OpenProcess(SYNCHRONIZE, 0, pid) };

    if handle.is_null() {
        let error = unsafe { GetLastError() };

        return match error {
            ERROR_INVALID_PARAMETER => Ok(false),
            ERROR_ACCESS_DENIED => Ok(true),
            _ => Err(io::Error::from_raw_os_error(error as i32)),
        };
    }

    let result = unsafe { WaitForSingleObject(handle, 0) };

    unsafe {
        CloseHandle(handle);
    }

    match result {
        0 => Ok(false),
        0x0000_0102 => Ok(true),
        _ => Err(io::Error::last_os_error()),
    }
}

pub fn get_process_start_time(pid: u32) -> Result<(u64, u64), io::Error> {
    use windows_sys::Win32::Foundation::FILETIME;

    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };

    if handle.is_null() {
        let error = unsafe { GetLastError() };
        return Err(io::Error::from_raw_os_error(error as i32));
    }

    let mut creation = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut exit = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut kernel = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut user = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };

    let result =
        unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) };

    unsafe {
        CloseHandle(handle);
    }

    if result == 0 {
        let error = unsafe { GetLastError() };
        return Err(io::Error::from_raw_os_error(error as i32));
    }

    // Combine FILETIME's two 32-bit fields into one 64-bit timestamp.
    let creation_ticks = ((creation.dwHighDateTime as u64) << 32) | creation.dwLowDateTime as u64;

    // Windows FILETIME counts 100-nanosecond intervals since 1601-01-01.
    const WINDOWS_TO_UNIX_EPOCH_SECONDS: u64 = 11_644_473_600;
    const TICKS_PER_SECOND: u64 = 10_000_000;

    let unix_ticks = creation_ticks
        .checked_sub(WINDOWS_TO_UNIX_EPOCH_SECONDS * TICKS_PER_SECOND)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Process creation time predates the Unix epoch",
            )
        })?;

    Ok((
        unix_ticks / TICKS_PER_SECOND,
        (unix_ticks % TICKS_PER_SECOND) / 10,
    ))
}

pub fn get_process_metrics(_pid: u32) -> (Option<f32>, Option<u64>) {
    (None, None)
}

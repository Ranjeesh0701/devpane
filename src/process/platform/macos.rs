use std::{io, mem};

pub fn stop_process(pid: u32) -> Result<bool, io::Error> {
    let pid = pid_to_i32(pid)?;

    // First attempt to terminate the entire process group to avoid orphaned child processes
    let pgid_result = unsafe { libc::kill(-pid, libc::SIGTERM) };
    if pgid_result == 0 {
        return Ok(true);
    }

    // Fall back to direct process signal if process group is not available
    let result = unsafe { libc::kill(pid, libc::SIGTERM) };

    if result == 0 {
        return Ok(true);
    }

    let err = io::Error::last_os_error();

    match err.raw_os_error() {
        Some(libc::ESRCH) => Ok(false),
        _ => Err(err),
    }
}

pub fn is_process_active(pid: u32) -> Result<bool, io::Error> {
    let pid = pid_to_i32(pid)?;

    let result = unsafe { libc::kill(pid, 0) };

    if result == 0 {
        return Ok(true);
    }

    let err = io::Error::last_os_error();

    match err.raw_os_error() {
        Some(libc::ESRCH) => Ok(false),
        Some(libc::EPERM) => Ok(true),
        _ => Err(err),
    }
}

pub fn get_process_start_time(pid: u32) -> Result<(u64, u64), io::Error> {
    let pid = pid_to_i32(pid)?;

    let mut info: libc::proc_bsdinfo = unsafe { mem::zeroed() };
    let size = mem::size_of::<libc::proc_bsdinfo>() as i32;

    let result = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDTBSDINFO,
            0,
            &mut info as *mut _ as *mut libc::c_void,
            size,
        )
    };

    if result <= 0 {
        return Err(io::Error::last_os_error());
    }

    if result < size {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "short proc_pidinfo result",
        ));
    }

    Ok((info.pbi_start_tvsec, info.pbi_start_tvusec))
}

fn pid_to_i32(pid: u32) -> Result<i32, io::Error> {
    i32::try_from(pid).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "pid out of range"))
}

pub fn get_process_metrics(pid: u32) -> (Option<f32>, Option<u64>) {
    let Ok(pid_i32) = pid_to_i32(pid) else {
        return (None, None);
    };

    let mut task_info: libc::proc_taskinfo = unsafe { mem::zeroed() };
    let size = mem::size_of::<libc::proc_taskinfo>() as i32;

    let result = unsafe {
        libc::proc_pidinfo(
            pid_i32,
            libc::PROC_PIDTASKINFO,
            0,
            &mut task_info as *mut _ as *mut libc::c_void,
            size,
        )
    };

    let memory_bytes = if result == size {
        Some(task_info.pti_resident_size)
    } else {
        None
    };

    let cpu = get_cpu_from_ps(pid);
    (cpu, memory_bytes)
}

fn get_cpu_from_ps(pid: u32) -> Option<f32> {
    let out = std::process::Command::new("ps")
        .args(["-o", "%cpu=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    s.trim().parse::<f32>().ok()
}

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
pub use macos::{
    get_process_metrics, get_process_start_time, is_process_active, reap_zombies, stop_process,
};

#[cfg(target_os = "windows")]
pub use windows::{
    get_process_metrics, get_process_start_time, is_process_active, reap_zombies, stop_process,
};

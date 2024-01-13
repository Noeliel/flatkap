// SPDX-FileCopyrightText: 2022 Noeliel
//
// SPDX-License-Identifier: LGPL-2.0-only

use crate::Result;
use std::{ffi::CString, path::PathBuf, ptr::null_mut};

const PROC_DIR_PREFIX: &str = "/proc";
const PROC_FILE_EXE: &str = "exe";

pub fn process_wait_blocking(pid: i32) {
    let rqtp: libc::timespec = libc::timespec {
        tv_sec: 1,
        tv_nsec: 0,
    };

    unsafe {
        loop {
            if libc::kill(pid, 0) != 0 {
                break;
            }
            libc::clock_nanosleep(libc::CLOCK_REALTIME, 0, &rqtp, null_mut());
        }
    }
}

pub fn process_send_signal(pid: i32, signal: i32) -> Result<()> {
    let mut procfs_path = PathBuf::from(PROC_DIR_PREFIX);
    procfs_path.push(pid.to_string());

    let procfs_path_cstr = CString::new(procfs_path.to_str().ok_or("Invalid procfs path.")?)
        .map_err(|_| "Failed to convert procfs path to CString.")?;

    unsafe {
        let pidfd = libc::open(procfs_path_cstr.as_ptr(), libc::O_RDONLY);
        libc::syscall(
            libc::SYS_pidfd_send_signal,
            pidfd,
            libc::c_int::from(signal),
            null_mut::<libc::c_int>(),
            libc::c_int::from(0),
        );
    }

    Ok(())
}

pub fn find_named_process_pids(name: &str) -> Result<Vec<i32>> {
    let proc_dir = PathBuf::from(PROC_DIR_PREFIX);
    let all_procdirs = proc_dir.read_dir()?;

    let matching_processes = all_procdirs
        .filter_map(|f| {
            let mut file = f.as_ref().ok()?.path();
            file.push(PROC_FILE_EXE);

            let exe_path = file.canonicalize().ok()?;
            let file_name = exe_path.file_name()?;

            if file_name == name {
                return f.ok()?.file_name().to_str()?.parse::<i32>().ok();
            }

            None
        })
        .collect();

    Ok(matching_processes)
}

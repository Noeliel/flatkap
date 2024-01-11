// SPDX-FileCopyrightText: 2022 Noeliel
//
// SPDX-License-Identifier: LGPL-2.0-only

use std::ptr::null_mut;

pub fn wait_for_pid_blocking(pid: i32) {
    loop {
        if unsafe { libc::kill(pid, 0) } != 0 {
            return;
        }

        let rqtp: libc::timespec = libc::timespec {
            tv_sec: 1,
            tv_nsec: 0,
        };
        unsafe {
            libc::clock_nanosleep(libc::CLOCK_REALTIME, 0, &rqtp, null_mut());
        }
    }
}

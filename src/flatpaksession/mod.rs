// SPDX-FileCopyrightText: 2022 Noeliel
//
// SPDX-License-Identifier: LGPL-2.0-only

mod fs;

use libc::{self, getuid};
use std::{
    collections::LinkedList,
    env,
    ffi::OsStr,
    io::Error,
    path::PathBuf,
    process::{self, Command},
};

const FLATKAP_TMP_DIR: &str = ".flatkap";

const FLATPAK_CMD: &str = "flatpak";
const SESSION_HELPER_NAME: &str = "flatpak-session-helper";
const PORTAL_NAME: &str = "flatpak-portal";

pub struct FlatpakSession {
    tmp_dir: PathBuf,
    pid: String,
}

impl FlatpakSession {
    pub fn launch() {
        // when this is dropped, the flatpak processes might be killed
        let _session = FlatpakSession::new();

        let mut args: LinkedList<_> = env::args().collect();
        args.pop_front();

        FlatpakSession::launch_and_join_flatpak(args);
    }

    fn new() -> Result<Self, Error> {
        let pid = process::id().to_string();
        let mut tmp_dir = env::temp_dir();
        unsafe {
            tmp_dir.push(FLATKAP_TMP_DIR.to_string() + "-" + &getuid().to_string());
        }

        fs::touch_file_in_dir(&pid, &tmp_dir)?;

        Ok(FlatpakSession { tmp_dir, pid })
    }

    fn launch_and_join_flatpak<I, S>(args: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let handle = Command::new(FLATPAK_CMD).args(args).spawn();

        if let Ok(mut thread) = handle {
            let _res = thread.wait();
        }
    }

    fn kill_lingering_processes_if_necessary(&self) {
        // println!("Killing lingering processes...");
        if let Ok(count) = fs::count_files_in_dir(&self.tmp_dir) {
            if 1 == count {
                let _res = Command::new("killall").args([SESSION_HELPER_NAME]).spawn();
                let _res = Command::new("killall").args([PORTAL_NAME]).spawn();
            }
        }
    }
}

impl Drop for FlatpakSession {
    fn drop(&mut self) {
        self.kill_lingering_processes_if_necessary();
        fs::remove_file_in_dir(&self.pid, &self.tmp_dir);
    }
}

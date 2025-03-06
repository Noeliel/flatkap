// SPDX-FileCopyrightText: 2022 Noeliel
//
// SPDX-License-Identifier: LGPL-2.0-only

use crate::{
    error::Result,
    fs,
    util::{find_named_process_pids, process_send_signal, process_wait_blocking},
};
use serde_json::{self, Value};
use std::{
    collections::LinkedList,
    env,
    ffi::OsStr,
    path::PathBuf,
    process::{Child, Command},
};

const TMP_DIR_FLATKAP: &str = ".flatkap";

const RUN_DIR_PREFIX: &str = "/run/user";
const RUN_DIR_SUFFIX: &str = ".flatpak";
const RUN_FILE_PID: &str = "pid";
const RUN_FILE_INFO: &str = "bwrapinfo.json";
const RUN_FILE_INFO_FIELD_PID: &str = "child-pid";

const CMD_FLATPAK: &str = "flatpak";

const PROC_SESSION_HELPER: &str = "flatpak-session-helper";
const PROC_PORTAL: &str = "flatpak-portal";

pub struct FlatpakSession {
    uid: String,
    tmp_dir: PathBuf,
    child_proc: Child,
    child_pid: String,
}

impl FlatpakSession {
    pub fn run() -> Result<()> {
        let mut args: LinkedList<_> = env::args().collect();
        args.pop_front();

        let mut session = FlatpakSession::new(args)?;
        session.wait_for_child()?;
        session.kill_lingering_processes_if_necessary()?;

        Ok(())
    }

    fn new<I, S>(args: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let uid = unsafe { libc::getuid() }.to_string();
        let mut tmp_dir = env::temp_dir();
        tmp_dir.push(TMP_DIR_FLATKAP.to_string() + "-" + &uid);

        if let Ok(child_proc) = FlatpakSession::launch_child_proc(args) {
            let child_pid = child_proc.id().to_string();
            let session = FlatpakSession {
                uid,
                tmp_dir,
                child_proc,
                child_pid,
            };

            fs::touch_file_in_dir(&session.child_pid, &session.tmp_dir)?;

            return Ok(session);
        }

        Err("Failed to create session. Your flatpak application didn't launch successfully!".into())
    }

    fn wait_for_child(&mut self) -> Result<()> {
        self.child_proc.wait()?;
        self.wait_for_app_bwrap()?;
        Ok(())
    }

    fn kill_lingering_processes_if_necessary(&self) -> Result<()> {
        let dir_contents = self.tmp_dir.read_dir()?;

        if 1 == dir_contents.count() {
            for proc in [PROC_SESSION_HELPER, PROC_PORTAL] {
                for pid in find_named_process_pids(proc).unwrap_or_default() {
                    _ = process_send_signal(pid, libc::SIGTERM);
                }
            }
        }

        Ok(())
    }

    fn launch_child_proc<I, S>(args: I) -> Result<Child>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Ok(Command::new(CMD_FLATPAK).args(args).spawn()?)
    }

    fn wait_for_app_bwrap(&mut self) -> Result<()> {
        let bwrap_pid = self.try_find_bwrap_pid()?;
        process_wait_blocking(bwrap_pid);
        Ok(())
    }

    fn try_find_bwrap_pid(&self) -> Result<i32> {
        let mut run_dir = PathBuf::from(RUN_DIR_PREFIX);
        run_dir.push(&self.uid);
        run_dir.push(RUN_DIR_SUFFIX);

        // locate all potential bwrap directories...
        let mut contents = run_dir.read_dir()?;

        // try to find the one that holds a "pid" file which contains our target pid as text...
        let bwrap_dir = contents
            .find_map(|f| {
                let file = f.as_ref().ok()?;
                let file_type = file.file_type().ok()?; // if file type can be determined

                if file_type.is_dir() {
                    let file_pid =
                        fs::read_file_in_dir(RUN_FILE_PID, file.path().as_path()).ok()?; // if pid file exists and we got content

                    if file_pid == self.child_pid {
                        // we found our path
                        return Some(f);
                    }
                }

                None
            })
            .ok_or("No pidfs dir found for pid.")??;

        let bwrap_dir = bwrap_dir.path();
        let bwrap_info_raw = fs::read_file_in_dir(RUN_FILE_INFO, bwrap_dir.as_path())?;
        let bwrap_info: Value =
            serde_json::from_str(&bwrap_info_raw).map_err(|_| "Failed to parse bwrapinfo.json.")?;
        let pid = bwrap_info
            .get(RUN_FILE_INFO_FIELD_PID)
            .ok_or("Failed to find pid.")?;

        Ok(pid.as_u64().ok_or("Failed to convert pid to integer.")? as i32)
    }
}

impl Drop for FlatpakSession {
    fn drop(&mut self) {
        let _ = fs::remove_file_in_dir(&self.child_pid, &self.tmp_dir);
    }
}

// SPDX-FileCopyrightText: 2022 Noeliel
//
// SPDX-License-Identifier: LGPL-2.0-only

mod fs;
mod libc_safe;

use serde_json::{self, Value};
use std::{
    collections::LinkedList,
    env,
    ffi::OsStr,
    io::{Error, ErrorKind},
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
const CMD_KILLALL: &str = "killall";
const CMD_TAIL: &str = "tail";

const PROC_SESSION_HELPER: &str = "flatpak-session-helper";
const PROC_PORTAL: &str = "flatpak-portal";

pub struct FlatpakSession {
    uid: String,
    tmp_dir: PathBuf,
    child_proc: Child,
    child_pid: String,
}

impl FlatpakSession {
    pub fn run() {
        let mut args: LinkedList<_> = env::args().collect();
        args.pop_front();

        match FlatpakSession::new(args) {
            Ok(mut session) => {
                // child is running ...

                if session.wait_for_child() {
                    session.kill_lingering_processes_if_necessary();
                } else {
                    println!("The flatpak session behaved unexpectedly. Flatkap won't attempt to kill lingering processes.");
                }

                session.quit();
            }
            Err(e) => println!("{}", e),
        }
    }

    fn new<I, S>(args: I) -> Result<Self, &'static str>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let uid = libc_safe::getuid().to_string();
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

            if fs::touch_file_in_dir(&session.child_pid, &session.tmp_dir).is_err() {
                return Err("Failed to create session. Couldn't set up lockfile.");
            }

            return Ok(session);
        }

        Err("Failed to create session. Your flatpak application didn't launch successfully!")
    }

    fn wait_for_child(&mut self) -> bool {
        self.child_proc.wait().is_ok() && self.wait_for_app_bwrap()
    }

    fn kill_lingering_processes_if_necessary(&self) {
        if let Ok(dir_contents) = self.tmp_dir.read_dir() {
            if 1 == dir_contents.count() {
                let _result = Command::new(CMD_KILLALL)
                    .args([PROC_SESSION_HELPER])
                    .output();
                let _result = Command::new(CMD_KILLALL).args([PROC_PORTAL]).output();
            }
        }
    }

    fn quit(&mut self) {
        let _result = fs::remove_file_in_dir(&self.child_pid, &self.tmp_dir);
    }

    fn launch_child_proc<I, S>(args: I) -> Result<Child, Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Command::new(CMD_FLATPAK).args(args).spawn()
    }

    fn wait_for_app_bwrap(&mut self) -> bool {
        if let Ok(bwrap_pid) = self.try_find_bwrap_pid() {
            // we have a pid; launch `tail --pid=<pid> -f /dev/null` to wait for the process to quit

            let tail_arg_pid = format!("--pid={}", bwrap_pid);
            let tail_arg_follow = "-f";
            let tail_arg_devnull = "/dev/null";

            return Command::new(CMD_TAIL)
                .args([
                    tail_arg_pid,
                    tail_arg_follow.to_string(),
                    tail_arg_devnull.to_string(),
                ])
                .output()
                .is_ok();
        }

        false
    }

    fn try_find_bwrap_pid(&self) -> Result<u64, Error> {
        let mut run_dir = PathBuf::from(RUN_DIR_PREFIX);
        run_dir.push(&self.uid);
        run_dir.push(RUN_DIR_SUFFIX);

        // locate all potential bwrap directories...
        let contents = run_dir.read_dir()?;

        // try to find the one that holds a "pid" file which contains our target pid as text...
        let bwrap_dir = contents
            .into_iter()
            .find(|file| {
                || -> Result<bool, Error> {
                    let file = file.as_ref().map_err(|_| {
                        Error::new(ErrorKind::NotFound, "Failed to access directory entry.")
                    })?;

                    let file_type = file.file_type()?; // if file type can be determined

                    if file_type.is_dir() {
                        let file_pid = fs::read_file_in_dir(RUN_FILE_PID, file.path().as_path())?; // if pid file exists and we got content

                        if file_pid == self.child_pid {
                            // we found our path
                            return Ok(true);
                        }
                    }

                    Ok(false)
                }()
                .unwrap_or(false) // return false on any error from the closure
            })
            .unwrap_or_else(|| {
                Err(Error::new(
                    ErrorKind::NotFound,
                    "Failed to find bwrap directory for app.",
                ))
            })?;

        let bwrap_dir = bwrap_dir.path();
        let bwrap_info_raw = fs::read_file_in_dir(RUN_FILE_INFO, bwrap_dir.as_path())?;
        let bwrap_info: Value = serde_json::from_str(&bwrap_info_raw)?;
        let pid = bwrap_info
            .get(RUN_FILE_INFO_FIELD_PID)
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Failed to read app bwrap pid."))?;

        pid.as_u64().ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidData,
                "Failed to convert pid into an integer.",
            )
        })
    }
}

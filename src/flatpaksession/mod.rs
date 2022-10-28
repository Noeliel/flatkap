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
    io::Error,
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

        if let Ok(child) = FlatpakSession::launch_child_proc(args) {
            let pid = child.id().to_string();
            let session = FlatpakSession {
                uid,
                tmp_dir,
                child_proc: child,
                child_pid: pid,
            };

            if fs::touch_file_in_dir(&session.child_pid, &session.tmp_dir).is_err() {
                return Err("Failed to create session. Couldn't setup lockfile.");
            }

            return Ok(session);
        }

        Err("Failed to create session. Your flatpak application didn't launch successfully!")
    }

    fn launch_child_proc<I, S>(args: I) -> Result<Child, Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Command::new(CMD_FLATPAK).args(args).spawn()
    }

    fn wait_for_child(&mut self) -> bool {
        self.child_proc.wait().is_ok() && self.try_locate_and_wait_for_app_bwrap()
    }

    // TODO: clean this up
    fn try_locate_and_wait_for_app_bwrap(&mut self) -> bool {
        let mut run_dir = PathBuf::from(RUN_DIR_PREFIX);
        run_dir.push(&self.uid);
        run_dir.push(RUN_DIR_SUFFIX);

        // locate all potential bwrap directories... (this is a bit messy)
        if let Ok(contents) = run_dir.read_dir() {
            // try to find the one that holds a "pid" file which contains our target pid as text...
            let bwrap_dir = contents.into_iter().find(|file| {
                if let Ok(file) = file {
                    // file exists

                    if let Ok(file_type) = file.file_type() {
                        // file type can be determined

                        if file_type.is_dir() {
                            // file type is dir

                            if let Ok(file_pid) =
                                fs::read_file_in_dir(RUN_FILE_PID, file.path().as_path())
                            {
                                // pid file exists and we got content

                                if file_pid == self.child_pid {
                                    // we found our path
                                    return true;
                                }
                            }
                        }
                    }
                }

                false
            });

            // if we found our path, try to parse the bwrap pid and wait for it to quit
            if let Some(Ok(bwrap_dir)) = bwrap_dir {
                let bwrap_dir = bwrap_dir.path();

                if let Ok(bwrap_info_raw) = fs::read_file_in_dir(RUN_FILE_INFO, bwrap_dir.as_path())
                {
                    let bwrap_info: Result<Value, _> = serde_json::from_str(&bwrap_info_raw);

                    if let Ok(info) = bwrap_info {
                        if let Some(pid) = info.get(RUN_FILE_INFO_FIELD_PID) {
                            if let Some(pid) = pid.as_u64() {
                                // we have a pid; launch `tail --pid=<pid> -f /dev/null` to wait for the process to quit

                                let tail_arg_pid = format!("--pid={}", pid);
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
                        }
                    }
                }
            }
        }

        false
    }

    fn quit(&mut self) {
        let _result = fs::remove_file_in_dir(&self.child_pid, &self.tmp_dir);
    }

    fn kill_lingering_processes_if_necessary(&self) {
        if let Ok(dir_contents) = self.tmp_dir.read_dir() {
            if 1 == dir_contents.count() {
                let _result = Command::new(CMD_KILLALL)
                    .args([PROC_SESSION_HELPER])
                    .spawn();
                let _result = Command::new(CMD_KILLALL).args([PROC_PORTAL]).spawn();
            }
        }
    }
}

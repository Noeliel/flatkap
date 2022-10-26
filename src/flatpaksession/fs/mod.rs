// SPDX-FileCopyrightText: 2022 Noeliel
//
// SPDX-License-Identifier: LGPL-2.0-only

use std::{
    fs,
    io::Error,
    path::{Path, PathBuf},
};

pub fn count_files_in_dir(dir: &Path) -> Result<usize, Error> {
    let dir_contents = dir.read_dir()?;
    Ok(dir_contents.count())
}

pub fn touch_file_in_dir(file_name: &str, dir: &PathBuf) -> Result<(), Error> {
    if !dir.exists() {
        fs::create_dir_all(dir)?;
    }

    let mut file_path = dir.clone();
    file_path.push(file_name);

    fs::write(file_path, [])?;

    Ok(())
}

pub fn remove_file_in_dir(file_name: &str, dir: &Path) {
    let mut file_path = dir.to_path_buf();
    file_path.push(file_name);

    let _res = fs::remove_file(file_path);
}

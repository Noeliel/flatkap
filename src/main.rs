// SPDX-FileCopyrightText: 2022 Noeliel
//
// SPDX-License-Identifier: LGPL-2.0-only

mod error;
mod flatpaksession;
mod fs;
mod util;

use crate::error::Result;
use flatpaksession::FlatpakSession;

fn main() {
    if let Err(e) = FlatpakSession::run() {
        match e {
            error::Error::Generic { message } => eprintln!("Error: {message}"),
            error::Error::IOError { message } => eprintln!("IOError: {message}"),
        }
    }
}

// SPDX-FileCopyrightText: 2022 Noeliel
//
// SPDX-License-Identifier: LGPL-2.0-only

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Generic { message: String },
    IOError { message: String },
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::IOError {
            message: value.to_string(),
        }
    }
}

impl From<&'static str> for Error {
    fn from(value: &'static str) -> Self {
        Error::Generic {
            message: value.to_owned(),
        }
    }
}

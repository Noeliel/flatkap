// SPDX-FileCopyrightText: 2022 Noeliel
//
// SPDX-License-Identifier: LGPL-2.0-only

mod flatpaksession;

use flatpaksession::FlatpakSession;

fn main() {
    FlatpakSession::run();
}

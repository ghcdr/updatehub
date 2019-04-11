// Copyright (C) 2019 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

use std::{
    io,
    path::{Path, PathBuf},
};

pub fn device_path_from_name(_mtdname: &str) -> Result<PathBuf, failure::Error> {
    unimplemented!("FIXME: Find device path from MTD name")
}

pub fn is_nand(_dev: &Path) -> io::Result<bool> {
    unimplemented!("FIXME: Check if MTD device is NAND");
}

// Copyright (C) 2019 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

use failure::bail;
use std::{
    fs,
    io::{self, BufRead},
    path::{Path, PathBuf},
};

pub fn device_path_from_name(mtdname: &str) -> Result<PathBuf, failure::Error> {
    // let proc = io::BufReader::new(fs::File::open("/proc/mtd")?);
    let proc = io::BufReader::new(
        r#"dev:  size    erasesize name
mtd0: 000a0000 00020000 "misc"
mtd1: 00420000 00020000 "recovery"
mtd2: 002c0000 00020000 "boot"
mtd3: 0fa00000 00020000 "system"
mtd4: 02800000 00020000 "test"
mtd5: 0af20000 00020000 "userdata""#
            .as_bytes(),
    );

    for line in proc.lines() {
        let line = line?;
        let words = str::split_whitespace(&line).collect::<Vec<_>>();
        if words.contains(&mtdname) {
            return Ok(PathBuf::from(format!(
                "/dev/{}",
                words[0].trim_end_matches(':')
            )));
        }
    }
    bail!(format!(
        "Couldn't find a flash device corresponding to the mtdname '{}'",
        mtdname
    ));
}

pub fn is_nand(_dev: &Path) -> io::Result<bool> {
    unimplemented!("FIXME: Check if MTD device is NAND");
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn path_from_name() {
        let path = device_path_from_name(&r#""test""#).unwrap();
        assert_eq!(path.to_str(), PathBuf::from("/dev/mtd4").to_str());
    }
}

// Copyright (C) 2019 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

use failure::ResultExt;
use which;

pub fn utilities_exists(utils: &[&str]) -> Result<(), failure::Error> {
    for u in utils {
        which::which(u).context(format!("Looking for {} utility", u))?;
    }

    Ok(())
}

// Copyright (C) 2019 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

use super::{definitions, ObjectInstaller, ObjectType};
use crate::utils;
use easy_process;
use failure::bail;
use serde::Deserialize;
use slog::slog_info;
use slog_scope::info;
use std::path::PathBuf;

#[derive(Deserialize, PartialEq, Debug)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Flash {
    filename: String,
    size: u64,
    sha256sum: String,
    #[serde(flatten)]
    target: definitions::TargetType,

    install_if_different: Option<definitions::InstallIfDifferent>,
}

impl_object_type!(Flash);

impl ObjectInstaller for Flash {
    fn check_requirements(&self) -> Result<(), failure::Error> {
        info!("'flash' handle checking requirements");

        utils::misc::utilities_exists(&["nandwrite", "flashcp", "flash_erase"])?;

        match &self.target {
            definitions::TargetType::Device(_) | definitions::TargetType::MTDName(_) => {
                self.target.valid()?
            }
            _ => bail!(format!("{:#?} device is not supported", self.target)),
        };

        Ok(())
    }

    fn install(&self, download_dir: PathBuf) -> Result<(), failure::Error> {
        info!("'flash' handle install");

        let source = download_dir.join(&self.sha256sum);
        let target = match &self.target {
            definitions::TargetType::Device(path) => path.clone(),
            definitions::TargetType::MTDName(name) => utils::mtd::device_path_from_name(name)?,
            _ => unreachable!("Device should be secured by check_requirements"),
        };

        easy_process::run(&format!("flash_erase {:?} 0 0", &target))?;

        if utils::mtd::is_nand(&target)? {
            easy_process::run(&format!("nandwrite -p {:?} {:?}", &target, &source))?;
        } else {
            easy_process::run(&format!("flashcp {:?} {:?}", &source, &target))?;
        }

        Ok(())
    }
}

#[test]
fn deserialize() {
    use pretty_assertions::assert_eq;
    use serde_json::json;

    assert_eq!(
        Flash {
            filename: "etc/passwd".to_string(),
            size: 1024,
            sha256sum: "cfe2be1c64b0387500853de0f48303e3de7b1c6f1508dc719eeafa0d41c36722"
                .to_string(),
            target: definitions::TargetType::Device(std::path::PathBuf::from("/dev/sda")),

            install_if_different: None,
        },
        serde_json::from_value::<Flash>(json!({
            "filename": "etc/passwd",
            "size": 1024,
            "sha256sum": "cfe2be1c64b0387500853de0f48303e3de7b1c6f1508dc719eeafa0d41c36722",
            "target-type": "device",
            "target": "/dev/sda",
        }))
        .unwrap()
    );
}

// Copyright (C) 2019 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

use super::{definitions, ObjectInstaller, ObjectType};
use crate::utils;
use failure::bail;
use serde::Deserialize;
use slog::slog_info;
use slog_scope::info;
use std::{fs, io, os::unix::fs::PermissionsExt, path::PathBuf};
use tempfile;

#[derive(Deserialize, PartialEq, Debug)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Copy {
    filename: String,
    filesystem: definitions::Filesystem,
    size: u64,
    sha256sum: String,
    #[serde(flatten)]
    target_type: definitions::TargetType,
    target_path: String,

    install_if_different: Option<definitions::InstallIfDifferent>,
    #[serde(flatten)]
    target_permissions: definitions::TargetPermissions,
    #[serde(default)]
    compressed: bool,
    #[serde(default)]
    required_uncompressed_size: u64,
    #[serde(flatten, default)]
    target_format: definitions::TargetFormat,
    #[serde(default)]
    mount_options: String,
}

impl_object_type!(Copy);

impl ObjectInstaller for Copy {
    fn check_requirements(&self) -> Result<(), failure::Error> {
        info!("'copy' handle checking requirements");
        if let definitions::TargetType::Device(_) = self.target_type.valid()? {
            return Ok(());
        }

        bail!("Unexpected target type, expected some device.")
    }

    fn install(&self, download_dir: PathBuf) -> Result<(), failure::Error> {
        info!("'copy' handler Install");

        let device: &PathBuf = match self.target_type {
            definitions::TargetType::Device(ref p) => p,
            _ => unreachable!("Device should be secured by check_requirements"),
        };
        let workdir = tempfile::tempdir()?;
        let workdir = workdir.path();
        let fs = self.filesystem;
        let mount_options = &self.mount_options;
        let format_options = &self.target_format.format_options;
        let chunk_size = definitions::ChunkSize::default().0;

        let dest = workdir.join(&self.target_path);
        let source = download_dir.join(self.sha256sum());

        if self.target_format.format {
            utils::fs::format(device, fs, &format_options)?;
        }

        utils::fs::mount(device, &workdir, fs, mount_options)?;

        let mut input = io::BufReader::with_capacity(chunk_size, fs::File::open(source)?);
        let mut output = io::BufWriter::with_capacity(
            chunk_size,
            fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&dest)?,
        );

        let orig_mode = dest.metadata()?.permissions().mode();
        dest.metadata()?.permissions().set_mode(0o666);
        io::copy(&mut input, &mut output)?;
        dest.metadata()?.permissions().set_mode(orig_mode);

        self.target_permissions
            .target_mode
            .map(|mode| utils::fs::chmod(&dest, mode));

        utils::fs::chown(
            &dest,
            &self.target_permissions.target_uid,
            &self.target_permissions.target_gid,
        )?;

        utils::fs::umount(&workdir)?;

        Ok(())
    }
}

#[test]
fn deserialize() {
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use std::path::PathBuf;

    assert_eq!(
        Copy {
            filename: "etc/passwd".to_string(),
            filesystem: definitions::Filesystem::Btrfs,
            size: 1024,
            sha256sum: "cfe2be1c64b0387500853de0f48303e3de7b1c6f1508dc719eeafa0d41c36722"
                .to_string(),
            target_type: definitions::TargetType::Device(PathBuf::from("/dev/sda")),
            target_path: "/etc/passwd".to_string(),
            install_if_different: Some(definitions::InstallIfDifferent::CheckSum(
                definitions::install_if_different::CheckSum::Sha256Sum
            )),
            target_permissions: definitions::TargetPermissions::default(),
            compressed: false,
            required_uncompressed_size: 0,
            target_format: definitions::TargetFormat::default(),
            mount_options: String::default(),
        },
        serde_json::from_value::<Copy>(json!({
            "filename": "etc/passwd",
            "size": 1024,
            "sha256sum": "cfe2be1c64b0387500853de0f48303e3de7b1c6f1508dc719eeafa0d41c36722",
            "install-if-different": "sha256sum",
            "filesystem": "btrfs",
            "target-type": "device",
            "target": "/dev/sda",
            "target-path": "/etc/passwd"
        }))
        .unwrap()
    );
}

// FIXME: missing tests:
// - copy over existing file
// - copy a new file
// - change uid / gid
// - change mode

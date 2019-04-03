// Copyright (C) 2019 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

use super::{definitions, ObjectInstaller, ObjectType};
use crate::utils;
use failure::bail;
use serde::Deserialize;
use slog::slog_info;
use slog_scope::info;
use std::{
    fs,
    io::{self, Write},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
};
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
        let guard_workdir = tempfile::tempdir()?;
        let workdir = guard_workdir.path();
        let fs = self.filesystem;
        let mount_options = &self.mount_options;
        let format_options = &self.target_format.format_options;
        let chunk_size = definitions::ChunkSize::default().0;

        let dest = workdir.join(&self.target_path);
        let source = download_dir.join(self.sha256sum());

        if self.target_format.format {
            utils::fs::format(device, fs, &format_options)?;
        }

        let _guard_mount = utils::fs::mount(device, &workdir, fs, mount_options)?;

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

        let metadata = dest.metadata()?;
        let orig_mode = metadata.permissions().mode();
        metadata.permissions().set_mode(0o100_666);
        io::copy(&mut input, &mut output)?;
        output.flush()?;
        metadata.permissions().set_mode(orig_mode);

        if let Some(mode) = self.target_permissions.target_mode {
            utils::fs::chmod(&dest, mode)?;
        }

        utils::fs::chown(
            &dest,
            &self.target_permissions.target_uid,
            &self.target_permissions.target_gid,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loopdev::LoopControl;
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use std::{
        io::{BufRead, Seek, SeekFrom, Write},
        iter,
        os::unix::fs::MetadataExt,
        path::PathBuf,
    };
    use tempfile::{tempdir, NamedTempFile};

    const DEFAULT_BYTE: u8 = 0xF;
    const ORIGINAL_BYTE: u8 = 0xA;
    const FILE_SIZE: usize = 2048;

    fn fake_copy_object<F>(
        mut f: F,
        original_permissions: Option<definitions::TargetPermissions>,
    ) -> Result<(), failure::Error>
    where
        F: FnMut(&mut Copy),
    {
        // Setup device
        let dev = LoopControl::open()?.next_free()?;
        let mut image = NamedTempFile::new()?;
        // FIXME: use seek for image file
        image.write_all(
            &iter::repeat(0)
                .take(1024 * 1024 + FILE_SIZE)
                .collect::<Vec<_>>(),
        )?;
        dev.attach_file(image.path())?;
        let mount_dir = tempdir()?;
        utils::fs::format(&dev.path().unwrap(), definitions::Filesystem::Ext4, &None)?;

        // Setup source file
        let download_dir = tempdir()?;
        let mut source = NamedTempFile::new_in(download_dir.path())?;
        source.write_all(
            &iter::repeat(DEFAULT_BYTE)
                .take(FILE_SIZE)
                .collect::<Vec<_>>(),
        )?;
        source.seek(SeekFrom::Start(0))?;

        // Setup some original file in the device
        if let Some(perm) = original_permissions {
            let _mount_guard = utils::fs::mount(
                &dev.path().unwrap(),
                &mount_dir.path(),
                definitions::Filesystem::Ext4,
                &"",
            )?;
            let mut ofile_path = mount_dir.path().to_path_buf();
            ofile_path.push(&"original_file");
            let mut f = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&ofile_path)?;
            f.write_all(
                &iter::repeat(ORIGINAL_BYTE)
                    .take(FILE_SIZE)
                    .collect::<Vec<_>>(),
            )?;
            f.flush()?;

            if let Some(mode) = perm.target_mode {
                utils::fs::chmod(&ofile_path, mode)?;
            }

            utils::fs::chown(&ofile_path, &perm.target_uid, &perm.target_gid)?;
        }

        let mut obj = Copy {
            filename: "".to_string(),
            filesystem: definitions::Filesystem::Ext4,
            size: FILE_SIZE as u64,
            sha256sum: source.path().to_string_lossy().to_string(),
            target_type: definitions::TargetType::Device(dev.path().unwrap()),
            target_path: "original_file".to_string(),

            install_if_different: None,
            target_permissions: definitions::TargetPermissions::default(),
            compressed: false,
            required_uncompressed_size: 0,
            target_format: definitions::TargetFormat::default(),
            mount_options: String::default(),
        };
        f(&mut obj);

        obj.check_requirements()?;
        obj.setup()?;
        obj.install(download_dir.path().to_path_buf())?;

        // Validade File
        let guard_workdir = tempfile::tempdir()?;
        let workdir = guard_workdir.path();
        let chunk_size = definitions::ChunkSize::default().0;
        let _guard_mount = utils::fs::mount(
            &dev.path().unwrap(),
            &workdir,
            obj.filesystem,
            &String::default(),
        )?;
        let dest = workdir.join(&obj.target_path);
        let source = download_dir.path().to_path_buf().join(obj.sha256sum);
        let mut f1_reader = io::BufReader::with_capacity(chunk_size, fs::File::open(&source)?);
        let mut f2_reader = io::BufReader::with_capacity(chunk_size, fs::File::open(&dest)?);
        loop {
            let buf1 = f1_reader.fill_buf()?;
            let len1 = buf1.len();
            let buf2 = f2_reader.fill_buf()?;
            let len2 = buf2.len();
            // Stop comparing when both the files reach EOF
            if len1 == 0 && len2 == 0 {
                break;
            }
            assert_eq!(buf1, buf2);
            f1_reader.consume(len1);
            f2_reader.consume(len2);
        }
        let metadata = dest.metadata()?;
        if let Some(mode) = obj.target_permissions.target_mode {
            assert_eq!(mode, metadata.mode() % 0o1000);
        }
        if let Some(uid) = obj.target_permissions.target_uid {
            let uid = uid.as_uid_t();
            assert_eq!(uid, metadata.uid() % 0o1000);
        }
        if let Some(gid) = obj.target_permissions.target_gid {
            let gid = gid.as_gid_t();
            assert_eq!(gid, metadata.gid() % 0o1000);
        }

        dev.detach()?;
        Ok(())
    }

    #[test]
    #[ignore]
    fn copy_over_formated_partion() {
        fake_copy_object(|obj| obj.target_format.format = true, None).unwrap();
    }

    #[test]
    #[ignore]
    fn copy_over_existing_file() {
        fake_copy_object(
            |_| (),
            Some(definitions::TargetPermissions {
                target_mode: Some(0o666),
                target_gid: Some(definitions::target_permissions::Id::Number(1000)),
                target_uid: Some(definitions::target_permissions::Id::Number(1000)),
            }),
        )
        .unwrap();
    }

    #[test]
    #[ignore]
    fn copy_change_uid() {
        fake_copy_object(
            |obj| {
                obj.target_permissions.target_uid =
                    Some(definitions::target_permissions::Id::Number(0))
            },
            None,
        )
        .unwrap();
    }

    #[test]
    #[ignore]
    fn copy_change_gid() {
        fake_copy_object(
            |obj| {
                obj.target_permissions.target_gid =
                    Some(definitions::target_permissions::Id::Number(0))
            },
            Some(definitions::TargetPermissions {
                target_mode: Some(0o666),
                target_gid: Some(definitions::target_permissions::Id::Number(1000)),
                target_uid: Some(definitions::target_permissions::Id::Number(1000)),
            }),
        )
        .unwrap();
    }

    #[test]
    #[ignore]
    fn copy_change_mode() {
        fake_copy_object(
            |obj| obj.target_permissions.target_mode = Some(0o444),
            Some(definitions::TargetPermissions {
                target_mode: Some(0o666),
                target_gid: Some(definitions::target_permissions::Id::Number(1000)),
                target_uid: Some(definitions::target_permissions::Id::Number(1000)),
            }),
        )
        .unwrap();
    }

    #[test]
    fn deserialize() {
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
}

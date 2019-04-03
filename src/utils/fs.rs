// Copyright (C) 2019 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

use crate::update_package::object::definitions::{target_permissions::Id, Filesystem};
use libc;
use std::{
    io,
    os::unix::{ffi::OsStrExt, fs::PermissionsExt},
    path::Path,
    process::Command,
};

pub(crate) fn format(target: &Path, fs: Filesystem, options: &Option<String>) -> io::Result<()> {
    let target = target.display();
    let options = options.clone().unwrap_or_else(|| "".to_string());

    let cmd = match fs {
        Filesystem::Jffs2 => format!("flash_erase -j {} {} 0 0", options, target),
        Filesystem::Ext2 | Filesystem::Ext3 | Filesystem::Ext4 => {
            format!("mkfs.{} -F {} {}", fs, options, target)
        }
        Filesystem::Ubifs => format!("mkfs.{} -y {} {}", fs, options, target),
        Filesystem::Xfs => format!("mkfs.{} -f {} {}", fs, options, target),
        Filesystem::Btrfs | Filesystem::Vfat | Filesystem::F2fs => {
            format!("mkfs.{} {} {}", fs, options, target)
        }
    };

    // FIXME: use easyprocess here
    let cmd: Vec<&str> = cmd.split(' ').collect();
    Command::new(cmd[0]).args(&cmd[1..]).output()?;

    Ok(())
}

pub(crate) fn mount(
    source: &Path,
    dest: &Path,
    fs: Filesystem,
    options: &str,
) -> io::Result<sys_mount::Mount> {
    sys_mount::Mount::new(
        source,
        dest,
        sys_mount::FilesystemType::Manual(&format!("{}", fs)),
        sys_mount::MountFlags::empty(),
        Some(options),
    )
}

pub(crate) fn umount(mount_path: &Path) -> io::Result<()> {
    sys_mount::unmount(mount_path, sys_mount::UnmountFlags::empty())
}

pub(crate) fn chmod(file: &Path, mode: u32) -> io::Result<()> {
    file.metadata()?.permissions().set_mode(mode);
    Ok(())
}

pub(crate) fn chown(
    file_path: &Path,
    file_uid: &Option<Id>,
    file_gid: &Option<Id>,
) -> io::Result<()> {
    let ret = unsafe {
        let uid = file_uid
            .as_ref()
            .map(|id| match id {
                Id::Name(s) => (*libc::getpwnam(s.as_str().as_ptr() as *const i8)).pw_uid,
                Id::Number(n) => *n as libc::uid_t,
            })
            .unwrap_or(-1 as i32 as libc::uid_t);
        let gid = file_gid
            .as_ref()
            .map(|id| match id {
                Id::Name(s) => (*libc::getgrnam(s.as_str().as_ptr() as *const i8)).gr_gid,
                Id::Number(n) => *n as libc::gid_t,
            })
            .unwrap_or(-1 as i32 as libc::gid_t);

        libc::lchown(
            file_path.as_os_str().as_bytes().as_ptr() as *const i8,
            uid,
            gid,
        )
    };

    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

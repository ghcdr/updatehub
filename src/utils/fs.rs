// Copyright (C) 2019 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

use crate::update_package::object::definitions::{target_permissions::Id, Filesystem};
// use easy_process;
use failure::ensure;
use nix::unistd::{Gid, Uid};
use std::{io, path::Path, process::Command};
use sys_mount::{Mount, Unmount, UnmountDrop};

pub(crate) fn format(
    target: &Path,
    fs: Filesystem,
    options: &Option<String>,
) -> Result<(), failure::Error> {
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

    // let output = easy_process::run(&cmd.as_str())?;
    let cmd: Vec<&str> = cmd.split(' ').filter(|s| !s.is_empty()).collect();
    let output = Command::new(cmd[0]).args(&cmd[1..]).output()?;
    ensure!(
        output.status.success(),
        format!("Format command failed with code: {:#?}", output)
    );
    // ensure!(
    //     output.stderr.is_empty(),
    //     format!("Format command filed: {:?}", output)
    // );

    Ok(())
}

pub(crate) fn mount(
    source: &Path,
    dest: &Path,
    fs: Filesystem,
    options: &str,
) -> io::Result<UnmountDrop<Mount>> {
    Ok(Mount::new(
        source,
        dest,
        format!("{}", fs).as_str(),
        sys_mount::MountFlags::empty(),
        Some(options),
    )?
    .into_unmount_drop(sys_mount::UnmountFlags::DETACH))
}

pub(crate) fn chmod(path: &Path, mode: u32) -> Result<(), failure::Error> {
    nix::sys::stat::fchmodat(
        None,
        path,
        nix::sys::stat::Mode::from_bits(mode).unwrap(),
        nix::sys::stat::FchmodatFlags::FollowSymlink,
    )?;
    Ok(())
}

pub(crate) fn chown(path: &Path, uid: &Option<Id>, gid: &Option<Id>) -> nix::Result<()> {
    let uid = uid.as_ref().map(|id| Uid::from_raw(id.as_uid_t()));
    let gid = gid.as_ref().map(|id| Gid::from_raw(id.as_gid_t()));

    nix::unistd::chown(path, uid, gid)
}

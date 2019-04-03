// Copyright (C) 2019 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

use serde::Deserialize;

#[derive(PartialEq, Debug, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct TargetPermissions {
    #[serde(deserialize_with(de::octal_from_str))]
    pub target_mode: Option<u32>,
    pub target_gid: Option<Id>,
    pub target_uid: Option<Id>,
}

#[derive(PartialEq, Debug, Deserialize)]
#[serde(untagged)]
pub enum Id {
    /// User or group name
    Name(String),

    /// User or group numeric id
    #[serde(deserialize_with(de::octal_from_str))]
    Number(u32),
}

impl Id {
    pub fn as_uid_t(&self) -> u32 {
        match self {
            Id::Name(s) => {
                let s = std::ffi::CString::new(s.as_str());
                unsafe { *nix::libc::getpwnam(s.unwrap().as_ptr()) }.pw_uid
            }
            Id::Number(n) => *n,
        }
    }

    pub fn as_gid_t(&self) -> u32 {
        match self {
            Id::Name(s) => {
                let s = std::ffi::CString::new(s.as_str());
                unsafe { *nix::libc::getgrnam(s.unwrap().as_ptr()) }.gr_gid
            }
            Id::Number(n) => *n,
        }
    }
}

#[test]
fn deserialize() {
    use pretty_assertions::assert_eq;
    use serde_json::json;

    assert_eq!(
        TargetPermissions {
            target_mode: Some(0o0777),
            target_gid: Some(Id::Name("wheel".to_string())),
            target_uid: Some(Id::Name("user".to_string())),
        },
        serde_json::from_value::<TargetPermissions>(json!({
            "target-mode": 0o0777,
            "target-uid": "user",
            "target-gid": "wheel"
        }))
        .unwrap()
    );

    assert_eq!(
        TargetPermissions {
            target_mode: None,
            target_gid: Some(Id::Number(1000)),
            target_uid: Some(Id::Number(1000)),
        },
        serde_json::from_value::<TargetPermissions>(json!({
            "target-uid": 1000,
            "target-gid": 1000,
        }))
        .unwrap()
    );
}

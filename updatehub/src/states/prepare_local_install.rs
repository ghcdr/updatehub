// Copyright (C) 2020 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

use super::{
    actor::{self, SharedState},
    Install, Result, State, StateChangeImpl, StateMachine, TransitionError,
};
use std::{fs, io, path::PathBuf};

#[derive(Debug, PartialEq)]
pub(super) struct PrepareLocalInstall {
    pub(super) update_file: PathBuf,
}

#[async_trait::async_trait]
impl StateChangeImpl for State<PrepareLocalInstall> {
    fn name(&self) -> &'static str {
        "prepare_local_install"
    }

    async fn handle(
        self,
        shared_state: &mut SharedState,
    ) -> Result<(StateMachine, actor::StepTransition)> {
        let dest_path = shared_state.settings.update.download_dir.clone();
        compress_tools::uncompress(self.0.update_file, &dest_path, compress_tools::Kind::Zip)
            .map_err(|_| TransitionError::Uncompress)?;

        let metadata = io::BufReader::new(fs::File::open(dest_path.join("metadata"))?);
        let update_package = serde_json::from_reader(metadata).unwrap();

        Ok((
            StateMachine::Install(State(Install { update_package })),
            actor::StepTransition::Immediate,
        ))
    }
}

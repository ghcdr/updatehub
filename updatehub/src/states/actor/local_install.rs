// Copyright (C) 2020 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

use super::{PrepareLocalInstall, State, StateMachine};
use actix::{AsyncContext, Context, Handler, Message, MessageResult};
use std::path::PathBuf;

#[derive(Message)]
#[rtype(Response)]
pub(crate) struct Request(pub(crate) PathBuf);

pub(crate) enum Response {
    RequestAccepted,
    InvalidState,
}

impl Handler<Request> for super::MachineActor {
    type Result = MessageResult<Request>;

    fn handle(&mut self, req: Request, ctx: &mut Context<Self>) -> Self::Result {
        if let Some(machine) = &self.sm.get_mut().state {
            let res = machine.for_any_state(|s| s.handle_local_install());
            return match res {
                Response::InvalidState => MessageResult(res),
                Response::RequestAccepted => {
                    let sm = self.sm.get_mut();
                    sm.stepper.restart(ctx.address());
                    sm.state.replace(StateMachine::PrepareLocalInstall(State(
                        PrepareLocalInstall { update_file: req.0 },
                    )));
                    MessageResult(res)
                }
            };
        }

        unreachable!("Failed to take StateMachine's ownership");
    }
}

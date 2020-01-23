// Copyright (C) 2019 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod test;

pub(crate) mod download_abort;
pub(crate) mod info;
pub(crate) mod probe;
/// Used to send `Step` messages to the `Machine` actor.
pub(crate) mod stepper;

use super::{Idle, Metadata, Probe, RuntimeSettings, Settings, State, StateMachine};
use actix::{Actor, Addr, Arbiter, AsyncContext, Context, Handler, Message, ResponseFuture};
use slog_scope::info;
use std::{cell::Cell, time::Duration};

// Given the limitations to asynchronously handle messages on actix 0.9,
// see: https://github.com/actix/actix/issues/308,
// we have decied to use Cell for unsafe pointer access to the state machine.
// Since the actor, by definition, won't handle multiple messages concurrently
// or poll more than one future at a time, and since whenever the systems
// polls the future the Actor is still alive, we belive this access is always
// valid.
pub(crate) struct MachineActor {
    sm: std::cell::Cell<Machine>,
}

pub(crate) struct Machine {
    state: Option<StateMachine>,
    shared_state: SharedState,
    stepper: stepper::Controller,
}

#[derive(Debug, PartialEq)]
pub(super) struct SharedState {
    pub(super) settings: Settings,
    pub(super) runtime_settings: RuntimeSettings,
    pub(super) firmware: Metadata,
}

impl SharedState {
    pub(super) fn server_address(&self) -> &str {
        self.runtime_settings
            .custom_server_address()
            .unwrap_or(&self.settings.network.server_address)
    }
}

impl Actor for MachineActor {
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        info!("Starting State Machine Actor...");
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        info!("Stopping State Machine Actor...");
    }
}

impl MachineActor {
    pub(super) fn new(
        state: StateMachine,
        settings: Settings,
        runtime_settings: RuntimeSettings,
        firmware: Metadata,
    ) -> Self {
        MachineActor {
            sm: Cell::new(Machine {
                state: Some(state),
                shared_state: SharedState { settings, runtime_settings, firmware },
                stepper: stepper::Controller::default(),
            }),
        }
    }

    pub(super) fn start(mut self) -> Addr<Self> {
        MachineActor::start_in_arbiter(&Arbiter::new(), move |ctx| {
            self.sm.get_mut().stepper.start(ctx.address());
            self
        })
    }
}

#[derive(Message)]
#[rtype(StepTransition)]
struct Step;

pub(crate) enum StepTransition {
    Delayed(Duration),
    Immediate,
    Never,
}

impl Handler<Step> for MachineActor {
    type Result = ResponseFuture<StepTransition>;

    fn handle(&mut self, _: Step, _: &mut Context<Self>) -> Self::Result {
        let sm = self.sm.as_ptr();
        Box::pin(async move {
            unsafe {
                if let Some(machine) = (*sm).state.take() {
                    let (state, transition) = machine
                        .move_to_next_state(&mut (*sm).shared_state)
                        .await
                        .unwrap_or_else(|e| (StateMachine::from(e), StepTransition::Immediate));
                    (*sm).state = Some(state);

                    transition
                } else {
                    unreachable!("Failed to take StateMachine from StateAgent")
                }
            }
        })
    }
}

//! The built-in steps, registered by [`register_builtin_steps`].

mod hosts;
mod look;
mod start;

use crate::services::OnboardingServices;
use crate::step::{DuplicateStepId, OnboardingRegistry};

pub fn register_builtin_steps(
    registry: &mut OnboardingRegistry,
    services: &OnboardingServices,
) -> Result<(), DuplicateStepId> {
    registry.register(look::LookStep::new(services))?;
    registry.register(start::StartStep::new(services))?;
    registry.register(hosts::HostsStep::new(services))?;
    Ok(())
}

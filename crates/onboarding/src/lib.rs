//! First-run onboarding: a short, skippable page of registered steps.
//!
//! One crate owns the whole capability: the [`OnboardingStep`] contract and
//! [`OnboardingRegistry`], persisted [`OnboardingProgress`], the
//! [`OnboardingView`] page and the built-in Look / Start / Hosts steps. Other
//! modules add steps by registering them; the shell only hosts the view as a
//! workspace tab. See ADR 0008.

pub mod command_provider;
pub mod host_rows;
mod progress;
mod services;
mod step;
mod steps;
mod view;

pub use progress::OnboardingProgress;
pub use services::OnboardingServices;
pub use step::{DuplicateStepId, OnboardingRegistry, OnboardingStep, StepInstance};
pub use steps::register_builtin_steps;
pub use view::{OnboardingEvent, OnboardingView};

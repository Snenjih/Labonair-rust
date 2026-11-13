//! The step contract and its registry.
//!
//! A step is one screen of the onboarding card. The owning module registers
//! steps; neither this crate's view nor the shell keeps a step list. IDs are
//! namespaced (`onboarding.look`) and a duplicate ID is an error.

use std::fmt;
use std::rc::Rc;

use gpui::{AnyView, App, SharedString, Task, Window};

/// What a step hands the card once it is built for one onboarding session.
/// Deferred work a step runs when the flow is finished.
pub type OnFinish = Box<dyn FnOnce(&mut App)>;

pub struct StepInstance {
    /// The step body (the card supplies title, hint and navigation).
    pub view: AnyView,
    /// Runs once when the user finishes the whole flow. Steps that apply their
    /// choice live (Look, Start) leave this `None`.
    pub on_finish: Option<OnFinish>,
}

pub trait OnboardingStep {
    /// Stable, namespaced identifier, e.g. `"onboarding.look"`.
    fn id(&self) -> &'static str;
    /// Sort key; steps run in ascending order, ties broken by `id`.
    fn order(&self) -> u32;
    fn title(&self) -> SharedString;
    fn hint(&self) -> SharedString;

    /// Whether the step has anything to ask right now. Must be cheap and must
    /// not block; asynchronous discovery belongs in [`Self::prepare`].
    fn is_applicable(&self, _cx: &App) -> bool {
        true
    }

    /// Asynchronous discovery that [`Self::is_applicable`] depends on. The
    /// card is only opened after every step's `prepare` has finished.
    fn prepare(&self, _cx: &mut App) -> Task<()> {
        Task::ready(())
    }

    fn build(&self, window: &mut Window, cx: &mut App) -> StepInstance;
}

#[derive(Debug, PartialEq, Eq)]
pub struct DuplicateStepId(pub &'static str);

impl fmt::Display for DuplicateStepId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "onboarding step `{}` is already registered", self.0)
    }
}

impl std::error::Error for DuplicateStepId {}

#[derive(Default)]
pub struct OnboardingRegistry {
    steps: Vec<Rc<dyn OnboardingStep>>,
}

impl OnboardingRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, step: impl OnboardingStep + 'static) -> Result<(), DuplicateStepId> {
        let id = step.id();
        if self.steps.iter().any(|existing| existing.id() == id) {
            return Err(DuplicateStepId(id));
        }
        self.steps.push(Rc::new(step));
        self.steps.sort_by_key(|step| (step.order(), step.id()));
        Ok(())
    }

    /// Every registered step, in flow order.
    pub fn steps(&self) -> &[Rc<dyn OnboardingStep>] {
        &self.steps
    }

    /// The steps that have something to ask right now, in flow order.
    pub fn applicable(&self, cx: &App) -> Vec<Rc<dyn OnboardingStep>> {
        self.steps
            .iter()
            .filter(|step| step.is_applicable(cx))
            .cloned()
            .collect()
    }

    /// The steps to show: applicable ones the user has not completed, or all
    /// applicable ones when `replay` is set (the `Onboarding: Open` command).
    pub fn pending(
        &self,
        progress: &crate::OnboardingProgress,
        replay: bool,
        cx: &App,
    ) -> Vec<Rc<dyn OnboardingStep>> {
        self.applicable(cx)
            .into_iter()
            .filter(|step| replay || !progress.is_completed(step.id()))
            .collect()
    }

    /// Runs every step's discovery. Await before asking [`Self::applicable`].
    pub fn prepare(&self, cx: &mut App) -> Task<()> {
        let tasks: Vec<Task<()>> = self.steps.iter().map(|step| step.prepare(cx)).collect();
        cx.spawn(async move |_| {
            for task in tasks {
                task.await;
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake(&'static str, u32);

    impl OnboardingStep for Fake {
        fn id(&self) -> &'static str {
            self.0
        }
        fn order(&self) -> u32 {
            self.1
        }
        fn title(&self) -> SharedString {
            SharedString::default()
        }
        fn hint(&self) -> SharedString {
            SharedString::default()
        }
        fn build(&self, _: &mut Window, _: &mut App) -> StepInstance {
            unreachable!("registry tests never build a step")
        }
    }

    #[test]
    fn rejects_a_duplicate_id() {
        let mut registry = OnboardingRegistry::new();
        registry.register(Fake("x.a", 1)).unwrap();
        assert_eq!(
            registry.register(Fake("x.a", 2)),
            Err(DuplicateStepId("x.a"))
        );
    }

    #[test]
    fn orders_by_order_then_id() {
        let mut registry = OnboardingRegistry::new();
        registry.register(Fake("x.c", 200)).unwrap();
        registry.register(Fake("x.b", 100)).unwrap();
        registry.register(Fake("x.a", 100)).unwrap();
        let ids: Vec<_> = registry.steps().iter().map(|s| s.id()).collect();
        assert_eq!(ids, ["x.a", "x.b", "x.c"]);
    }
}

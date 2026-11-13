//! What the composition root injects into the built-in steps.

use std::rc::Rc;
use std::sync::Arc;

use gpui::{App, Entity};
use labonair_settings::{enqueue_user_settings_write, SettingsContent, SettingsStore};
use labonair_ssh::SshConfigService;
use labonair_theme::store::ThemeStore;

/// Names of the hosts already in the app.
pub type ExistingHostNames = Rc<dyn Fn(&App) -> Vec<String>>;
/// Re-derives theme store and window material after a Look change.
pub type ApplyAppearance = Rc<dyn Fn(&mut App)>;

pub struct OnboardingServices {
    pub theme: Entity<ThemeStore>,
    pub tokio: tokio::runtime::Handle,
    pub ssh_config: Arc<dyn SshConfigService>,
    /// Lets the import flag hosts that already exist.
    pub existing_host_names: ExistingHostNames,
    /// Called once after a successful import so the host manager can reload.
    pub on_hosts_imported: Rc<dyn Fn(&mut App)>,
    pub apply_appearance: ApplyAppearance,
}

/// Commit one user-settings change in memory and write it off the GPUI thread,
/// the same route the Settings window uses. Failures go to the notification
/// center.
pub(crate) fn write_user_setting(cx: &mut App, change: impl FnOnce(&mut SettingsContent)) {
    if !cx.has_global::<SettingsStore>() {
        return;
    }
    let request = cx
        .global_mut::<SettingsStore>()
        .update_user_settings_deferred(change);
    let receiver = match request {
        Ok(Some(request)) => enqueue_user_settings_write(request),
        Ok(None) => return,
        Err(error) => Err(error),
    };
    match receiver {
        Ok(receiver) => cx
            .spawn(async move |cx| {
                let result = cx
                    .background_executor()
                    .spawn(async move {
                        receiver
                            .recv()
                            .unwrap_or_else(|_| Err("settings writer stopped before saving".into()))
                    })
                    .await;
                if let Err(error) = result {
                    let _ = cx.update(|cx| {
                        labonair_notifications::notify_err(
                            "Could not save setting",
                            Err::<(), _>(error),
                            cx,
                        )
                    });
                }
            })
            .detach(),
        Err(error) => {
            labonair_notifications::notify_err("Could not save setting", Err::<(), _>(error), cx);
        }
    }
}

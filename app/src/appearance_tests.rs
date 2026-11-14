use settings::Setting;
use warpui::{App, SingletonEntity};

use super::{AppearanceManager, register};
use crate::settings::FontSettings;
use crate::test_util::settings::initialize_settings_for_tests;

#[test]
fn terminal_fallback_font_loads_at_startup_and_updates_without_restart() {
    App::test((), |mut app| async move {
        initialize_settings_for_tests(&mut app);
        app.update(|app| {
            FontSettings::handle(app).update(app, |settings, ctx| {
                settings
                    .terminal_fallback_font_name
                    .set_value("Fallback".to_string(), ctx)
                    .unwrap();
            });
        });
        register(&mut app);
        app.read(|app| {
            assert!(
                AppearanceManager::as_ref(app)
                    .terminal_fallback_font_family()
                    .is_some()
            );
        });
        app.update(|app| {
            FontSettings::handle(app).update(app, |settings, ctx| {
                settings
                    .terminal_fallback_font_name
                    .clear_value(ctx)
                    .unwrap();
            });
        });
        app.read(|app| {
            assert!(
                AppearanceManager::as_ref(app)
                    .terminal_fallback_font_family()
                    .is_none()
            );
        });
        app.update(|app| {
            FontSettings::handle(app).update(app, |settings, ctx| {
                settings
                    .terminal_fallback_font_name
                    .set_value("Fallback".to_string(), ctx)
                    .unwrap();
            });
        });
        app.read(|app| {
            assert!(
                AppearanceManager::as_ref(app)
                    .terminal_fallback_font_family()
                    .is_some()
            );
        });
    });
}

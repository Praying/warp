use settings::macros::{define_settings_group, implement_setting_for_enum};
use settings::{SupportedPlatforms, SyncToCloud};

use crate::i18n::AppLanguage;

implement_setting_for_enum!(
    AppLanguage,
    LanguageSettings,
    SupportedPlatforms::ALL,
    SyncToCloud::Never,
    surface: settings::SettingSurfaces::GUI,
    private: false,
    toml_path: "appearance.language",
    description: "The language used for the application interface.",
);

define_settings_group!(LanguageSettings, settings: [
    app_language: AppLanguage,
]);

#[cfg(test)]
#[path = "language_tests.rs"]
mod tests;

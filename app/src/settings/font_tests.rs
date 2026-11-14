use settings::schema::SettingSchemaEntry;
use settings::{Setting, SettingSurfaces, SyncToCloud};
use settings_value::SettingsValue;

use super::TerminalFallbackFontName;

#[test]
fn terminal_fallback_font_is_local_gui_setting_with_automatic_default() {
    assert_eq!(TerminalFallbackFontName::default_value(), "");
    assert_eq!(
        TerminalFallbackFontName::sync_to_cloud(),
        SyncToCloud::Never
    );
    assert_eq!(
        TerminalFallbackFontName::toml_path(),
        Some("appearance.text.fallback_font_name")
    );
    let entry = inventory::iter::<SettingSchemaEntry>
        .into_iter()
        .find(|entry| {
            entry.storage_key == "fallback_font_name" && entry.hierarchy == Some("appearance.text")
        })
        .expect("fallback font schema entry");
    assert_eq!((entry.surfaces_fn)(), SettingSurfaces::GUI);
    let value = serde_json::json!("Noto Sans Mono CJK SC");
    let font = String::from_file_value(&value).unwrap();
    assert_eq!(font.to_file_value(), value);
}

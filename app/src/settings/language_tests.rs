use settings::schema::SettingSchemaEntry;
use settings::{Setting, SettingSurfaces, SyncToCloud};
use settings_value::SettingsValue;

use super::AppLanguage;

#[test]
fn app_language_is_local_gui_setting_defaulting_to_english() {
    assert_eq!(AppLanguage::default_value(), AppLanguage::English);
    assert_eq!(AppLanguage::sync_to_cloud(), SyncToCloud::Never);
    assert_eq!(AppLanguage::toml_path(), Some("appearance.language"));
    let entry = inventory::iter::<SettingSchemaEntry>
        .into_iter()
        .find(|entry| entry.storage_key == "language" && entry.hierarchy == Some("appearance"))
        .expect("app language schema entry");
    assert_eq!((entry.surfaces_fn)(), SettingSurfaces::GUI);
    let value = serde_json::json!("simplified_chinese");
    let language = AppLanguage::from_file_value(&value).unwrap();
    assert_eq!(language, AppLanguage::SimplifiedChinese);
    assert_eq!(language.to_file_value(), value);
}

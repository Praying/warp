use super::{AppLanguage, set_language, tr};

#[test]
fn tr_passes_through_in_english() {
    set_language(AppLanguage::English);
    assert_eq!(tr("Appearance"), "Appearance");
    assert_eq!(tr("not in the dictionary"), "not in the dictionary");
}

#[test]
fn tr_translates_known_strings_in_chinese() {
    set_language(AppLanguage::SimplifiedChinese);
    assert_eq!(tr("Appearance"), "外观");
    assert_eq!(tr("Settings"), "设置");
    assert_eq!(tr("New Window"), "新建窗口");
}

#[test]
fn tr_falls_back_to_english_for_unknown_strings() {
    set_language(AppLanguage::SimplifiedChinese);
    assert_eq!(
        tr("some brand new upstream string"),
        "some brand new upstream string"
    );
}

#[test]
fn tr_switches_back_to_english() {
    set_language(AppLanguage::SimplifiedChinese);
    assert_eq!(tr("Appearance"), "外观");
    set_language(AppLanguage::English);
    assert_eq!(tr("Appearance"), "Appearance");
}

#[test]
fn dropdown_labels_use_native_names() {
    assert_eq!(AppLanguage::English.as_dropdown_label(), "English");
    assert_eq!(
        AppLanguage::SimplifiedChinese.as_dropdown_label(),
        "简体中文"
    );
    assert_eq!(AppLanguage::all().len(), 2);
}

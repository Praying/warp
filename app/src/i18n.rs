use std::borrow::Cow;
use std::sync::atomic::{AtomicU8, Ordering};

use serde::{Deserialize, Serialize};
use settings::Setting as _;
use warpui::{AppContext, SingletonEntity};

use crate::settings::{LanguageSettings, LanguageSettingsChangedEvent};

mod zh_cn;

/// The language used for the application interface.
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(
    description = "The language used for the application interface.",
    rename_all = "snake_case"
)]
pub enum AppLanguage {
    #[default]
    English,
    SimplifiedChinese,
}

impl AppLanguage {
    pub fn as_dropdown_label(&self) -> &'static str {
        match self {
            Self::English => "English",
            Self::SimplifiedChinese => "简体中文",
        }
    }

    pub fn all() -> [AppLanguage; 2] {
        [Self::English, Self::SimplifiedChinese]
    }
}

static LANGUAGE: AtomicU8 = AtomicU8::new(AppLanguage::English as u8);

pub fn set_language(language: AppLanguage) {
    LANGUAGE.store(language as u8, Ordering::Release);
}

fn is_simplified_chinese() -> bool {
    LANGUAGE.load(Ordering::Acquire) == AppLanguage::SimplifiedChinese as u8
}

/// Translates a user-facing English string into the active interface language.
///
/// The English source string doubles as the lookup key: unknown strings and
/// the English locale pass through unchanged, so untranslated surfaces keep
/// working and upstream string changes simply fall back to English.
pub fn tr(text: &str) -> Cow<'_, str> {
    if is_simplified_chinese()
        && let Some(translated) = zh_cn::translate(text)
    {
        return Cow::Borrowed(translated);
    }
    Cow::Borrowed(text)
}

/// Seeds the active language from settings and keeps it in sync with changes,
/// invalidating all views so translated labels re-render without a restart.
pub fn init(ctx: &mut AppContext) {
    set_language(*LanguageSettings::as_ref(ctx).app_language.value());
    ctx.subscribe_to_model(&LanguageSettings::handle(ctx), |_, event, ctx| {
        if matches!(event, LanguageSettingsChangedEvent::AppLanguage { .. }) {
            set_language(*LanguageSettings::as_ref(ctx).app_language.value());
            ctx.invalidate_all_views();
        }
    });
}

#[cfg(test)]
#[path = "i18n_tests.rs"]
mod tests;

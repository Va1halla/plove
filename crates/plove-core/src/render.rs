//! Рендер шаблонов модулей в готовые конфиги.
//!
//! Шаблоны написаны на Tera — это почти Jinja2 из Python:
//! `{{ colors.bg.hex }}`, `{% if %}`, `{% for %}`.

use std::collections::BTreeMap;

use anyhow::{Context, Result};

use crate::modules::Module;
use crate::theme::ThemeContext;

/// Готовый конфиг: куда писать (путь как в манифесте, с `~/`) и что.
pub struct RenderedFile {
    pub dest: String,
    pub content: String,
}

/// Отрендерить все шаблоны модуля.
///
/// * `apps` — команды запуска других компонентов по ролям (`terminal` → `kitty`).
/// * `options` — настройки модуля: его `defaults` с наложенными `overrides`.
pub fn render_module(
    module: &Module,
    theme: &ThemeContext,
    apps: &BTreeMap<String, String>,
    options: &serde_json::Value,
) -> Result<Vec<RenderedFile>> {
    // Context — словарь переменных, доступных шаблону.
    let mut context = tera::Context::from_serialize(theme)?;
    context.insert("apps", apps);
    context.insert("options", options);

    let mut files = Vec::new();
    for entry in &module.manifest.templates {
        let source = module.template(&entry.src)?;
        // one_off: разовый рендер строки. `false` — не экранировать HTML.
        let content = tera::Tera::one_off(source, &context, false)
            .with_context(|| format!("ошибка в шаблоне {}/{}", module.manifest.id, entry.src))?;
        files.push(RenderedFile {
            dest: entry.dest.clone(),
            content,
        });
    }
    Ok(files)
}

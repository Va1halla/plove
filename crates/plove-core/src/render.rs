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

/// Переменные, общие для всех шаблонов: тема и путь к обоям
/// (пустая строка, если обоев нет).
pub fn base_context(theme: &ThemeContext, wallpaper: &str) -> Result<tera::Context> {
    // Context — словарь переменных, доступных шаблону.
    let mut context = tera::Context::from_serialize(theme)?;
    context.insert("wallpaper", wallpaper);
    Ok(context)
}

/// Отрендерить строку-шаблон. `what` нужен только для понятного сообщения об ошибке.
pub fn render_str(template: &str, context: &tera::Context, what: &str) -> Result<String> {
    // one_off: разовый рендер строки. `false` — не экранировать HTML.
    tera::Tera::one_off(template, context, false)
        .with_context(|| format!("ошибка в шаблоне {what}"))
}

/// Отрендерить все шаблоны модуля.
///
/// * `apps` — команды запуска других компонентов по ролям (`terminal` → `kitty`).
/// * `options` — настройки модуля: его `defaults` с наложенными `overrides`.
pub fn render_module(
    module: &Module,
    base: &tera::Context,
    apps: &BTreeMap<String, String>,
    options: &serde_json::Value,
) -> Result<Vec<RenderedFile>> {
    let mut context = base.clone();
    context.insert("apps", apps);
    context.insert("options", options);

    let mut files = Vec::new();
    for entry in &module.manifest.templates {
        let source = module.template(&entry.src)?;
        let what = format!("{}/{}", module.manifest.id, entry.src);
        files.push(RenderedFile {
            dest: entry.dest.clone(),
            content: render_str(source, &context, &what)?,
        });
    }
    Ok(files)
}

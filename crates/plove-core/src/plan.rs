//! План: что именно изменится, если применить райс.
//!
//! Построение плана ничего не меняет в системе, только читает её.
//! Это как `terraform plan`: сначала показываем, потом спрашиваем.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{anyhow, bail, Result};

use crate::modules::{Module, Registry};
use crate::render::render_module;
use crate::spec::RiceSpec;
use crate::system::{self, Env};
use crate::theme::{font_package, theme_context};

/// Что будет с файлом.
// `enum` в Rust — как enum в C#, но варианты ещё и сравниваются через `==`
// благодаря `PartialEq`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    /// Файла ещё нет, будет создан.
    Create,
    /// Файл есть, содержимое изменится.
    Update,
    /// Файл уже такой, как нужно.
    Unchanged,
}

pub struct FileChange {
    pub path: PathBuf,
    pub status: FileStatus,
    pub content: String,
}

pub struct PackageStatus {
    pub name: String,
    /// `Some(true)` — стоит, `Some(false)` — нет, `None` — не удалось проверить.
    pub installed: Option<bool>,
}

pub struct Plan {
    pub spec: RiceSpec,
    pub packages: Vec<PackageStatus>,
    pub files: Vec<FileChange>,
    /// Пустые файлы для ручных правок, которых ещё нет.
    pub user_files: Vec<PathBuf>,
    /// Команды перезагрузки программ после записи конфигов.
    pub reloads: Vec<String>,
    pub warnings: Vec<String>,
}

impl Plan {
    pub fn missing_packages(&self) -> Vec<&str> {
        self.packages
            .iter()
            .filter(|p| p.installed == Some(false))
            .map(|p| p.name.as_str())
            .collect()
    }

    /// Файлы, которые реально будут записаны.
    pub fn changed_files(&self) -> Vec<&FileChange> {
        self.files
            .iter()
            .filter(|f| f.status != FileStatus::Unchanged)
            .collect()
    }

    /// Нечего делать: всё уже применено.
    pub fn is_noop(&self) -> bool {
        self.missing_packages().is_empty()
            && self.changed_files().is_empty()
            && self.user_files.is_empty()
    }
}

/// Построить план для райса.
pub fn build(spec: RiceSpec, registry: &Registry, env: &Env) -> Result<Plan> {
    let mut warnings = Vec::new();

    // 1. Находим модуль для каждой роли.
    let mut modules: Vec<&Module> = Vec::new();
    for (role, id) in &spec.components {
        let module = registry.get(id).ok_or_else(|| {
            anyhow!(
                "модуль «{id}» не найден. Доступные модули: {}",
                registry.ids().join(", ")
            )
        })?;
        if module.manifest.role != *role {
            bail!(
                "«{id}» — это {}, его нельзя поставить на роль {role}",
                module.manifest.role
            );
        }
        modules.push(module);
    }

    // 2. Проверяем совместимость: например, панель знает, с какими композиторами работает.
    for module in &modules {
        for (other_role, allowed) in &module.manifest.compatible_with {
            if let Some(chosen) = spec.components.get(other_role) {
                if !allowed.contains(chosen) {
                    bail!(
                        "{} не работает с {chosen}. Совместимые варианты для роли {other_role}: {}",
                        module.manifest.id,
                        allowed.join(", ")
                    );
                }
            }
        }
    }
    for id in spec.overrides.keys() {
        if !modules.iter().any(|m| m.manifest.id == *id) {
            warnings.push(format!(
                "overrides для «{id}» не используются: такого модуля нет в components"
            ));
        }
    }

    // 3. Команды запуска по ролям, чтобы модули могли ссылаться друг на друга.
    let apps: BTreeMap<String, String> = modules
        .iter()
        .filter_map(|m| Some((m.manifest.role.clone(), m.manifest.exec.clone()?)))
        .collect();

    // 4. Рендерим конфиги и сравниваем с тем, что сейчас на диске.
    let theme = theme_context(&spec);
    let mut files: Vec<FileChange> = Vec::new();
    for module in &modules {
        let options = module_options(module, spec.overrides.get(&module.manifest.id))?;
        for rendered in render_module(module, &theme, &apps, &options)? {
            let path = env.expand(&rendered.dest);
            if files.iter().any(|f| f.path == path) {
                bail!("два модуля пишут в один файл {}", rendered.dest);
            }
            // read_to_string вернёт ошибку, если файла нет: `.ok()` превращает её в None.
            let status = match std::fs::read_to_string(&path).ok() {
                None => FileStatus::Create,
                Some(current) if current == rendered.content => FileStatus::Unchanged,
                Some(_) => FileStatus::Update,
            };
            files.push(FileChange {
                path,
                status,
                content: rendered.content,
            });
        }
    }

    // 5. Пакеты: от модулей и от шрифтов. Без повторов, порядок сохраняем.
    let mut package_names: Vec<String> = Vec::new();
    for module in &modules {
        package_names.extend(module.manifest.packages.iter().cloned());
    }
    let fonts = &spec.theme.fonts;
    for font in [&fonts.ui, &fonts.mono] {
        match font_package(font) {
            Some(package) => package_names.push(package.to_string()),
            None => warnings.push(format!(
                "не знаю, какой пакет ставит шрифт «{font}». Установите его вручную"
            )),
        }
    }
    let mut packages: Vec<PackageStatus> = Vec::new();
    for name in package_names {
        if !packages.iter().any(|p| p.name == name) {
            let installed = system::package_installed(&name);
            packages.push(PackageStatus { name, installed });
        }
    }

    // 6. Файлы для ручных правок и команды перезагрузки.
    let user_files = modules
        .iter()
        .flat_map(|m| &m.manifest.user_files)
        .map(|f| env.expand(f))
        .filter(|path| !path.exists())
        .collect();
    let reloads = modules
        .iter()
        .filter_map(|m| m.manifest.reload.clone())
        .collect();

    if spec.theme.wallpaper.is_some() {
        warnings.push("обои пока не применяются, это появится в следующей версии".to_string());
    }

    Ok(Plan {
        spec,
        packages,
        files,
        user_files,
        reloads,
        warnings,
    })
}

/// Настройки модуля: его `defaults`, поверх которых наложены `overrides` из райса.
/// Неизвестный ключ — ошибка, чтобы опечатка не терялась молча.
fn module_options(
    module: &Module,
    overrides: Option<&serde_yaml::Mapping>,
) -> Result<serde_json::Value> {
    let mut options = module.manifest.defaults.clone();
    if let Some(overrides) = overrides {
        for (key, value) in overrides {
            if !options.contains_key(key) {
                let known: Vec<&str> = options.keys().filter_map(|k| k.as_str()).collect();
                bail!(
                    "у модуля {} нет настройки «{}». Доступные: {}",
                    module.manifest.id,
                    key.as_str().unwrap_or("?"),
                    known.join(", ")
                );
            }
            options.insert(key.clone(), value.clone());
        }
    }
    Ok(serde_json::to_value(options)?)
}

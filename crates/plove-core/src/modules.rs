//! Модули: по одному на каждую программу (hyprland, waybar, kitty…).
//!
//! Модуль — это папка в `modules/` с манифестом `module.yaml` и шаблонами.
//! Чтобы добавить новую программу, достаточно добавить папку: код ядра
//! при этом не меняется.
//!
//! Папка `modules/` вшивается прямо в бинарник при сборке (макрос
//! `include_dir!`), как embedded resources в C#. Поэтому plove — один
//! файл, которому не нужны данные рядом.

use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Context, Result};
use include_dir::{include_dir, Dir};
use serde::Deserialize;

static BUILTIN_MODULES: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../modules");

/// Содержимое `module.yaml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub id: String,
    /// Роль в райсе: compositor, bar, terminal…
    pub role: String,
    /// Команда запуска. Нужна другим модулям: например, Hyprland
    /// запускает панель и вешает терминал на Super+Enter.
    /// Это тоже шаблон: может использовать `{{ wallpaper }}` и цвета темы.
    #[serde(default)]
    pub exec: Option<String>,
    /// Пакеты pacman, которые нужны модулю.
    #[serde(default)]
    pub packages: Vec<String>,
    /// С какими модулями других ролей совместим: роль → список id.
    #[serde(default)]
    pub compatible_with: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub templates: Vec<TemplateEntry>,
    /// Файлы для ручных правок пользователя. plove создаёт их пустыми
    /// и никогда не перезаписывает.
    #[serde(default)]
    pub user_files: Vec<String>,
    /// Команда, после которой программа перечитывает конфиг.
    /// Шаблон, как и `exec`; внутри доступна `{{ exec }}` — готовая команда запуска.
    #[serde(default)]
    pub reload: Option<String>,
    /// Настройки, которые можно менять через `overrides`, и их значения по умолчанию.
    #[serde(default)]
    pub defaults: serde_yaml::Mapping,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateEntry {
    /// Имя файла шаблона в папке модуля.
    pub src: String,
    /// Куда писать результат. Всегда внутри домашней папки, начинается с `~/`.
    pub dest: String,
}

pub struct Module {
    pub manifest: Manifest,
    dir: &'static Dir<'static>,
}

impl Module {
    /// Текст шаблона по имени файла.
    pub fn template(&self, name: &str) -> Result<&'static str> {
        let file = self
            .dir
            .files()
            .find(|f| f.path().file_name().is_some_and(|n| n == name))
            .ok_or_else(|| anyhow!("в модуле {} нет шаблона {name}", self.manifest.id))?;
        file.contents_utf8()
            .ok_or_else(|| anyhow!("шаблон {}/{name} не в UTF-8", self.manifest.id))
    }
}

/// Реестр всех известных модулей.
pub struct Registry {
    modules: BTreeMap<String, Module>,
}

impl Registry {
    /// Загрузить модули, вшитые в бинарник.
    pub fn builtin() -> Result<Registry> {
        let mut modules = BTreeMap::new();
        for dir in BUILTIN_MODULES.dirs() {
            let module = load_module(dir)
                .with_context(|| format!("ошибка в модуле {}", dir.path().display()))?;
            modules.insert(module.manifest.id.clone(), module);
        }
        Ok(Registry { modules })
    }

    pub fn get(&self, id: &str) -> Option<&Module> {
        self.modules.get(id)
    }

    /// Все модули по порядку id.
    pub fn iter(&self) -> impl Iterator<Item = &Module> {
        self.modules.values()
    }

    pub fn ids(&self) -> Vec<&str> {
        self.modules.keys().map(String::as_str).collect()
    }
}

fn load_module(dir: &'static Dir<'static>) -> Result<Module> {
    let manifest_file = dir
        .files()
        .find(|f| f.path().file_name().is_some_and(|n| n == "module.yaml"))
        .ok_or_else(|| anyhow!("нет файла module.yaml"))?;
    let text = manifest_file
        .contents_utf8()
        .ok_or_else(|| anyhow!("module.yaml не в UTF-8"))?;
    let manifest: Manifest = serde_yaml::from_str(text)?;

    let dir_name = dir
        .path()
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    if manifest.id != dir_name {
        bail!(
            "id «{}» не совпадает с именем папки «{dir_name}»",
            manifest.id
        );
    }
    // Модули пишут только в домашнюю папку. Это ограничение безопасности:
    // ни один модуль не может тронуть системные файлы.
    let dests = manifest.templates.iter().map(|t| &t.dest);
    for dest in dests.chain(&manifest.user_files) {
        if !dest.starts_with("~/") {
            bail!("путь {dest} должен начинаться с ~/");
        }
    }
    let module = Module { manifest, dir };
    // Проверяем, что все шаблоны из манифеста существуют.
    for t in &module.manifest.templates {
        module.template(&t.src)?;
    }
    Ok(module)
}

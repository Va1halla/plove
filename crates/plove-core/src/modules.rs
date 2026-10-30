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
    /// Пакеты из AUR (ставятся через paru; paru plove поставит сам).
    #[serde(default)]
    pub aur: Vec<String>,
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
    /// Значения из `settings` при загрузке тоже попадают сюда.
    #[serde(default)]
    pub defaults: serde_yaml::Mapping,
    /// Описанные настройки: название, пояснение, границы. По ним мастер
    /// строит экран настроек, а план проверяет значения из райса.
    #[serde(default)]
    pub settings: Vec<Setting>,
    /// Запускать `exec` вместе с Hyprland (exec-once): панель, обои, демоны.
    #[serde(default)]
    pub autostart: bool,
    /// Переменные окружения для всех программ сессии, в формате Hyprland
    /// `ИМЯ,значение`, например `QT_QPA_PLATFORMTHEME,qt6ct`.
    #[serde(default)]
    pub env: Vec<String>,
    /// Горячие клавиши в формате Hyprland без `bind =`, например
    /// `$mod, D, exec, {{ exec }}`. Шаблоны: доступны `exec`, `apps`, цвета.
    #[serde(default)]
    pub binds: Vec<String>,
    /// Только у лаунчеров: как вызвать его в режиме «выбрать строку из списка»
    /// (dmenu). Им пользуются меню выключения и история буфера обмена.
    #[serde(default)]
    pub dmenu: Option<String>,
    /// Только у терминалов: префикс команды «открыть программу в терминале»,
    /// например `alacritty -e`. Им пользуются btop, yazi и подобные.
    #[serde(default)]
    pub run_in: Option<String>,
    /// Имя процесса фоновой программы. Если модуль убрали из райса,
    /// plove остановит этот процесс при следующем применении.
    #[serde(default)]
    pub process: Option<String>,
    /// Программы, которые мешают этой и останавливаются при применении
    /// (например, два демона уведомлений не уживаются вместе).
    #[serde(default)]
    pub stops: Vec<String>,
    /// Пакеты, которые конфликтуют с пакетами модуля: план предупредит,
    /// если они установлены (pacman предложит их удалить).
    #[serde(default)]
    pub conflicts: Vec<String>,
    /// Строки, которые нужно дописать в чужой файл, не перезаписывая его
    /// (например, подключить конфиг plove в ~/.zshrc).
    #[serde(default)]
    pub ensure_lines: Vec<EnsureLine>,
    /// Команды, которые выполняются после каждого применения с изменениями,
    /// даже вне графической сессии (например, программы по умолчанию).
    #[serde(default)]
    pub on_apply: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnsureLine {
    /// Файл, всегда внутри домашней папки (`~/…`).
    pub file: String,
    /// Строка, которая должна там быть.
    pub line: String,
    /// Не трогать файл, если существует этот путь (например, init.vim
    /// вместо init.lua: Neovim не любит оба сразу).
    #[serde(default)]
    pub unless_exists: Option<String>,
}

/// Одна описанная настройка модуля.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Setting {
    /// Ключ в `overrides`, например `blur_size`.
    pub id: String,
    /// Группа на экране настроек: «Эффекты», «Окна»…
    pub group: String,
    pub title: String,
    /// Пояснение в одно-два предложения.
    #[serde(default)]
    pub about: String,
    pub default: serde_yaml::Value,
    /// Для чисел: наименьшее и наибольшее значение.
    #[serde(default)]
    pub range: Option<[f64; 2]>,
    /// Для чисел: шаг ползунка (по умолчанию 1).
    #[serde(default)]
    pub step: Option<f64>,
    /// Для выбора из списка: допустимые значения.
    #[serde(default)]
    pub choices: Vec<String>,
    /// Для списков: из чего можно собирать (свои виджеты райса тоже можно).
    #[serde(default)]
    pub items: Vec<String>,
}

/// Какого вида настройка: от этого зависит, как её показывать и проверять.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingKind {
    Switch,
    Number,
    Choice,
    /// Список или строка без ограничений: меняется только в файле райса.
    Other,
}

impl Setting {
    pub fn kind(&self) -> SettingKind {
        if !self.choices.is_empty() {
            SettingKind::Choice
        } else if self.range.is_some() {
            SettingKind::Number
        } else if self.default.is_bool() {
            SettingKind::Switch
        } else {
            SettingKind::Other
        }
    }

    /// Проверить значение из райса. Ошибка объясняет, что не так и что можно.
    pub fn check(&self, value: &serde_yaml::Value) -> Result<()> {
        match self.kind() {
            SettingKind::Switch if !value.is_bool() => {
                bail!("«{}» — это вкл/выкл: true или false", self.title)
            }
            SettingKind::Choice => {
                let ok = value
                    .as_str()
                    .is_some_and(|v| self.choices.iter().any(|c| c == v));
                if !ok {
                    bail!("«{}»: можно {}", self.title, self.choices.join(", "));
                }
            }
            SettingKind::Number => {
                let [min, max] = self.range.unwrap_or([f64::MIN, f64::MAX]);
                match value.as_f64() {
                    Some(v) if (min..=max).contains(&v) => {}
                    _ => bail!("«{}»: число от {min} до {max}", self.title),
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Следующее (или предыдущее) значение: для стрелок ←→ в мастере.
    pub fn step_value(&self, value: &serde_yaml::Value, forward: bool) -> serde_yaml::Value {
        use serde_yaml::Value;
        match self.kind() {
            SettingKind::Switch => Value::Bool(!value.as_bool().unwrap_or(false)),
            SettingKind::Choice => {
                let n = self.choices.len();
                let i = value
                    .as_str()
                    .and_then(|v| self.choices.iter().position(|c| c == v))
                    .unwrap_or(0);
                let next = if forward {
                    (i + 1) % n
                } else {
                    (i + n - 1) % n
                };
                Value::String(self.choices[next].clone())
            }
            SettingKind::Number => {
                let [min, max] = self.range.unwrap_or([0.0, 0.0]);
                let step = self.step.unwrap_or(1.0);
                let v = value.as_f64().unwrap_or(min);
                let next = if forward { v + step } else { v - step }.clamp(min, max);
                // Целые остаются целыми: `6`, а не `6.0` в файле райса.
                let whole = step.fract() == 0.0 && min.fract() == 0.0;
                if whole {
                    Value::Number((next.round() as i64).into())
                } else {
                    Value::Number(((next * 100.0).round() / 100.0).into())
                }
            }
            SettingKind::Other => value.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateEntry {
    /// Имя файла шаблона в папке модуля.
    pub src: String,
    /// Куда писать результат. Всегда внутри домашней папки, начинается с `~/`.
    pub dest: String,
    /// Скопировать как есть, без Tera: например, QML, где `{#` и `{{`
    /// значат своё.
    #[serde(default)]
    pub copy: bool,
    /// Условие Tera, например `hypr_lua`: файл пишется, только если оно
    /// истинно. Иначе plove удалит этот файл, если писала его раньше.
    #[serde(default)]
    pub when: Option<String>,
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
    let mut manifest: Manifest = serde_yaml::from_str(text)?;
    // Значения описанных настроек — те же defaults: шаблонам всё равно,
    // откуда пришло значение.
    for setting in &manifest.settings {
        if manifest.defaults.contains_key(setting.id.as_str()) {
            bail!(
                "настройка {} описана и в defaults, и в settings",
                setting.id
            );
        }
        setting
            .check(&setting.default)
            .with_context(|| format!("значение по умолчанию у {}", setting.id))?;
        manifest
            .defaults
            .insert(setting.id.clone().into(), setting.default.clone());
    }

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

/// Значение настройки одной строкой: `true`, `0.15`, `us,ru`, `[]`.
pub fn show_value(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::String(s) => s.clone(),
        other => serde_yaml::to_string(other)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

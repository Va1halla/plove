//! План: что именно изменится, если применить райс.
//!
//! Построение плана ничего не меняет в системе, только читает её.
//! Это как `terraform plan`: сначала показываем, потом спрашиваем.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{anyhow, bail, Context, Result};

use crate::modules::{Module, Registry};
use crate::render;
use crate::spec::{RiceSpec, Wallpaper};
use crate::state;
use crate::system::{self, Env};
use crate::theme::{font_package, theme_context};
use crate::wallpaper;

/// Куда кладутся обои, нарисованные plove.
pub const GENERATED_WALLPAPER: &str = "~/.local/share/plove/wallpaper.png";

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
    /// Байты, а не текст: среди файлов бывают картинки (обои).
    pub content: Vec<u8>,
}

pub struct PackageStatus {
    pub name: String,
    /// `Some(true)` — стоит, `Some(false)` — нет, `None` — не удалось проверить.
    pub installed: Option<bool>,
    /// Пакет из AUR: ставится через paru, а не pacman.
    pub aur: bool,
}

pub struct Plan {
    pub spec: RiceSpec,
    pub packages: Vec<PackageStatus>,
    pub files: Vec<FileChange>,
    /// Пустые файлы для ручных правок, которых ещё нет.
    pub user_files: Vec<PathBuf>,
    /// Команды перезагрузки программ после записи конфигов
    /// (только внутри графической сессии).
    pub reloads: Vec<String>,
    /// Команды после записи конфигов, которые выполняются всегда.
    pub on_apply: Vec<String>,
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

    /// Недостающие пакеты по источнику: `aur = false` — pacman, `true` — AUR.
    pub fn missing_from(&self, aur: bool) -> Vec<&str> {
        self.packages
            .iter()
            .filter(|p| p.installed == Some(false) && p.aur == aur)
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

    // 3. Обои: свой файл или нарисованные из палитры. Нужны, только если
    //    в райсе есть компонент, который их показывает.
    let mut files: Vec<FileChange> = Vec::new();
    let mut wallpaper_path = String::new();
    match (
        &spec.theme.wallpaper,
        spec.components.contains_key("wallpaper"),
    ) {
        (None, _) => {}
        (Some(_), false) => warnings.push(
            "обои заданы, но в components нет wallpaper, поэтому они не применятся".to_string(),
        ),
        (Some(Wallpaper::File(file)), true) => {
            let path = env.expand(file);
            if !path.exists() {
                warnings.push(format!("файл обоев {file} не найден"));
            }
            wallpaper_path = path.display().to_string();
        }
        (Some(Wallpaper::Generated { generate }), true) => {
            let path = env.expand(GENERATED_WALLPAPER);
            wallpaper_path = path.display().to_string();
            let png = wallpaper::generate_png(*generate, &spec.theme.palette)?;
            files.push(file_change(path, png));
        }
    }

    // 4. Команды запуска по ролям, чтобы модули могли ссылаться друг на друга.
    let theme = theme_context(&spec);
    let mut base = render::base_context(&theme, &wallpaper_path)?;
    // Виртуальная машина (vmware, oracle, kvm…) или пустая строка.
    // В песочнице всегда пусто, чтобы результат не зависел от машины.
    let vm = if env.sandbox {
        String::new()
    } else {
        system::virtual_machine()
    };
    base.insert("vm", &vm);
    // Домашняя папка: для программ, которые не понимают ~ в путях.
    base.insert("home", &env.home.display().to_string());
    // Режим «выбрать из списка» у лаунчера. Нужен до команд запуска:
    // через него работает, например, меню выключения.
    // То же для «открыть программу в терминале».
    let mut dmenu = String::new();
    let mut run_in = String::new();
    for module in &modules {
        let id = &module.manifest.id;
        if let Some(template) = &module.manifest.dmenu {
            dmenu = render::render_str(template, &base, &format!("{id}/module.yaml (dmenu)"))?;
        }
        if let Some(template) = &module.manifest.run_in {
            run_in = render::render_str(template, &base, &format!("{id}/module.yaml (run_in)"))?;
        }
    }
    base.insert("dmenu", &dmenu);
    base.insert("run_in", &run_in);
    let mut apps: BTreeMap<String, String> = BTreeMap::new();
    for module in &modules {
        if let Some(exec) = &module.manifest.exec {
            let what = format!("{}/module.yaml (exec)", module.manifest.id);
            apps.insert(
                module.manifest.role.clone(),
                render::render_str(exec, &base, &what)?,
            );
        }
    }

    // Автозапуск и горячие клавиши всех модулей: их собирает конфиг Hyprland.
    let mut autostart = Vec::new();
    let mut binds = Vec::new();
    let session_env: Vec<&String> = modules.iter().flat_map(|m| &m.manifest.env).collect();
    // Клавиши, которые уже заняты самим конфигом Hyprland (см. hyprland.conf.tera).
    let mut taken_keys: BTreeMap<String, String> = ["Q", "V", "F"]
        .into_iter()
        .map(|k| (bind_key(&format!("$mod, {k}")), "hyprland".to_string()))
        .collect();
    for module in &modules {
        let exec = apps.get(&module.manifest.role);
        if module.manifest.autostart {
            if let Some(exec) = exec {
                autostart.push(exec.clone());
            }
        }
        let mut context = base.clone();
        context.insert("apps", &apps);
        context.insert("exec", &exec);
        for bind in &module.manifest.binds {
            let what = format!("{}/module.yaml (binds)", module.manifest.id);
            let bind = render::render_str(bind, &context, &what)?;
            // Два модуля на одной клавише: Hyprland запустит оба, это почти всегда ошибка.
            let key = bind_key(&bind);
            if let Some(owner) = taken_keys.get(&key) {
                warnings.push(format!(
                    "клавиша {} занята дважды: {} и {}",
                    key, owner, module.manifest.id
                ));
            } else {
                taken_keys.insert(key, module.manifest.id.clone());
            }
            binds.push(bind);
        }
    }
    base.insert("autostart", &autostart);
    base.insert("binds", &binds);
    base.insert("session_env", &session_env);

    // 5. Рендерим конфиги и сравниваем с тем, что сейчас на диске.
    for module in &modules {
        let options = module_options(module, spec.overrides.get(&module.manifest.id))?;
        for rendered in render::render_module(module, &base, &apps, &options)? {
            let path = env.expand(&rendered.dest);
            if files.iter().any(|f| f.path == path) {
                bail!("два модуля пишут в один файл {}", rendered.dest);
            }
            files.push(file_change(path, rendered.content.into_bytes()));
        }
    }

    // Строки в чужих файлах: дописываем, только если их там ещё нет.
    for module in &modules {
        for entry in &module.manifest.ensure_lines {
            if !entry.file.starts_with("~/") {
                bail!(
                    "{}: ensure_lines должен указывать на файл в ~/",
                    module.manifest.id
                );
            }
            if let Some(other) = &entry.unless_exists {
                if env.expand(other).exists() {
                    warnings.push(format!(
                        "{}: есть {other}, поэтому {} не трогаю. Добавьте вручную: {}",
                        module.manifest.id, entry.file, entry.line
                    ));
                    continue;
                }
            }
            let path = env.expand(&entry.file);
            let mut text = std::fs::read_to_string(&path).unwrap_or_default();
            if text.lines().any(|l| l.trim() == entry.line.trim()) {
                continue;
            }
            if files.iter().any(|f| f.path == path) {
                bail!("два модуля пишут в один файл {}", entry.file);
            }
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str(&entry.line);
            text.push('\n');
            files.push(file_change(path, text.into_bytes()));
        }
    }

    // 6. Пакеты: от модулей и от шрифтов. Без повторов, порядок сохраняем.
    // Второй элемент пары: пакет из AUR.
    let mut package_names: Vec<(String, bool)> = Vec::new();
    for module in &modules {
        package_names.extend(module.manifest.packages.iter().map(|p| (p.clone(), false)));
        package_names.extend(module.manifest.aur.iter().map(|p| (p.clone(), true)));
    }
    let fonts = &spec.theme.fonts;
    for font in [&fonts.ui, &fonts.mono] {
        match font_package(font) {
            Some(package) => package_names.push((package.to_string(), false)),
            None => warnings.push(format!(
                "не знаю, какой пакет ставит шрифт «{font}». Установите его вручную"
            )),
        }
    }
    let mut packages: Vec<PackageStatus> = Vec::new();
    for (name, aur) in package_names {
        if !packages.iter().any(|p| p.name == name) {
            let installed = system::package_installed(&name);
            packages.push(PackageStatus {
                name,
                installed,
                aur,
            });
        }
    }

    // 7. Файлы для ручных правок и команды перезагрузки.
    let user_files = modules
        .iter()
        .flat_map(|m| &m.manifest.user_files)
        .map(|f| env.expand(f))
        .filter(|path| !path.exists())
        .collect();
    // Сначала останавливаем программы, которые мешают выбранным
    // (но не сами выбранные: их имя совпадает с первым словом команды запуска).
    let chosen: Vec<&str> = apps
        .values()
        .filter_map(|exec| exec.split_whitespace().next())
        .collect();
    let mut reloads = Vec::new();
    let mut stop = |name: &str| {
        let command = stop_command(name);
        if !chosen.contains(&name) && !reloads.contains(&command) {
            reloads.push(command);
        }
    };
    // Программы, которые были в прошлом применённом райсе, а теперь убраны:
    // иначе они так и висели бы до перезахода (например, виджеты eww).
    for id in previous_modules(env) {
        if spec.components.values().any(|c| *c == id) {
            continue;
        }
        if let Some(process) = registry.get(&id).and_then(|m| m.manifest.process.as_ref()) {
            stop(process);
        }
    }
    for module in &modules {
        for name in &module.manifest.stops {
            stop(name);
        }
    }
    for module in &modules {
        for package in &module.manifest.conflicts {
            if system::package_installed(package) == Some(true) {
                warnings.push(format!(
                    "{} конфликтует с установленным пакетом {package}: pacman предложит его удалить",
                    module.manifest.id
                ));
            }
        }
    }
    let mut on_apply = Vec::new();
    for module in &modules {
        for command in &module.manifest.on_apply {
            let what = format!("{}/module.yaml (on_apply)", module.manifest.id);
            on_apply.push(render::render_str(command, &base, &what)?);
        }
    }
    for module in &modules {
        if let Some(reload) = &module.manifest.reload {
            let mut context = base.clone();
            context.insert("exec", &apps.get(&module.manifest.role));
            let options = module_options(module, spec.overrides.get(&module.manifest.id))?;
            context.insert("options", &options);
            let what = format!("{}/module.yaml (reload)", module.manifest.id);
            reloads.push(render::render_str(reload, &context, &what)?);
        }
    }

    Ok(Plan {
        spec,
        packages,
        files,
        user_files,
        reloads,
        on_apply,
        warnings,
    })
}

/// Сравнить новое содержимое файла с тем, что сейчас на диске.
/// Модули последнего применённого райса (пусто, если применений не было).
fn previous_modules(env: &Env) -> Vec<String> {
    let last = state::list(env).ok().and_then(|mut g| g.pop());
    last.and_then(|g| std::fs::read_to_string(g.dir.join("spec.yaml")).ok())
        .and_then(|text| RiceSpec::from_yaml(&text).ok())
        .map(|spec| spec.components.into_values().collect())
        .unwrap_or_default()
}

/// Команда, которая останавливает процесс и не считается ошибкой, если его нет.
fn stop_command(name: &str) -> String {
    // Linux хранит имя процесса только до 15 символов, поэтому длинные
    // имена и имена с аргументами ищем по всей командной строке.
    let flag = if name.len() > 15 || name.contains(' ') {
        "-f"
    } else {
        "-x"
    };
    format!("pkill {flag} '{name}'; true")
}

/// Сочетание клавиш из строки `bind`: «$mod SHIFT, E, exec, …» → «$MOD+SHIFT+E».
/// Модификаторы сортируются, чтобы «SHIFT $mod» и «$mod SHIFT» совпали.
fn bind_key(bind: &str) -> String {
    let mut parts = bind.splitn(3, ',');
    let mut mods: Vec<String> = parts
        .next()
        .unwrap_or("")
        .split_whitespace()
        .map(str::to_uppercase)
        .collect();
    mods.sort();
    mods.push(parts.next().unwrap_or("").trim().to_uppercase());
    mods.join("+")
}

fn file_change(path: PathBuf, content: Vec<u8>) -> FileChange {
    // fs::read вернёт ошибку, если файла нет: `.ok()` превращает её в None.
    let status = match std::fs::read(&path).ok() {
        None => FileStatus::Create,
        Some(current) if current == content => FileStatus::Unchanged,
        Some(_) => FileStatus::Update,
    };
    FileChange {
        path,
        status,
        content,
    }
}

/// Настройки модуля: его `defaults`, поверх которых наложены `overrides` из райса.
/// Неизвестный ключ — ошибка, чтобы опечатка не терялась молча.
pub fn module_options(
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
            let setting = module
                .manifest
                .settings
                .iter()
                .find(|s| Some(s.id.as_str()) == key.as_str());
            if let Some(setting) = setting {
                setting
                    .check(value)
                    .with_context(|| format!("модуль {}", module.manifest.id))?;
            }
            options.insert(key.clone(), value.clone());
        }
    }
    Ok(serde_json::to_value(options)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_key_ignores_modifier_order_and_case() {
        assert_eq!(bind_key("$mod SHIFT, E, exec, wlogout"), "$MOD+SHIFT+E");
        assert_eq!(bind_key("SHIFT $mod, e, exec, other"), "$MOD+SHIFT+E");
        assert_eq!(bind_key(", Print, exec, grim"), "PRINT");
        assert_ne!(
            bind_key("$mod, E, exec, thunar"),
            bind_key("$mod SHIFT, E, exec, x")
        );
    }
}

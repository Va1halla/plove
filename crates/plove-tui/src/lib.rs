//! TUI-мастер: `plove` без аргументов.
//!
//! Пять экранов по порядку:
//!
//! ```text
//! 1. Проверка   — готова ли система (то же, что plove doctor)
//! 2. Райс       — выбор готового райса
//! 3. Настройки  — раскладка, положение панели, выбор программ
//! 4. Тонкая     — все описанные настройки каждой программы с пояснениями
//! 5. План       — что будет установлено и записано
//! ```
//!
//! Мастер сам ничего не применяет: он возвращает выбранный райс, а
//! применяет его CLI уже вне TUI, потому что pacman и sudo спрашивают
//! пароль в обычном терминале.
//!
//! Устройство как у любого игрового цикла: нарисовать экран → дождаться
//! клавиши → изменить состояние → снова нарисовать. В ratatui экран
//! каждый раз рисуется заново целиком из состояния `App`.

use std::io;

use anyhow::Result;
use plove_core::check::{self, Check, Level, LiveProbe, Probe};
use std::collections::BTreeMap;

use plove_core::modules::{show_value, Registry, Setting, SettingKind};
use plove_core::plan::{self, FileStatus, Plan};
use plove_core::presets;
use plove_core::spec::{Color as SpecColor, RiceSpec};
use plove_core::system::Env;
use plove_preview::Pixmap;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::{Resize, StatefulImage};
use serde_yaml::Value;

/// Чем закончился мастер.
pub enum Outcome {
    /// Пользователь подтвердил план: применить этот райс.
    /// `from_picture` — райс собран в мастере из своей картинки, его
    /// стоит сохранить в файл, чтобы потом поменять (`plove edit`).
    Apply {
        spec: Box<RiceSpec>,
        from_picture: bool,
    },
    Quit,
}

/// Запустить мастер в текущем терминале.
///
/// `own` — райс пользователя (`plove edit файл.yaml`): он встаёт первым
/// в списке и уже выбран, а дальше всё как с готовыми райсами.
pub fn run(registry: &Registry, env: &Env, own: Option<RiceSpec>) -> Result<Outcome> {
    let mut app = App::new(&LiveProbe)?;
    if let Some(spec) = own {
        app.presets.insert(0, spec);
        app.own = true;
    }
    app.sandbox = env.sandbox;
    app.show_preview = std::env::var("TERM").map_or(true, |term| term != "linux");
    let mut terminal = ratatui::init();
    if app.show_preview {
        // Спросить терминал, умеет ли он показывать картинки (kitty, ghostty,
        // WezTerm, foot…) и какой у него размер клетки в пикселях.
        // Не умеет — остаются наши полублоки.
        app.picker = Picker::from_query_stdio()
            .ok()
            .filter(|p| p.protocol_type() != ProtocolType::Halfblocks);
    }
    let result = event_loop(&mut terminal, &mut app, registry, env);
    // Вернуть терминал в обычный режим, даже если была ошибка.
    ratatui::restore();
    result
}

fn event_loop(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    registry: &Registry,
    env: &Env,
) -> Result<Outcome> {
    loop {
        terminal.draw(|frame| app.draw(frame))?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        // На некоторых терминалах приходят и нажатия, и отпускания клавиш.
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match app.handle(key.code, registry, env) {
            Action::None => {}
            Action::Quit => return Ok(Outcome::Quit),
            Action::Apply(spec) => {
                return Ok(Outcome::Apply {
                    spec,
                    from_picture: app.picked,
                })
            }
            Action::Fix => {
                // Исправления спрашивают пароль sudo, поэтому временно
                // выходим из TUI в обычный терминал.
                ratatui::restore();
                run_fixes(&app.checks);
                *terminal = ratatui::init();
                app.checks = check::run_all(&LiveProbe);
            }
        }
    }
}

fn run_fixes(checks: &[Check]) {
    for c in checks.iter().filter(|c| !c.fix.is_empty()) {
        println!("Исправляю: {}", c.title);
        for command in &c.fix {
            println!("  $ {}", command.join(" "));
        }
        if let Err(e) = check::apply_fix(c) {
            println!("  ошибка: {e:#}");
        }
    }
    if checks.iter().any(|c| c.needs_relogin) {
        println!(
            "\nПосле мастера выйдите из системы и войдите заново: изменения групп вступят в силу."
        );
    }
    println!("\nНажмите Enter, чтобы вернуться в мастер.");
    let mut line = String::new();
    let _ = io::stdin().read_line(&mut line);
}

// ───────────────────────── состояние ─────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Checks,
    Rice,
    Settings,
    Tune,
    Plan,
}

impl Step {
    const ALL: [Step; 5] = [
        Step::Checks,
        Step::Rice,
        Step::Settings,
        Step::Tune,
        Step::Plan,
    ];

    fn title(self) -> &'static str {
        match self {
            Step::Checks => "Проверка",
            Step::Rice => "Райс",
            Step::Settings => "Настройки",
            Step::Tune => "Тонкая настройка",
            Step::Plan => "План",
        }
    }
}

/// Что сделать после нажатия клавиши.
enum Action {
    None,
    Quit,
    Fix,
    /// `Box` — чтобы перечисление не раздувалось до размера RiceSpec.
    Apply(Box<RiceSpec>),
}

/// Роли, для которых мастер предлагает выбрать программу:
/// (роль, подпись, можно ли обойтись без неё).
const PROGRAM_ROLES: [(&str, &str, bool); 7] = [
    ("terminal", "Терминал", false),
    ("launcher", "Лаунчер", false),
    ("notifications", "Уведомления", false),
    ("wallpaper", "Обои", false),
    ("power", "Меню выхода", false),
    ("widgets", "Виджеты", true),
    ("nightlight", "Ночной режим", true),
];

/// Выбор программы для одной роли, например лаунчер fuzzel или rofi.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Program {
    role: &'static str,
    label: &'static str,
    /// id модулей этой роли; пустая строка — «без этой программы».
    choices: Vec<String>,
    current: usize,
}

/// Токены стиля на экране настроек: (ключ в theme.style, подпись, варианты).
const STYLE_TOKENS: [(&str, &str, &[&str]); 7] = [
    ("shape", "Форма", &["sharp", "rounded", "pill"]),
    (
        "bar",
        "Вид панели",
        &["solid", "floating", "islands", "minimal"],
    ),
    (
        "separators",
        "Разделители",
        &["none", "dots", "line", "arrows", "slants", "round"],
    ),
    (
        "borders",
        "Рамки окон",
        &["none", "thin", "accent", "gradient"],
    ),
    ("density", "Плотность", &["compact", "normal", "spacious"]),
    ("labels", "Подписи", &["glyphs", "text", "both"]),
    ("glass", "Стекло", &["none", "light", "strong"]),
];

const POSITIONS: [&str; 4] = ["top", "bottom", "left", "right"];

/// Настройки, которые мастер позволяет поменять поверх готового райса.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Settings {
    /// Раскладка us,ru вместо us.
    russian: bool,
    /// Индекс в POSITIONS.
    position: usize,
    /// Индексы вариантов в STYLE_TOKENS.
    style: [usize; 7],
    programs: Vec<Program>,
}

impl Settings {
    /// Строки до списка программ: раскладка, панель и токены стиля.
    const SWITCHES: usize = 2 + STYLE_TOKENS.len();

    /// Прочитать текущие значения из райса. Из реестра берём, какие
    /// программы вообще есть для каждой роли.
    fn from_spec(spec: &RiceSpec, registry: &Registry) -> Settings {
        let get = |module: &str, key: &str| spec.overrides.get(module).and_then(|m| m.get(key));
        let mut programs = Vec::new();
        for (role, label, optional) in PROGRAM_ROLES {
            let mut choices: Vec<String> = registry
                .iter()
                .filter(|m| m.manifest.role == role)
                .map(|m| m.manifest.id.clone())
                .collect();
            choices.sort();
            if optional {
                choices.insert(0, String::new());
            }
            // Выбирать не из чего — строку не показываем.
            if choices.len() < 2 {
                continue;
            }
            let chosen = spec.components.get(role).map_or("", String::as_str);
            let current = choices.iter().position(|c| c == chosen).unwrap_or(0);
            programs.push(Program {
                role,
                label,
                choices,
                current,
            });
        }
        // Стиль через YAML: так не нужно перечислять варианты каждого enum в коде.
        let style = serde_yaml::to_value(spec.theme.style).unwrap_or_default();
        let mut indexes = [0; 7];
        for (i, (key, _, variants)) in STYLE_TOKENS.iter().enumerate() {
            let value = style.get(key).and_then(Value::as_str).unwrap_or("");
            indexes[i] = variants.iter().position(|v| *v == value).unwrap_or(0);
        }
        let position = get("waybar", "position")
            .and_then(Value::as_str)
            .unwrap_or("top");
        Settings {
            russian: get("hyprland", "kb_layout")
                .and_then(Value::as_str)
                .is_some_and(|l| l.contains("ru")),
            position: POSITIONS.iter().position(|p| *p == position).unwrap_or(0),
            style: indexes,
            programs,
        }
    }

    fn len(&self) -> usize {
        Self::SWITCHES + self.programs.len()
    }

    /// Записать настройки в райс.
    fn apply_to(&self, spec: &mut RiceSpec) {
        let layout = if self.russian { "us,ru" } else { "us" };
        set(spec, "hyprland", "kb_layout", layout.into());
        set(spec, "waybar", "position", POSITIONS[self.position].into());
        let mut style = serde_yaml::Mapping::new();
        for (i, (key, _, variants)) in STYLE_TOKENS.iter().enumerate() {
            style.insert((*key).into(), variants[self.style[i]].into());
        }
        if let Ok(style) = serde_yaml::from_value(Value::Mapping(style)) {
            spec.theme.style = style;
        }
        for program in &self.programs {
            let id = &program.choices[program.current];
            if id.is_empty() {
                spec.components.remove(program.role);
            } else {
                spec.components.insert(program.role.to_string(), id.clone());
            }
        }
    }

    /// Поменять значение строки: → вперёд по списку, ← назад.
    fn change(&mut self, row: usize, forward: bool) {
        // Как `%` в Python: + n, чтобы не уйти в минус при движении назад.
        let cycle = |i: usize, n: usize| {
            if forward {
                (i + 1) % n
            } else {
                (i + n - 1) % n
            }
        };
        match row {
            0 => self.russian = !self.russian,
            1 => self.position = cycle(self.position, POSITIONS.len()),
            r if r < Self::SWITCHES => {
                let i = r - 2;
                self.style[i] = cycle(self.style[i], STYLE_TOKENS[i].2.len());
            }
            _ => {
                let program = &mut self.programs[row - Self::SWITCHES];
                program.current = cycle(program.current, program.choices.len());
            }
        }
    }

    fn rows(&self) -> Vec<(&'static str, String)> {
        let layout = if self.russian {
            "us + ru (Alt+Shift)"
        } else {
            "только us"
        };
        let position = ["сверху", "снизу", "слева", "справа"][self.position];
        let mut rows = vec![
            ("Раскладка", layout.to_string()),
            ("Панель", position.to_string()),
        ];
        for (i, (_, label, variants)) in STYLE_TOKENS.iter().enumerate() {
            rows.push((*label, variants[self.style[i]].to_string()));
        }
        for program in &self.programs {
            let id = &program.choices[program.current];
            let value = if id.is_empty() { "нет" } else { id.as_str() };
            rows.push((program.label, value.to_string()));
        }
        rows
    }
}

/// Значение настройки для экрана: переключатель, выбор или ползунок.
fn format_value(setting: &Setting, value: &Value) -> String {
    match setting.kind() {
        SettingKind::Switch => {
            if value.as_bool() == Some(true) {
                "● вкл".into()
            } else {
                "○ выкл".into()
            }
        }
        SettingKind::Choice => format!("◀ {} ▶", show_value(value)),
        SettingKind::Number => {
            // Ползунок из 10 делений: ▰▰▰▱▱▱▱▱▱▱ 6
            let [min, max] = setting.range.unwrap_or([0.0, 1.0]);
            let v = value.as_f64().unwrap_or(min);
            let filled = (((v - min) / (max - min)) * 10.0).round() as usize;
            let bar: String = (0..10)
                .map(|i| if i < filled { '▰' } else { '▱' })
                .collect();
            format!("{bar} {}", show_value(value))
        }
        SettingKind::Other => format!("{} (в файле райса)", show_value(value)),
    }
}

/// Записать одно значение в `overrides.<module>.<key>`.
fn set(spec: &mut RiceSpec, module: &str, key: &str, value: Value) {
    spec.overrides
        .entry(module.to_string())
        .or_default()
        .insert(key.into(), value);
}

/// Программа райса с описанными настройками и их текущими значениями.
struct TuneModule {
    id: String,
    settings: Vec<Setting>,
    values: Vec<Value>,
}

/// Окно выбора своей картинки: путь вручную или из найденных.
struct ImagePick {
    input: String,
    found: Vec<std::path::PathBuf>,
    selected: usize,
    error: Option<String>,
}

impl ImagePick {
    fn new(env: &Env) -> ImagePick {
        ImagePick {
            input: String::new(),
            found: find_images(env),
            selected: 0,
            error: None,
        }
    }

    /// Что выбрано: путь из строки ввода, а если она пуста — из списка.
    fn path(&self, env: &Env) -> Option<std::path::PathBuf> {
        let typed = self.input.trim();
        if typed.is_empty() {
            self.found.get(self.selected).cloned()
        } else {
            Some(env.expand(typed))
        }
    }
}

/// Картинки в обычных местах: Pictures, Downloads, wallpapers (до 3 уровней).
fn find_images(env: &Env) -> Vec<std::path::PathBuf> {
    fn walk(dir: &std::path::Path, depth: usize, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && depth > 0 {
                walk(&path, depth - 1, out);
            } else if path.extension().is_some_and(|e| {
                matches!(
                    e.to_string_lossy().to_lowercase().as_str(),
                    "png" | "jpg" | "jpeg" | "webp"
                )
            }) {
                out.push(path);
            }
        }
    }
    let mut found = Vec::new();
    for dir in [
        "~/Pictures",
        "~/Изображения",
        "~/Downloads",
        "~/Загрузки",
        "~/wallpapers",
        "~/Wallpapers",
    ] {
        walk(&env.expand(dir), 3, &mut found);
    }
    found.sort();
    found.dedup();
    found.truncate(300);
    found
}

struct App {
    step: Step,
    checks: Vec<Check>,
    presets: Vec<RiceSpec>,
    rice: ListState,
    settings: Settings,
    setting_row: usize,
    /// Тонкая настройка: программы, выбранная программа и строка, фокус справа.
    tune: Vec<TuneModule>,
    tune_module: usize,
    tune_row: usize,
    tune_inside: bool,
    /// Что пользователь поменял: (модуль, настройка) → значение.
    tweaks: BTreeMap<(String, String), Value>,
    plan: Option<Plan>,
    /// Ошибка построения плана, показывается на последнем экране.
    error: Option<String>,
    /// Показывать картинку-превью. В консоли Linux (TTY) выключено:
    /// там 16 цветов, и вместо картинки вышла бы каша.
    show_preview: bool,
    /// Последнее превью и райс, по которому оно нарисовано (в YAML):
    /// перерисовываем, только когда райс поменялся.
    preview: Option<(String, Pixmap)>,
    /// Терминал умеет настоящие картинки: через что их показывать
    /// и картинка, уже подготовленная для него.
    picker: Option<Picker>,
    image: Option<StatefulProtocol>,
    /// Первый в списке — райс из файла пользователя.
    own: bool,
    /// Выбор своей картинки: открыт ли и что в нём.
    pick: Option<ImagePick>,
    /// Первый в списке райс собран из своей картинки.
    picked: bool,
    /// Песочница (`--root`): система не меняется, так что проблемы
    /// из проверки не мешают пройти дальше.
    sandbox: bool,
}

impl App {
    fn new(probe: &dyn Probe) -> Result<App> {
        Ok(App {
            step: Step::Checks,
            checks: check::run_all(probe),
            presets: presets::all()?,
            rice: ListState::default().with_selected(Some(0)),
            settings: Settings {
                russian: false,
                position: 0,
                style: [0; 7],
                programs: Vec::new(),
            },
            setting_row: 0,
            tune: Vec::new(),
            tune_module: 0,
            tune_row: 0,
            tune_inside: false,
            tweaks: BTreeMap::new(),
            plan: None,
            error: None,
            show_preview: false,
            preview: None,
            picker: None,
            image: None,
            own: false,
            pick: None,
            picked: false,
            sandbox: false,
        })
    }

    fn selected_rice(&self) -> &RiceSpec {
        let i = self.rice.selected().unwrap_or(0);
        &self.presets[i.min(self.presets.len() - 1)]
    }

    /// В списке райсов выбран последний пункт — «Своя картинка».
    fn on_picture_item(&self) -> bool {
        self.rice.selected() == Some(self.presets.len())
    }

    /// Окно выбора картинки: набрать путь или выбрать из найденных.
    fn handle_pick(&mut self, key: KeyCode, registry: &Registry, env: &Env) {
        let Some(pick) = &mut self.pick else {
            return;
        };
        match key {
            KeyCode::Esc => self.pick = None,
            KeyCode::Up => pick.selected = pick.selected.saturating_sub(1),
            KeyCode::Down => {
                pick.selected = (pick.selected + 1).min(pick.found.len().saturating_sub(1))
            }
            KeyCode::Backspace => {
                pick.input.pop();
            }
            KeyCode::Char(c) => pick.input.push(c),
            KeyCode::Enter => {
                let Some(path) = pick.path(env) else {
                    pick.error = Some("Наберите путь к картинке.".into());
                    return;
                };
                match presets::from_image(&path) {
                    Ok(spec) => {
                        self.pick = None;
                        self.presets.insert(0, spec);
                        self.picked = true;
                        self.own = false;
                        self.rice.select(Some(0));
                        self.settings = Settings::from_spec(self.selected_rice(), registry);
                        self.setting_row = 0;
                        self.step = Step::Settings;
                    }
                    Err(e) => pick.error = Some(format!("{e:#}")),
                }
            }
            _ => {}
        }
    }

    /// Выбранный райс с применёнными настройками.
    fn final_spec(&self) -> RiceSpec {
        let mut spec = self.selected_rice().clone();
        self.settings.apply_to(&mut spec);
        for ((module, key), value) in &self.tweaks {
            // Настройки программ, которые потом убрали из райса, не пишем:
            // иначе план споткнётся о модуль, которого нет.
            if spec.components.values().any(|id| id == module) {
                set(&mut spec, module, key, value.clone());
            }
        }
        spec
    }

    /// Собрать список программ с настройками для текущего райса.
    fn build_tune(&mut self, registry: &Registry) {
        let spec = self.final_spec();
        self.tune = spec
            .components
            .values()
            .filter_map(|id| registry.get(id))
            .filter(|m| !m.manifest.settings.is_empty())
            .map(|m| {
                let id = m.manifest.id.clone();
                let values = m
                    .manifest
                    .settings
                    .iter()
                    .map(|s| {
                        spec.overrides
                            .get(&id)
                            .and_then(|o| o.get(s.id.as_str()))
                            .cloned()
                            .unwrap_or_else(|| s.default.clone())
                    })
                    .collect();
                TuneModule {
                    id,
                    settings: m.manifest.settings.clone(),
                    values,
                }
            })
            .collect();
        // Композитор первым: у него больше всего настроек, дальше по алфавиту.
        self.tune
            .sort_by_key(|m| (m.id != spec.components["compositor"], m.id.clone()));
        self.tune_module = self.tune_module.min(self.tune.len().saturating_sub(1));
        self.tune_row = 0;
        self.tune_inside = false;
    }

    /// Обновить превью под то, что сейчас на экране.
    fn refresh_preview(&mut self, registry: &Registry, env: &Env) {
        // Пока выбирают картинку, показываем её саму: райса из неё ещё нет.
        if let Some(pick) = &self.pick {
            let path = pick.path(env);
            let key = format!("картинка {path:?}");
            if self.preview.as_ref().is_some_and(|(k, _)| *k == key) {
                return;
            }
            self.preview = path
                .and_then(|p| plove_preview::wallpaper_only(&p).ok())
                .map(|image| (key, image));
        } else {
            let spec = match self.step {
                Step::Rice if self.on_picture_item() => {
                    self.preview = None;
                    self.image = None;
                    return;
                }
                Step::Rice => self.selected_rice().clone(),
                Step::Settings | Step::Tune => self.final_spec(),
                _ => return,
            };
            let key = serde_yaml::to_string(&spec).unwrap_or_default();
            if self.preview.as_ref().is_some_and(|(k, _)| *k == key) {
                return;
            }
            // Не вышло (например, нет файла обоев) — просто без картинки.
            self.preview = plove_preview::render(&spec, registry, env)
                .ok()
                .map(|image| (key, image));
        }
        self.image = match (&self.picker, &self.preview) {
            (Some(picker), Some((_, pixmap))) => {
                // Превью непрозрачное, поэтому премультиплицированные
                // пиксели tiny-skia можно отдать как обычный RGBA.
                image::RgbaImage::from_raw(pixmap.width(), pixmap.height(), pixmap.data().to_vec())
                    .map(|rgba| picker.new_resize_protocol(rgba.into()))
            }
            _ => None,
        };
    }

    fn build_plan(&mut self, registry: &Registry, env: &Env) {
        match plan::build(self.final_spec(), registry, env) {
            Ok(plan) => {
                self.plan = Some(plan);
                self.error = None;
            }
            Err(e) => {
                self.plan = None;
                self.error = Some(format!("{e:#}"));
            }
        }
        self.step = Step::Plan;
    }

    fn handle(&mut self, key: KeyCode, registry: &Registry, env: &Env) -> Action {
        let action = self.handle_key(key, registry, env);
        if self.show_preview {
            self.refresh_preview(registry, env);
        }
        action
    }

    fn handle_key(&mut self, key: KeyCode, registry: &Registry, env: &Env) -> Action {
        if self.pick.is_some() {
            self.handle_pick(key, registry, env);
            return Action::None;
        }
        if key == KeyCode::Char('q') {
            return Action::Quit;
        }
        let back = matches!(key, KeyCode::Esc | KeyCode::Backspace | KeyCode::Left);
        match self.step {
            Step::Checks => match key {
                KeyCode::Char('f') if self.checks.iter().any(|c| !c.fix.is_empty()) => {
                    return Action::Fix
                }
                KeyCode::Enter if self.sandbox || !check::has_failures(&self.checks) => {
                    self.step = Step::Rice
                }
                KeyCode::Esc => return Action::Quit,
                _ => {}
            },
            Step::Rice => match key {
                KeyCode::Up | KeyCode::Char('k') => self.rice.select_previous(),
                KeyCode::Down | KeyCode::Char('j') => {
                    // select_next не знает длину списка, поэтому ограничиваем сами.
                    // Последний пункт — «Своя картинка».
                    let last = self.presets.len();
                    let next = self.rice.selected().map_or(0, |i| (i + 1).min(last));
                    self.rice.select(Some(next));
                }
                KeyCode::Enter | KeyCode::Right if self.on_picture_item() => {
                    self.pick = Some(ImagePick::new(env));
                }
                KeyCode::Enter | KeyCode::Right => {
                    self.settings = Settings::from_spec(self.selected_rice(), registry);
                    self.setting_row = 0;
                    self.step = Step::Settings;
                }
                _ if back => self.step = Step::Checks,
                _ => {}
            },
            Step::Settings => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.setting_row = self.setting_row.saturating_sub(1)
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.setting_row = (self.setting_row + 1).min(self.settings.len() - 1)
                }
                // ← и → меняют значение (у программ — листают список). Назад — Esc.
                KeyCode::Char(' ') | KeyCode::Right => self.settings.change(self.setting_row, true),
                KeyCode::Left => self.settings.change(self.setting_row, false),
                KeyCode::Enter => {
                    self.build_tune(registry);
                    self.step = Step::Tune;
                }
                KeyCode::Esc | KeyCode::Backspace => self.step = Step::Rice,
                _ => {}
            },
            Step::Tune if self.tune_inside => {
                let module = &mut self.tune[self.tune_module];
                let last = module.settings.len() - 1;
                match key {
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.tune_row = self.tune_row.saturating_sub(1)
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.tune_row = (self.tune_row + 1).min(last)
                    }
                    KeyCode::Left | KeyCode::Right | KeyCode::Char(' ') => {
                        let setting = &module.settings[self.tune_row];
                        let value = &mut module.values[self.tune_row];
                        *value = setting.step_value(value, key != KeyCode::Left);
                        self.tweaks
                            .insert((module.id.clone(), setting.id.clone()), value.clone());
                    }
                    KeyCode::Enter => self.build_plan(registry, env),
                    KeyCode::Esc | KeyCode::Backspace => self.tune_inside = false,
                    _ => {}
                }
            }
            Step::Tune => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.tune_module = self.tune_module.saturating_sub(1)
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.tune_module = (self.tune_module + 1).min(self.tune.len().saturating_sub(1))
                }
                KeyCode::Right if !self.tune.is_empty() => {
                    self.tune_inside = true;
                    self.tune_row = 0;
                }
                KeyCode::Enter => self.build_plan(registry, env),
                _ if back => self.step = Step::Settings,
                _ => {}
            },
            Step::Plan => match key {
                KeyCode::Enter if self.plan.is_some() => {
                    return Action::Apply(Box::new(self.final_spec()));
                }
                _ if back => self.step = Step::Tune,
                _ => {}
            },
        }
        Action::None
    }

    // ───────────────────────── отрисовка ─────────────────────────

    fn draw(&mut self, frame: &mut Frame) {
        let [header, body, footer] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .areas(frame.area());

        frame.render_widget(self.header(), header);
        match self.step {
            Step::Checks => self.draw_checks(frame, body),
            Step::Rice => self.draw_rice(frame, body),
            Step::Settings => self.draw_settings(frame, body),
            Step::Tune => self.draw_tune(frame, body),
            Step::Plan => self.draw_plan(frame, body),
        }
        frame.render_widget(Paragraph::new(self.hints()).dark_gray(), footer);
    }

    /// Заголовок с шагами: текущий выделен.
    fn header(&self) -> Paragraph<'static> {
        let mut spans = vec![
            Span::styled(" plove ", Style::new().bold().reversed()),
            Span::raw("  "),
        ];
        for (i, step) in Step::ALL.iter().enumerate() {
            let label = format!("{} {}", i + 1, step.title());
            let style = if *step == self.step {
                Style::new().bold().cyan()
            } else {
                Style::new().dark_gray()
            };
            spans.push(Span::styled(label, style));
            if i + 1 < Step::ALL.len() {
                spans.push(Span::raw("  ›  ").dark_gray());
            }
        }
        Paragraph::new(Line::from(spans)).block(Block::bordered())
    }

    fn hints(&self) -> &'static str {
        match self.step {
            Step::Checks if self.sandbox => {
                " песочница: проблемы системы не мешают · Enter дальше · q выйти"
            }
            Step::Checks if check::has_failures(&self.checks) => {
                " есть проблемы, которые нужно исправить вручную · f исправить автоматически · q выйти"
            }
            Step::Checks => " Enter дальше · f исправить автоматически · q выйти",
            Step::Rice if self.pick.is_some() => {
                " ↑↓ выбрать картинку · Enter собрать райс · Esc отмена"
            }
            Step::Rice => " ↑↓ выбрать · Enter дальше · Esc назад · q выйти",
            Step::Settings => " ↑↓ выбрать · ←→ или Пробел изменить · Enter дальше · Esc назад",
            Step::Tune if self.tune_inside => {
                " ↑↓ выбрать · ←→ изменить · Esc к списку программ · Enter показать план"
            }
            Step::Tune => " ↑↓ программа · → открыть настройки · Enter показать план · Esc назад",
            Step::Plan => " Enter применить · Esc назад · q выйти",
        }
    }

    fn draw_checks(&self, frame: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = vec![Line::raw("")];
        for c in &self.checks {
            let (mark, color) = match c.level {
                Level::Ok => ("✓", Color::Green),
                Level::Warn => ("!", Color::Yellow),
                Level::Fail => ("✗", Color::Red),
            };
            lines.push(Line::from(vec![
                Span::styled(format!(" {mark} "), Style::new().fg(color).bold()),
                Span::styled(format!("{:<24}", c.title), Style::new().bold()),
                Span::raw(c.detail.clone()),
            ]));
        }
        if self.checks.iter().any(|c| !c.fix.is_empty()) {
            lines.push(Line::raw(""));
            lines.push(Line::raw(" Часть проблем plove может исправить сам: нажмите f.").yellow());
        }
        let text = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(Block::bordered().title(" Проверка системы "));
        frame.render_widget(text, area);
    }

    fn draw_rice(&mut self, frame: &mut Frame, area: Rect) {
        let [list_area, info_area] =
            Layout::horizontal([Constraint::Length(24), Constraint::Min(30)]).areas(area);

        let items: Vec<ListItem> = self
            .presets
            .iter()
            .enumerate()
            .map(|(i, spec)| match i {
                0 if self.own => ListItem::new(format!(" {} (ваш)", spec.name)).cyan(),
                0 if self.picked => ListItem::new(format!(" {} (своя)", spec.name)).cyan(),
                _ => ListItem::new(format!(" {}", spec.name)),
            })
            .chain(std::iter::once(ListItem::new(" + Своя картинка").green()))
            .collect();
        let list = List::new(items)
            .block(Block::bordered().title(" Райсы "))
            .highlight_style(Style::new().reversed().bold())
            .highlight_symbol("›");
        frame.render_stateful_widget(list, list_area, &mut self.rice);

        if self.pick.is_some() || self.on_picture_item() {
            self.draw_pick(frame, info_area);
            return;
        }
        let spec = self.selected_rice();
        let p = &spec.theme.palette;
        let g = &spec.theme.geometry;
        let mut lines = vec![
            Line::raw(""),
            Line::raw(format!(" {}", spec.description.as_deref().unwrap_or(""))).bold(),
            Line::raw(""),
            Line::from(
                [(" фон ", p.bg), (" текст ", p.fg), (" акцент ", p.accent)]
                    .into_iter()
                    .flat_map(|(label, c)| [Span::raw(label), swatch(c, 4)])
                    .collect::<Vec<_>>(),
            ),
            Line::from(
                std::iter::once(Span::raw(" терминал "))
                    .chain(p.ansi.iter().map(|&c| swatch(c, 2)))
                    .collect::<Vec<_>>(),
            ),
            Line::raw(""),
            Line::raw(format!(
                " Шрифты: {} / {}",
                spec.theme.fonts.ui, spec.theme.fonts.mono
            )),
            Line::raw(format!(
                " Отступы {} px, рамка {} px, скругление {} px, прозрачность {}",
                g.gaps, g.border, g.radius, g.opacity
            )),
            Line::raw(""),
        ];
        if self.show_preview {
            lines.push(Line::raw(" В полном размере: plove preview").dark_gray());
        } else {
            // В TTY цвета приблизительные: консоль Linux знает только 16 цветов.
            lines.push(Line::raw(format!(" Картинка: plove preview {}", spec.name)).dark_gray());
            lines.push(Line::raw(" (в TTY цвета показываются приблизительно)").dark_gray());
        }
        let block = Block::bordered().title(format!(" {} ", spec.name));
        let inner = block.inner(info_area);
        frame.render_widget(block, info_area);
        // Сверху описание, под ним картинка на всё оставшееся место.
        let [text_area, picture_area] =
            Layout::vertical([Constraint::Length(lines.len() as u16), Constraint::Min(0)])
                .areas(inner);
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), text_area);
        self.draw_preview(frame, picture_area);
    }

    /// Свой райс из картинки: подсказка, а если окно выбора открыто —
    /// строка ввода, найденные картинки и сама выбранная картинка.
    fn draw_pick(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered().title(" Своя картинка ");
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let Some(pick) = &self.pick else {
            let text = vec![
                Line::raw(""),
                Line::raw(" Райс вокруг ваших обоев.").bold(),
                Line::raw(""),
                Line::raw(" plove возьмёт цвета из картинки: фон, текст, акцент и 16 цветов"),
                Line::raw(" терминала. Стиль, программы и тонкие настройки — на следующих шагах."),
                Line::raw(" Готовые райсы при этом не используются."),
                Line::raw(""),
                Line::raw(" Enter — выбрать картинку (PNG, JPEG, WebP).").cyan(),
            ];
            frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: false }), inner);
            return;
        };
        let [text_area, picture_area] =
            Layout::vertical([Constraint::Percentage(50), Constraint::Min(6)]).areas(inner);
        let mut lines = vec![
            Line::from(vec![
                Span::raw(" Путь: "),
                Span::styled(format!("{}▏", pick.input), Style::new().bold()),
            ]),
            Line::raw(" Наберите путь (можно с ~) или выберите ↑↓ из найденных.").dark_gray(),
        ];
        if let Some(error) = &pick.error {
            lines.push(Line::raw(format!(" {error}")).red());
        }
        lines.push(Line::raw(""));
        if pick.found.is_empty() {
            lines.push(
                Line::raw(" В ~/Pictures, ~/Downloads и ~/wallpapers картинок нет.").dark_gray(),
            );
        }
        let shown = text_area.height.saturating_sub(lines.len() as u16) as usize;
        let start = pick.selected.saturating_sub(shown.saturating_sub(1));
        for (i, path) in pick.found.iter().enumerate().skip(start).take(shown) {
            let typed = !pick.input.trim().is_empty();
            let name = env_display(path);
            lines.push(if i == pick.selected && !typed {
                Line::styled(format!(" › {name}"), Style::new().reversed().bold())
            } else {
                Line::raw(format!("   {name}"))
            });
        }
        frame.render_widget(Paragraph::new(lines), text_area);
        self.draw_preview(frame, picture_area);
    }

    /// Картинка-превью по центру `area`, с пропорциями экрана 16:9.
    ///
    /// Если терминал умеет картинки — настоящая картинка. Иначе полублоки:
    /// в одной клетке две точки, верхняя — цветом символа «▀», нижняя —
    /// цветом фона клетки.
    fn draw_preview(&mut self, frame: &mut Frame, area: Rect) {
        let Some((_, image)) = &self.preview else {
            return;
        };
        // Клетка терминала обычно вдвое выше своей ширины; точный размер
        // знает только терминал, который умеет картинки.
        let (cell_w, cell_h) = self.picker.as_ref().map_or((1, 2), |p| p.font_size());
        let (cell_w, cell_h) = (u32::from(cell_w.max(1)), u32::from(cell_h.max(1)));
        let fit_w = u32::from(area.width).min(u32::from(area.height) * cell_h * 16 / (9 * cell_w));
        let width = fit_w as u16;
        let rows = (fit_w * cell_w * 9 / (16 * cell_h)) as u16;
        if width < 16 || rows < 4 {
            return;
        }
        let x0 = area.x + (area.width - width) / 2;
        let y0 = area.y + (area.height - rows) / 2;
        if let Some(state) = &mut self.image {
            let target = Rect::new(x0, y0, width, rows);
            frame.render_stateful_widget(
                StatefulImage::default()
                    .resize(Resize::Scale(Some(image::imageops::FilterType::Triangle))),
                target,
                state,
            );
            return;
        }
        let pixels = plove_preview::thumbnail(image, width.into(), (rows * 2).into());
        let buffer = frame.buffer_mut();
        for row in 0..rows {
            for col in 0..width {
                let top = pixels[(row * 2 * width + col) as usize];
                let bottom = pixels[((row * 2 + 1) * width + col) as usize];
                buffer[(x0 + col, y0 + row)]
                    .set_char('▀')
                    .set_fg(Color::Rgb(top[0], top[1], top[2]))
                    .set_bg(Color::Rgb(bottom[0], bottom[1], bottom[2]));
            }
        }
    }

    fn draw_settings(&mut self, frame: &mut Frame, area: Rect) {
        let mut lines = vec![Line::raw("")];
        for (i, (name, value)) in self.settings.rows().iter().enumerate() {
            let selected = i == self.setting_row;
            let style = if selected {
                Style::new().reversed().bold()
            } else {
                Style::new()
            };
            lines.push(Line::from(vec![
                Span::raw(if selected { " › " } else { "   " }),
                Span::styled(format!("{name:<14}"), style),
                Span::raw("  ◀ "),
                Span::styled(value.clone(), Style::new().cyan()),
                Span::raw(" ▶"),
            ]));
        }
        lines.push(Line::raw(""));
        lines.push(
            Line::raw(" Остальное можно поменять потом в файле райса или в user.conf.").dark_gray(),
        );
        let title = format!(" Настройки: {} ", self.selected_rice().name);
        // Справа превью, если хватает ширины: видно, что меняет каждая настройка.
        if self.preview.is_some() && area.width >= 90 {
            let [list_area, picture_area] =
                Layout::horizontal([Constraint::Length(52), Constraint::Min(30)]).areas(area);
            frame.render_widget(
                Paragraph::new(lines)
                    .wrap(Wrap { trim: false })
                    .block(Block::bordered().title(title)),
                list_area,
            );
            let block = Block::bordered().title(" Как будет выглядеть ");
            let inner = block.inner(picture_area);
            frame.render_widget(block, picture_area);
            self.draw_preview(frame, inner);
        } else {
            frame.render_widget(
                Paragraph::new(lines).block(Block::bordered().title(title)),
                area,
            );
        }
    }

    fn draw_tune(&mut self, frame: &mut Frame, area: Rect) {
        let [list_area, mut info_area] =
            Layout::horizontal([Constraint::Length(22), Constraint::Min(40)]).areas(area);
        // Превью справа от настроек, если хватает ширины, иначе под ними.
        if self.preview.is_some() {
            let picture_area = if info_area.width >= 64 + 40 {
                let [settings, picture] =
                    Layout::horizontal([Constraint::Length(64), Constraint::Min(40)])
                        .areas(info_area);
                info_area = settings;
                Some(picture)
            } else if info_area.height >= 30 {
                let [settings, picture] =
                    Layout::vertical([Constraint::Min(14), Constraint::Percentage(40)])
                        .areas(info_area);
                info_area = settings;
                Some(picture)
            } else {
                None
            };
            if let Some(picture_area) = picture_area {
                let block = Block::bordered().title(" Как будет выглядеть ");
                let inner = block.inner(picture_area);
                frame.render_widget(block, picture_area);
                self.draw_preview(frame, inner);
            }
        }

        let items: Vec<ListItem> = self
            .tune
            .iter()
            .map(|m| ListItem::new(format!(" {}", m.id)))
            .collect();
        let mut state = ListState::default().with_selected(Some(self.tune_module));
        let highlight = if self.tune_inside {
            Style::new().bold().cyan()
        } else {
            Style::new().reversed().bold()
        };
        let list = List::new(items)
            .block(Block::bordered().title(" Программы "))
            .highlight_style(highlight)
            .highlight_symbol("›");
        frame.render_stateful_widget(list, list_area, &mut state);

        let Some(module) = self.tune.get(self.tune_module) else {
            let text = Paragraph::new(" У программ этого райса пока нет описанных настроек.")
                .block(Block::bordered());
            frame.render_widget(text, info_area);
            return;
        };
        // Строки по группам; выбранную держим в видимой части экрана.
        let mut lines: Vec<Line> = Vec::new();
        let mut selected_line = 0;
        let mut group = "";
        for (i, (setting, value)) in module.settings.iter().zip(&module.values).enumerate() {
            if setting.group != group {
                group = &setting.group;
                lines.push(Line::raw(""));
                lines.push(Line::raw(format!(" {group}")).bold().dark_gray());
            }
            let selected = self.tune_inside && i == self.tune_row;
            if selected {
                selected_line = lines.len();
            }
            let changed = *value != setting.default;
            let name_style = if selected {
                Style::new().reversed().bold()
            } else {
                Style::new()
            };
            lines.push(Line::from(vec![
                Span::raw(if selected { " › " } else { "   " }),
                Span::styled(format!("{:<34}", setting.title), name_style),
                Span::raw(" "),
                Span::styled(
                    format_value(setting, value),
                    if changed {
                        Style::new().yellow().bold()
                    } else {
                        Style::new().cyan()
                    },
                ),
            ]));
            if selected && !setting.about.is_empty() {
                lines.push(Line::raw(format!("     {}", setting.about)).dark_gray());
            }
        }
        let height = info_area.height.saturating_sub(2) as usize;
        let scroll = selected_line.saturating_sub(height.saturating_sub(3));
        let text = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((scroll as u16, 0))
            .block(Block::bordered().title(format!(" {} ", module.id)));
        frame.render_widget(text, info_area);
    }

    fn draw_plan(&self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered().title(" Что будет сделано ");
        let Some(plan) = &self.plan else {
            let error = self.error.clone().unwrap_or_default();
            let text = Paragraph::new(vec![
                Line::raw(""),
                Line::raw(" Не удалось построить план:").red().bold(),
                Line::raw(format!(" {error}")),
            ])
            .wrap(Wrap { trim: false })
            .block(block);
            frame.render_widget(text, area);
            return;
        };

        let mut lines = vec![Line::raw(" Пакеты").bold()];
        for p in &plan.packages {
            let (mark, note, color) = match p.installed {
                Some(true) => ("=", "уже установлен", Color::DarkGray),
                Some(false) => ("+", "будет установлен", Color::Green),
                None => ("?", "не удалось проверить", Color::Yellow),
            };
            lines.push(Line::styled(
                format!(
                    "   {mark} {:<28} {note}{}",
                    p.name,
                    if p.aur { " (AUR)" } else { "" }
                ),
                Style::new().fg(color),
            ));
        }
        lines.push(Line::raw(""));
        lines.push(Line::raw(" Конфиги").bold());
        for f in &plan.files {
            let (mark, note, color) = match f.status {
                FileStatus::Create => ("+", "новый", Color::Green),
                FileStatus::Update => (
                    "~",
                    "изменится (старый сохранится для отката)",
                    Color::Yellow,
                ),
                FileStatus::Unchanged => ("=", "без изменений", Color::DarkGray),
            };
            let path = env_display(&f.path);
            lines.push(Line::styled(
                format!("   {mark} {path:<40} {note}"),
                Style::new().fg(color),
            ));
        }
        for warning in &plan.warnings {
            lines.push(Line::raw(""));
            lines.push(Line::raw(format!(" ! {warning}")).yellow());
        }
        lines.push(Line::raw(""));
        let last = if plan.is_noop() {
            " Всё уже применено. Enter — выйти без изменений."
        } else {
            " Enter — применить. Пакеты ставятся через sudo pacman, пароль спросят после мастера."
        };
        lines.push(Line::raw(last).cyan());
        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .block(block),
            area,
        );
    }
}

/// Цветной прямоугольник шириной `width` клеток.
fn swatch(color: SpecColor, width: usize) -> Span<'static> {
    Span::styled(
        " ".repeat(width),
        Style::new().bg(Color::Rgb(color.r, color.g, color.b)),
    )
}

/// Путь с `~` вместо домашней папки, как в CLI.
fn env_display(path: &std::path::Path) -> String {
    match std::env::var_os("HOME") {
        Some(home) => match path.strip_prefix(&home) {
            Ok(rest) => format!("~/{}", rest.display()),
            Err(_) => path.display().to_string(),
        },
        None => path.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    /// Система, где всё в порядке.
    struct Healthy;

    impl Probe for Healthy {
        fn is_root(&self) -> bool {
            false
        }
        fn user(&self) -> String {
            "val".into()
        }
        fn groups(&self) -> Vec<String> {
            vec!["wheel".into(), "seat".into()]
        }
        fn command_exists(&self, name: &str) -> bool {
            matches!(name, "pacman" | "sudo" | "seatd" | "paru")
        }
        fn service_active(&self, _: &str) -> bool {
            true
        }
        fn service_enabled(&self, _: &str) -> bool {
            true
        }
        fn path_exists(&self, path: &str) -> bool {
            path == "/dev/dri/card0"
        }
        fn online(&self) -> bool {
            true
        }
    }

    /// ← в настройках меняет значение, а не уводит на прошлый экран.
    #[test]
    fn left_arrow_toggles_setting() {
        let registry = Registry::builtin().unwrap();
        let env = Env::sandbox(std::env::temp_dir().join("plove-tui-left"));
        let mut app = App::new(&Healthy).unwrap();
        app.handle(KeyCode::Enter, &registry, &env);
        app.handle(KeyCode::Enter, &registry, &env);
        assert!(matches!(app.step, Step::Settings));
        let before = app.settings.russian;
        app.handle(KeyCode::Left, &registry, &env);
        assert!(matches!(app.step, Step::Settings));
        assert_ne!(app.settings.russian, before);
        app.handle(KeyCode::Esc, &registry, &env);
        assert!(matches!(app.step, Step::Rice));
    }

    /// Программы листаются по кругу в обе стороны, а «нет» убирает роль из райса.
    #[test]
    fn programs_can_be_switched() {
        let registry = Registry::builtin().unwrap();
        let mut spec = presets::get("tokyo-night").unwrap().unwrap();
        let mut settings = Settings::from_spec(&spec, &registry);
        let launcher = Settings::SWITCHES
            + settings
                .programs
                .iter()
                .position(|p| p.role == "launcher")
                .unwrap();
        let widgets = Settings::SWITCHES
            + settings
                .programs
                .iter()
                .position(|p| p.role == "widgets")
                .unwrap();
        settings.change(launcher, true);
        settings.change(launcher, true);
        settings.change(launcher, false);
        // eww стоит первым после «нет»: шаг назад — без виджетов.
        settings.change(widgets, false);
        settings.apply_to(&mut spec);
        assert_eq!(spec.components["launcher"], "rofi");
        assert!(!spec.components.contains_key("widgets"));
    }

    /// Числа двигаются шагом и не выходят за границы, списки идут по кругу.
    #[test]
    fn setting_steps_stay_in_range() {
        let registry = Registry::builtin().unwrap();
        let hypr = &registry.get("hyprland").unwrap().manifest;
        let find = |id: &str| hypr.settings.iter().find(|s| s.id == id).unwrap();
        let blur = find("blur_size");
        let mut v = blur.default.clone();
        for _ in 0..40 {
            v = blur.step_value(&v, true);
        }
        assert_eq!(v, Value::from(20));
        let dim = find("dim_strength");
        assert_eq!(dim.step_value(&Value::from(0.15), true), Value::from(0.2));
        let layout = find("layout");
        assert_eq!(
            layout.step_value(&Value::from("dwindle"), false),
            Value::from("master")
        );
        assert!(blur.check(&Value::from(99)).is_err());
        assert_eq!(format_value(blur, &Value::from(6)), "▰▰▰▱▱▱▱▱▱▱ 6");
    }

    #[test]
    fn settings_roundtrip() {
        let registry = Registry::builtin().unwrap();
        let mut spec = presets::get("gruvbox").unwrap().unwrap();
        let mut settings = Settings::from_spec(&spec, &registry);
        settings.russian = true;
        settings.position = 2;
        settings.style = [2, 2, 3, 3, 0, 1, 2];
        settings.apply_to(&mut spec);
        assert_eq!(Settings::from_spec(&spec, &registry), settings);
    }

    /// Пройти мастер клавишами от начала до конца и нарисовать каждый экран.
    #[test]
    fn walk_through_the_wizard() {
        let home = std::env::temp_dir().join("plove-tui-test");
        let env = Env::sandbox(home);
        let registry = Registry::builtin().unwrap();
        let mut app = App::new(&Healthy).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();

        let keys = [
            KeyCode::Enter,
            KeyCode::Down,
            KeyCode::Enter,
            KeyCode::Char(' '),
            KeyCode::Enter,
            KeyCode::Right,
            KeyCode::Down,
            KeyCode::Right,
            KeyCode::Enter,
        ];
        for key in keys {
            terminal.draw(|f| app.draw(f)).unwrap();
            assert!(matches!(app.handle(key, &registry, &env), Action::None));
        }
        assert_eq!(app.step, Step::Plan);
        terminal.draw(|f| app.draw(f)).unwrap();
        let Action::Apply(spec) = app.handle(KeyCode::Enter, &registry, &env) else {
            panic!("ожидалось применение");
        };
        // Второй по алфавиту райс — gruvbox, и раскладку мы переключили пробелом.
        assert_eq!(spec.name, "gruvbox");
        // На экране тонкой настройки включили затемнение неактивных окон.
        assert_eq!(
            spec.overrides["hyprland"]["dim_inactive"],
            Value::Bool(true)
        );
        let gruvbox = presets::get("gruvbox").unwrap().unwrap();
        assert!(
            Settings::from_spec(&spec, &registry).russian
                == !Settings::from_spec(&gruvbox, &registry).russian
        );
    }

    #[test]
    fn preview_is_drawn() {
        let env = Env::sandbox(std::env::temp_dir().join("plove-tui-preview"));
        let registry = Registry::builtin().unwrap();
        let mut app = App::new(&Healthy).unwrap();
        app.show_preview = true;
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
        for key in [KeyCode::Enter, KeyCode::Enter] {
            app.handle(key, &registry, &env);
        }
        assert_eq!(app.step, Step::Settings);
        assert!(app.preview.is_some());
        terminal.draw(|f| app.draw(f)).unwrap();
        let screen = format!("{}", terminal.backend());
        if std::env::var_os("PLOVE_SHOW_SCREEN").is_some() {
            println!("{screen}");
        }
        assert!(screen.contains('▀'));
        // Смена настройки перерисовывает превью.
        let before = app.preview.as_ref().unwrap().0.clone();
        app.handle(KeyCode::Down, &registry, &env);
        app.handle(KeyCode::Right, &registry, &env);
        assert_ne!(app.preview.as_ref().unwrap().0, before);
    }

    /// Свой райс из картинки: найти её в ~/Pictures, собрать райс и
    /// пройти мастер до конца.
    #[test]
    fn rice_from_own_picture() {
        let home = std::env::temp_dir().join("plove-tui-picture");
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(home.join("Pictures")).unwrap();
        let palette = presets::get("gruvbox").unwrap().unwrap().theme.palette;
        let png = plove_core::wallpaper::generate_png(plove_core::spec::Pattern::Waves, &palette)
            .unwrap();
        std::fs::write(home.join("Pictures/sunset.png"), png).unwrap();
        let env = Env::sandbox(home.clone());
        let registry = Registry::builtin().unwrap();
        let mut app = App::new(&Healthy).unwrap();
        app.show_preview = true;
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();

        app.handle(KeyCode::Enter, &registry, &env);
        // Вниз до последнего пункта — «Своя картинка».
        for _ in 0..app.presets.len() {
            app.handle(KeyCode::Down, &registry, &env);
        }
        assert!(app.on_picture_item());
        app.handle(KeyCode::Enter, &registry, &env);
        let pick = app.pick.as_ref().unwrap();
        assert_eq!(pick.found, vec![home.join("Pictures/sunset.png")]);
        // Пока выбирают, в превью сама картинка.
        assert!(app.preview.is_some());
        terminal.draw(|f| app.draw(f)).unwrap();
        // q в окне выбора — просто буква пути, а не выход.
        assert!(matches!(
            app.handle(KeyCode::Char('q'), &registry, &env),
            Action::None
        ));
        app.handle(KeyCode::Backspace, &registry, &env);

        app.handle(KeyCode::Enter, &registry, &env);
        assert_eq!(app.step, Step::Settings);
        assert!(app.picked);
        let spec = app.final_spec();
        assert_eq!(spec.name, "sunset");
        assert_eq!(spec.components["wallpaper"], "swaybg");
        for key in [KeyCode::Enter, KeyCode::Enter] {
            terminal.draw(|f| app.draw(f)).unwrap();
            app.handle(key, &registry, &env);
        }
        assert!(app.plan.is_some(), "{:?}", app.error);
    }
}

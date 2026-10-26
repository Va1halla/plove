//! TUI-мастер: `plove` без аргументов.
//!
//! Четыре экрана по порядку:
//!
//! ```text
//! 1. Проверка   — готова ли система (то же, что plove doctor)
//! 2. Райс       — выбор готового райса
//! 3. Настройки  — раскладка, положение и вид панели
//! 4. План       — что будет установлено и записано
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
use plove_core::modules::Registry;
use plove_core::plan::{self, FileStatus, Plan};
use plove_core::presets;
use plove_core::spec::{Color as SpecColor, RiceSpec};
use plove_core::system::Env;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};
use serde_yaml::Value;

/// Чем закончился мастер.
pub enum Outcome {
    /// Пользователь подтвердил план: применить этот райс.
    Apply(Box<RiceSpec>),
    Quit,
}

/// Запустить мастер в текущем терминале.
pub fn run(registry: &Registry, env: &Env) -> Result<Outcome> {
    let mut app = App::new(&LiveProbe)?;
    let mut terminal = ratatui::init();
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
            Action::Apply(spec) => return Ok(Outcome::Apply(spec)),
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
    Plan,
}

impl Step {
    const ALL: [Step; 4] = [Step::Checks, Step::Rice, Step::Settings, Step::Plan];

    fn title(self) -> &'static str {
        match self {
            Step::Checks => "Проверка",
            Step::Rice => "Райс",
            Step::Settings => "Настройки",
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

/// Настройки, которые мастер позволяет поменять поверх готового райса.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Settings {
    /// Раскладка us,ru вместо us.
    russian: bool,
    bar_bottom: bool,
    bar_floating: bool,
}

impl Settings {
    const ROWS: usize = 3;

    /// Прочитать текущие значения из overrides райса.
    fn from_spec(spec: &RiceSpec) -> Settings {
        let get = |module: &str, key: &str| spec.overrides.get(module).and_then(|m| m.get(key));
        Settings {
            russian: get("hyprland", "kb_layout")
                .and_then(Value::as_str)
                .is_some_and(|l| l.contains("ru")),
            bar_bottom: get("waybar", "position").and_then(Value::as_str) == Some("bottom"),
            bar_floating: get("waybar", "floating")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }
    }

    /// Записать настройки в overrides райса.
    fn apply_to(self, spec: &mut RiceSpec) {
        let layout = if self.russian { "us,ru" } else { "us" };
        let position = if self.bar_bottom { "bottom" } else { "top" };
        set(spec, "hyprland", "kb_layout", layout.into());
        set(spec, "waybar", "position", position.into());
        set(spec, "waybar", "floating", self.bar_floating.into());
    }

    fn toggle(&mut self, row: usize) {
        match row {
            0 => self.russian = !self.russian,
            1 => self.bar_bottom = !self.bar_bottom,
            _ => self.bar_floating = !self.bar_floating,
        }
    }

    fn rows(self) -> [(&'static str, &'static str); Self::ROWS] {
        [
            (
                "Раскладка",
                if self.russian {
                    "us + ru (Alt+Shift)"
                } else {
                    "только us"
                },
            ),
            (
                "Панель",
                if self.bar_bottom {
                    "снизу"
                } else {
                    "сверху"
                },
            ),
            (
                "Вид панели",
                if self.bar_floating {
                    "плавающая"
                } else {
                    "вплотную к краю"
                },
            ),
        ]
    }
}

/// Записать одно значение в `overrides.<module>.<key>`.
fn set(spec: &mut RiceSpec, module: &str, key: &str, value: Value) {
    spec.overrides
        .entry(module.to_string())
        .or_default()
        .insert(key.into(), value);
}

struct App {
    step: Step,
    checks: Vec<Check>,
    presets: Vec<RiceSpec>,
    rice: ListState,
    settings: Settings,
    setting_row: usize,
    plan: Option<Plan>,
    /// Ошибка построения плана, показывается на последнем экране.
    error: Option<String>,
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
                bar_bottom: false,
                bar_floating: false,
            },
            setting_row: 0,
            plan: None,
            error: None,
        })
    }

    fn selected_rice(&self) -> &RiceSpec {
        &self.presets[self.rice.selected().unwrap_or(0)]
    }

    /// Выбранный райс с применёнными настройками.
    fn final_spec(&self) -> RiceSpec {
        let mut spec = self.selected_rice().clone();
        self.settings.apply_to(&mut spec);
        spec
    }

    fn handle(&mut self, key: KeyCode, registry: &Registry, env: &Env) -> Action {
        if key == KeyCode::Char('q') {
            return Action::Quit;
        }
        let back = matches!(key, KeyCode::Esc | KeyCode::Backspace | KeyCode::Left);
        match self.step {
            Step::Checks => match key {
                KeyCode::Char('f') if self.checks.iter().any(|c| !c.fix.is_empty()) => {
                    return Action::Fix
                }
                KeyCode::Enter if !check::has_failures(&self.checks) => self.step = Step::Rice,
                KeyCode::Esc => return Action::Quit,
                _ => {}
            },
            Step::Rice => match key {
                KeyCode::Up | KeyCode::Char('k') => self.rice.select_previous(),
                KeyCode::Down | KeyCode::Char('j') => {
                    // select_next не знает длину списка, поэтому ограничиваем сами.
                    let last = self.presets.len() - 1;
                    let next = self.rice.selected().map_or(0, |i| (i + 1).min(last));
                    self.rice.select(Some(next));
                }
                KeyCode::Enter | KeyCode::Right => {
                    self.settings = Settings::from_spec(self.selected_rice());
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
                    self.setting_row = (self.setting_row + 1).min(Settings::ROWS - 1)
                }
                // Все настройки сейчас — переключатели с двумя значениями,
                // поэтому и ←, и → просто меняют значение. Назад — Esc.
                KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right => {
                    self.settings.toggle(self.setting_row)
                }
                KeyCode::Enter => {
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
                KeyCode::Esc | KeyCode::Backspace => self.step = Step::Rice,
                _ => {}
            },
            Step::Plan => match key {
                KeyCode::Enter if self.plan.is_some() => {
                    return Action::Apply(Box::new(self.final_spec()))
                }
                _ if back => self.step = Step::Settings,
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
            Step::Checks if check::has_failures(&self.checks) => {
                " есть проблемы, которые нужно исправить вручную · f исправить автоматически · q выйти"
            }
            Step::Checks => " Enter дальше · f исправить автоматически · q выйти",
            Step::Rice => " ↑↓ выбрать · Enter дальше · Esc назад · q выйти",
            Step::Settings => " ↑↓ выбрать · ←→ или Пробел изменить · Enter показать план · Esc назад",
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
            .map(|spec| ListItem::new(format!(" {}", spec.name)))
            .collect();
        let list = List::new(items)
            .block(Block::bordered().title(" Готовые райсы "))
            .highlight_style(Style::new().reversed().bold())
            .highlight_symbol("›");
        frame.render_stateful_widget(list, list_area, &mut self.rice);

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
            Line::raw(format!(" Картинка: plove preview {}", spec.name)).dark_gray(),
        ];
        // В TTY цвета будут приблизительными: консоль Linux знает только 16 цветов.
        lines.push(Line::raw(" (в TTY цвета показываются приблизительно)").dark_gray());
        let info = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(Block::bordered().title(format!(" {} ", spec.name)));
        frame.render_widget(info, info_area);
    }

    fn draw_settings(&self, frame: &mut Frame, area: Rect) {
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
                Span::styled(*value, Style::new().cyan()),
                Span::raw(" ▶"),
            ]));
        }
        lines.push(Line::raw(""));
        lines.push(
            Line::raw(" Остальное можно поменять потом в файле райса или в user.conf.").dark_gray(),
        );
        let title = format!(" Настройки: {} ", self.selected_rice().name);
        frame.render_widget(
            Paragraph::new(lines).block(Block::bordered().title(title)),
            area,
        );
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
                format!("   {mark} {:<28} {note}", p.name),
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
            matches!(name, "pacman" | "sudo" | "seatd")
        }
        fn service_active(&self, _: &str) -> bool {
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

    #[test]
    fn settings_roundtrip() {
        let mut spec = presets::get("gruvbox").unwrap().unwrap();
        let settings = Settings {
            russian: true,
            bar_bottom: true,
            bar_floating: true,
        };
        settings.apply_to(&mut spec);
        assert_eq!(Settings::from_spec(&spec), settings);
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
        assert!(
            Settings::from_spec(&spec).russian
                == !Settings::from_spec(&presets::get("gruvbox").unwrap().unwrap()).russian
        );
    }
}

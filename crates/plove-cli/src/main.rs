//! Командная строка plove.
//!
//! Здесь только разбор аргументов и красивый вывод. Вся работа
//! делается в plove-core: CLI — тонкий клиент над ядром.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use plove_core::apply::{self, ApplyOptions};
use plove_core::check::{self, Check, Level, LiveProbe};
use plove_core::modules::Registry;
use plove_core::palette;
use plove_core::plan::{self, FileStatus, Plan};
use plove_core::presets;
use plove_core::spec::{Color, RiceSpec, Wallpaper};
use plove_core::state;
use plove_core::system::Env;

/// Конструктор райсов для Arch Linux.
// clap строит парсер аргументов прямо из этой структуры:
// поля становятся флагами, комментарии `///` — текстом справки (`plove --help`).
#[derive(Parser)]
#[command(name = "plove", version)]
struct Cli {
    /// Песочница: считать эту папку домашней, не ставить пакеты и не трогать систему
    #[arg(long, global = true, value_name = "ПАПКА")]
    root: Option<PathBuf>,

    /// Без команды открывается мастер: проверка системы, выбор райса, план
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Показать, что изменит райс, ничего не меняя
    Plan {
        /// Встроенный райс (см. `plove presets`) или путь к файлу .yaml
        rice: String,
    },
    /// Применить райс
    Apply {
        /// Встроенный райс (см. `plove presets`) или путь к файлу .yaml
        rice: String,
        /// Не спрашивать подтверждение
        #[arg(short, long)]
        yes: bool,
        /// Не ставить пакеты
        #[arg(long)]
        skip_packages: bool,
    },
    /// Открыть свой райс в мастере, поменять и сохранить обратно в файл.
    /// Если файла нет, мастер начнёт с готовых райсов и сохранит выбранный туда
    Edit {
        /// Файл райса .yaml
        file: PathBuf,
    },
    /// Быстро сменить райс: без вопросов и без установки пакетов.
    /// Если райсу нужны программы, которых нет, подскажет `plove apply`
    Switch {
        /// Встроенный райс (см. `plove presets`) или путь к файлу .yaml
        rice: String,
    },
    /// Откатить последнее применение
    Rollback {
        /// Не спрашивать подтверждение
        #[arg(short, long)]
        yes: bool,
    },
    /// Показать историю применений
    History,
    /// Показать встроенные райсы
    Presets,
    /// Нарисовать превью райса в PNG
    Preview {
        /// Встроенный райс (см. `plove presets`) или путь к файлу .yaml
        rice: String,
        /// Куда сохранить картинку (по умолчанию <имя райса>.png)
        #[arg(short, long, value_name = "ФАЙЛ")]
        output: Option<PathBuf>,
    },
    /// Сделать свой райс из картинки: цвета подбираются по обоям
    New {
        /// Обои: PNG, JPEG или WebP
        image: PathBuf,
        /// Взять шрифты, стиль и программы из готового райса. Без него —
        /// нейтральная основа, стиль потом выбирается в мастере (plove edit)
        #[arg(long)]
        base: Option<String>,
        /// Куда сохранить райс (по умолчанию <имя картинки>.yaml)
        #[arg(short, long, value_name = "ФАЙЛ")]
        output: Option<PathBuf>,
        #[command(flatten)]
        colors: ColorArgs,
    },
    /// Заново подобрать цвета райса по его обоям, не трогая остальное
    Recolor {
        /// Файл райса .yaml с обоями-картинкой
        rice: PathBuf,
        #[command(flatten)]
        colors: ColorArgs,
    },
    /// Проверить, готова ли система к райсу
    Doctor {
        /// Исправить то, что можно исправить автоматически
        #[arg(long)]
        fix: bool,
    },
    /// Проверить свой виджет: выполнить его команду и показать, что увидит панель
    Widget {
        /// Райс, в котором описан виджет (файл .yaml)
        rice: String,
        /// Имя виджета из раздела widgets
        id: String,
    },
    /// Показать встроенные модули, а с именем — все настройки модуля
    Modules {
        /// Модуль, например hyprland
        id: Option<String>,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // `{:#}` печатает всю цепочку причин: «ошибка в файле… : версия 2 не поддерживается».
            eprintln!("ошибка: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    // Окружение создаём только для команд, которые читают или меняют систему:
    // `presets`, `modules` и `preview` работают и без него (в том числе от root).
    let root = cli.root;
    let system_env = || match &root {
        Some(root) => Ok(Env::sandbox(root.clone())),
        None => Env::real(),
    };
    let registry = Registry::builtin()?;

    let Some(command) = cli.command else {
        // `plove` без аргументов: мастер в терминале. Сам он ничего не
        // применяет, а возвращает выбранный райс, и применяем мы его здесь,
        // уже в обычном терминале: вывод pacman и вопрос пароля sudo
        // внутри полноэкранного интерфейса не поместились бы.
        return wizard(&registry, &system_env()?, None);
    };

    match command {
        Command::Edit { file } => wizard(&registry, &system_env()?, Some(&file))?,
        Command::Plan { rice } => {
            let env = system_env()?;
            let plan = plan::build(load_rice(&rice)?, &registry, &env)?;
            print_plan(&plan, &env);
        }
        Command::Apply {
            rice,
            yes,
            skip_packages,
        } => {
            let env = system_env()?;
            if !env.sandbox {
                preflight()?;
            }
            let plan = plan::build(load_rice(&rice)?, &registry, &env)?;
            print_plan(&plan, &env);
            if plan.is_noop() || !(yes || confirm("Применить?")?) {
                return Ok(());
            }
            apply_and_report(&plan, &env, skip_packages, yes)?;
        }
        Command::Switch { rice } => {
            let env = system_env()?;
            let plan = plan::build(load_rice(&rice)?, &registry, &env)?;
            // `Some(false)` — пакет точно не установлен; `None` (не Arch,
            // песочница) не мешает: проверить нечем.
            let missing: Vec<&str> = plan
                .packages
                .iter()
                .filter(|p| p.installed == Some(false))
                .map(|p| p.name.as_str())
                .collect();
            if !missing.is_empty() {
                bail!(
                    "для «{rice}» не хватает пакетов: {}. Поставьте их: plove apply {rice}",
                    missing.join(", ")
                );
            }
            for warning in &plan.warnings {
                println!("! {warning}");
            }
            if plan.is_noop() {
                println!("«{}» уже применён.", plan.spec.name);
                return Ok(());
            }
            apply_and_report(&plan, &env, true, false)?;
        }
        Command::Rollback { yes } => {
            let env = system_env()?;
            let Some(last) = state::list(&env)?.pop() else {
                println!("Откатывать нечего: история пуста.");
                return Ok(());
            };
            println!(
                "Откатить поколение #{} (райс «{}»), файлов: {}?",
                last.number,
                last.record.spec_name,
                last.record.files.len()
            );
            if !(yes || confirm("Продолжить?")?) {
                return Ok(());
            }
            if let Some(generation) = state::rollback(&env)? {
                let session = (!env.sandbox)
                    .then(plove_core::system::graphical_session)
                    .flatten();
                if let Some(session) = session {
                    for command in &generation.record.reloads {
                        plove_core::system::run_reload(command, &session);
                    }
                }
                println!("Откачено поколение #{}.", generation.number);
            }
        }
        Command::History => {
            let env = system_env()?;
            let generations = state::list(&env)?;
            if generations.is_empty() {
                println!("История пуста.");
            }
            for g in generations {
                println!(
                    "#{:<4} райс «{}», файлов: {}",
                    g.number,
                    g.record.spec_name,
                    g.record.files.len()
                );
            }
        }
        Command::Presets => {
            for spec in presets::all()? {
                let description = spec.description.as_deref().unwrap_or("");
                println!("{:<18} {description}", spec.name);
            }
            println!("\nПрименить: plove apply <имя>. Посмотреть: plove preview <имя>");
        }
        Command::Preview { rice, output } => {
            let spec = load_rice(&rice)?;
            let output = output.unwrap_or_else(|| PathBuf::from(format!("{}.png", spec.name)));
            // Превью только читает файлы, поэтому ему хватает «песочницы» в домашней папке.
            let home = std::env::var_os("HOME").unwrap_or_default();
            let env = Env::sandbox(home.into());
            let png = plove_preview::render_png(&spec, &registry, &env)?;
            std::fs::write(&output, png)?;
            println!("Превью сохранено в {}", output.display());
        }
        Command::New {
            image,
            base,
            output,
            colors,
        } => new_from_wallpaper(&image, base.as_deref(), output, colors.options()?)?,
        Command::Recolor { rice, colors } => recolor(&rice, &Env::real()?, colors.options()?)?,
        Command::Doctor { fix } => doctor(fix)?,
        Command::Widget { rice, id } => {
            let spec = load_rice(&rice)?;
            let widget = spec.widgets.get(&id).with_context(|| {
                let known: Vec<&str> = spec.widgets.keys().map(String::as_str).collect();
                format!("в райсе нет виджета «{id}». Есть: {}", known.join(", "))
            })?;
            println!("$ {}", widget.command);
            let output = std::process::Command::new("sh")
                .args(["-c", &widget.command])
                .output()
                .context("не удалось запустить sh")?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Waybar показывает последнюю строку вывода.
            let value = stdout.lines().last().unwrap_or("").trim();
            if !output.status.success() {
                println!(
                    "! команда завершилась с ошибкой: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                );
            }
            let text = widget.text.replace("{value}", value);
            println!("На панели: {} {text}", widget.icon);
            println!("Обновляется раз в {} с", widget.every);
        }
        Command::Modules { id: Some(id) } => {
            let module = registry
                .get(&id)
                .with_context(|| format!("модуль «{id}» не найден, список: plove modules"))?;
            print_settings(module);
        }
        Command::Modules { id: None } => {
            for module in registry.iter() {
                let m = &module.manifest;
                let packages: Vec<String> = m
                    .packages
                    .iter()
                    .cloned()
                    .chain(m.aur.iter().map(|p| format!("{p} (AUR)")))
                    .collect();
                let settings = match m.settings.len() {
                    0 => String::new(),
                    n => format!("  настроек: {n}"),
                };
                println!(
                    "{:<16} {:<15} {}{settings}",
                    m.id,
                    m.role,
                    packages.join(", ")
                );
            }
            println!("\nНастройки модуля: plove modules <модуль>");
        }
    }
    Ok(())
}

/// Настройки модуля по группам: что делает, какие значения можно, что по умолчанию.
fn print_settings(module: &plove_core::modules::Module) {
    use plove_core::modules::SettingKind;
    let m = &module.manifest;
    println!("{} ({})\n", m.id, m.role);
    if m.settings.is_empty() {
        // Старые модули: настройки без описаний.
        for (key, value) in &m.defaults {
            let value = plove_core::modules::show_value(value);
            println!("  {:<22} {value}", key.as_str().unwrap_or("?"));
        }
        return;
    }
    let mut group = "";
    for s in &m.settings {
        if s.group != group {
            group = &s.group;
            println!("{group}");
        }
        let default = plove_core::modules::show_value(&s.default);
        let values = match s.kind() {
            SettingKind::Switch => "вкл/выкл".to_string(),
            SettingKind::Choice => s.choices.join(" | "),
            SettingKind::Number => {
                let [min, max] = s.range.unwrap_or_default();
                format!("{min}…{max}")
            }
            SettingKind::Other => "в файле райса".to_string(),
        };
        println!("  {:<22} {}", s.id, s.title);
        println!("  {:<22} {values}, по умолчанию {default}", "");
        if !s.about.is_empty() {
            println!("  {:<22} {}", "", s.about);
        }
    }
    println!(
        "\nМенять в файле райса: overrides: {{ {}: {{ ключ: значение }} }}",
        m.id
    );
}

/// `plove doctor`: показать все проверки и, с `--fix`, починить что можно.
/// Применить план и рассказать, что получилось.
fn apply_and_report(plan: &Plan, env: &Env, skip_packages: bool, no_confirm: bool) -> Result<()> {
    let options = ApplyOptions {
        skip_packages,
        no_confirm,
    };
    let report = apply::apply(plan, env, &options)?;
    println!();
    match report.generation {
        Some(n) => println!(
            "Готово: записано файлов {}, поколение #{n}. Откатить: plove rollback",
            report.files_written
        ),
        None => println!("Готово."),
    }
    if report.reloads_skipped {
        println!("Графическая сессия не запущена: настройки подхватятся при старте Hyprland.");
        println!("Запустить: start-hyprland");
    }
    for command in report.failed_reloads {
        println!("  не сработало «{command}» (программа, скорее всего, не запущена)");
    }
    Ok(())
}

fn doctor(fix: bool) -> Result<()> {
    let checks = check::run_all(&LiveProbe);
    for c in &checks {
        print_check(c);
    }
    let fixable: Vec<&Check> = checks.iter().filter(|c| !c.fix.is_empty()).collect();
    if fixable.is_empty() {
        if !check::has_failures(&checks) {
            println!("\nСистема готова.");
        }
        return Ok(());
    }
    if !fix {
        println!("\nЧасть проблем можно исправить автоматически: plove doctor --fix");
        return Ok(());
    }
    println!();
    for c in &fixable {
        println!("Исправляю: {}", c.title);
        for command in &c.fix {
            println!("  $ {}", command.join(" "));
        }
        check::apply_fix(c)?;
    }
    if fixable.iter().any(|c| c.needs_relogin) {
        println!("\nГотово. Выйдите из системы (exit) и войдите заново, чтобы изменения групп вступили в силу.");
    } else {
        println!("\nГотово.");
    }
    Ok(())
}

/// Проверка системы перед `apply`: при серьёзных проблемах останавливаемся,
/// о мелких предупреждаем.
fn preflight() -> Result<()> {
    let checks = check::run_all(&LiveProbe);
    let problems: Vec<&Check> = checks.iter().filter(|c| c.level != Level::Ok).collect();
    if problems.is_empty() {
        return Ok(());
    }
    println!("Проверка системы:");
    for c in &problems {
        print_check(c);
    }
    if check::has_failures(&checks) {
        bail!("система не готова. Подробнее: plove doctor");
    }
    if problems.iter().any(|c| !c.fix.is_empty()) {
        println!("Исправить автоматически: plove doctor --fix");
    }
    println!();
    Ok(())
}

fn print_check(c: &Check) {
    let mark = match c.level {
        Level::Ok => "✓",
        Level::Warn => "!",
        Level::Fail => "✗",
    };
    println!("  {mark} {:<24} {}", c.title, c.detail);
}

/// Райс по имени встроенного или по пути к файлу.
/// Если файл с таким именем есть, он важнее встроенного райса.
/// `plove new`: взять готовый райс и перекрасить его в цвета картинки.
fn new_from_wallpaper(
    image: &Path,
    base: Option<&str>,
    output: Option<PathBuf>,
    options: palette::Options,
) -> Result<()> {
    let mut spec = presets::from_image_with(image, options)?;
    if let Some(base) = base {
        let mut from = load_rice(base)?;
        from.description = Some(format!("Цвета из обоев, остальное из {}", from.name));
        from.name = spec.name;
        from.theme.palette = spec.theme.palette;
        from.theme.wallpaper = spec.theme.wallpaper;
        from.validate()?;
        spec = from;
    }

    let output = output.unwrap_or_else(|| PathBuf::from(format!("{}.yaml", spec.name)));
    let header = format!(
        "# Сделано командой `plove new` из {}.\n# Файл можно править руками.\n",
        image.display()
    );
    std::fs::write(&output, header + &spec.to_yaml()?)
        .with_context(|| format!("не удалось записать {}", output.display()))?;

    println!("Палитра из {}:", image.display());
    print_palette(&spec.theme.palette);
    println!("\nРайс сохранён в {}. Дальше:", output.display());
    println!(
        "  plove edit {}      # стиль, программы и настройки в мастере",
        output.display()
    );
    println!("  plove preview {}   # посмотреть", output.display());
    println!("  plove apply {}     # применить", output.display());
    Ok(())
}

/// Цветной квадратик в терминале (24-битный цвет, его понимают kitty и TTY с fbterm).
fn swatch(c: Color) -> String {
    format!("\x1b[48;2;{};{};{}m   \x1b[0m", c.r, c.g, c.b)
}

/// Мастер; с `file` — редактирование своего райса с сохранением в файл.
fn wizard(registry: &Registry, env: &Env, file: Option<&Path>) -> Result<()> {
    let own = match file {
        Some(file) if file.exists() => Some(RiceSpec::load(file)?),
        _ => None,
    };
    let plove_tui::Outcome::Apply { spec, from_picture } = plove_tui::run(registry, env, own)?
    else {
        return Ok(());
    };
    if let Some(file) = file {
        save_rice(&spec, file)?;
    } else if from_picture {
        // Райс из своей картинки есть только в мастере: сохраняем его,
        // чтобы потом поменять (`plove edit`) или применить снова.
        let file = env.expand(&format!("~/.config/plove/rices/{}.yaml", spec.name));
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("не удалось создать {}", dir.display()))?;
        }
        save_rice(&spec, &file)?;
    }
    let plan = plan::build(*spec, registry, env)?;
    print_plan(&plan, env);
    if plan.is_noop() {
        return Ok(());
    }
    apply_and_report(&plan, env, false, false)
}

/// Как подбирать цвета из обоев: `plove new` и `plove recolor`.
#[derive(clap::Args)]
struct ColorArgs {
    /// Акцент: auto (самый яркий цвет), contrast (другой оттенок, чем у
    /// большей части картинки), dominant (основной цвет) или #rrggbb
    #[arg(long, default_value = "auto")]
    accent: String,
    /// Фон: tinted (с оттенком обоев), neutral (почти серый), deep (темнее)
    #[arg(long, default_value = "tinted")]
    background: String,
    /// Цвета терминала: soft (приглушённые) или vivid (яркие)
    #[arg(long, default_value = "soft")]
    terminal: String,
}

impl ColorArgs {
    fn options(&self) -> Result<palette::Options> {
        Ok(palette::Options {
            accent: palette::Accent::parse(&self.accent)?,
            background: palette::Background::parse(&self.background)?,
            terminal: palette::Terminal::parse(&self.terminal)?,
        })
    }
}

fn print_palette(p: &plove_core::spec::Palette) {
    println!("  фон     {} {}", swatch(p.bg), p.bg.hex());
    println!("  текст   {} {}", swatch(p.fg), p.fg.hex());
    println!("  акцент  {} {}", swatch(p.accent), p.accent.hex());
    let ansi: String = p.ansi.iter().map(|&c| swatch(c)).collect();
    println!("  терминал {ansi}");
}

/// `plove recolor`: новые цвета из обоев райса, всё остальное как было.
fn recolor(file: &Path, env: &Env, options: palette::Options) -> Result<()> {
    let mut spec = RiceSpec::load(file)?;
    let Some(Wallpaper::File(wallpaper)) = &spec.theme.wallpaper else {
        bail!(
            "у райса {} обои не из файла: цвета брать не из чего",
            file.display()
        );
    };
    let image = env.expand(wallpaper);
    spec.theme.palette = palette::from_pixels_with(&palette::pixels(&image)?, options);
    spec.validate()?;
    println!("Палитра из {}:", image.display());
    print_palette(&spec.theme.palette);
    save_rice(&spec, file)?;
    println!("Применить: plove apply {}", file.display());
    Ok(())
}

/// Записать райс в файл. Старый файл остаётся рядом как `.bak`:
/// комментарии в нём при сохранении теряются, а так их можно вернуть.
fn save_rice(spec: &RiceSpec, file: &Path) -> Result<()> {
    if file.exists() {
        let mut backup = file.as_os_str().to_owned();
        backup.push(".bak");
        std::fs::copy(file, &backup)
            .with_context(|| format!("не удалось сохранить копию {}", file.display()))?;
    }
    let text = format!(
        "# Райс plove. Сохранено командой `plove edit`; применить: plove apply {}\n{}",
        file.display(),
        spec.to_yaml()?
    );
    std::fs::write(file, text)
        .with_context(|| format!("не удалось записать {}", file.display()))?;
    println!("Райс сохранён в {}", file.display());
    Ok(())
}

fn load_rice(rice: &str) -> Result<RiceSpec> {
    let path = Path::new(rice);
    if path.exists() {
        return RiceSpec::load(path);
    }
    match presets::get(rice)? {
        Some(spec) => Ok(spec),
        None => {
            let names: Vec<String> = presets::all()?.into_iter().map(|s| s.name).collect();
            bail!(
                "нет ни файла «{rice}», ни встроенного райса с таким именем. Встроенные: {}",
                names.join(", ")
            )
        }
    }
}

fn print_plan(plan: &Plan, env: &Env) {
    println!("Райс «{}»", plan.spec.name);
    if env.sandbox {
        println!("Песочница: {}", env.home.display());
    }

    println!("\nПакеты:");
    for package in &plan.packages {
        let (mark, note) = match package.installed {
            Some(true) => ("=", "уже установлен"),
            Some(false) => ("+", "будет установлен"),
            None => ("?", "не удалось проверить"),
        };
        let source = if package.aur { " (AUR)" } else { "" };
        println!("  {mark} {:<28} {note}{source}", package.name);
    }

    println!("\nКонфиги:");
    for file in &plan.files {
        let (mark, note) = match file.status {
            FileStatus::Create => ("+", "новый"),
            FileStatus::Update => ("~", "изменится"),
            FileStatus::Unchanged => ("=", "без изменений"),
            FileStatus::Remove => ("-", "удалится: программы больше нет в райсе"),
        };
        println!("  {mark} {:<38} {note}", env.display(&file.path));
    }

    if !plan.user_files.is_empty() {
        println!("\nПустые файлы для ваших правок (plove их не перезаписывает):");
        for path in &plan.user_files {
            println!("  + {}", env.display(path));
        }
    }

    for warning in &plan.warnings {
        println!("\n! {warning}");
    }

    if plan.is_noop() {
        println!("\nВсё уже применено, делать нечего.");
    }
}

/// Спросить «да/нет» в терминале. По умолчанию — нет.
fn confirm(question: &str) -> Result<bool> {
    print!("\n{question} [y/N] ");
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    let answer = answer.trim().to_lowercase();
    Ok(matches!(answer.as_str(), "y" | "yes" | "д" | "да"))
}

//! Командная строка plove.
//!
//! Здесь только разбор аргументов и красивый вывод. Вся работа
//! делается в plove-core: CLI — тонкий клиент над ядром.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use plove_core::apply::{self, ApplyOptions};
use plove_core::check::{self, Check, Level, LiveProbe};
use plove_core::modules::Registry;
use plove_core::plan::{self, FileStatus, Plan};
use plove_core::presets;
use plove_core::spec::RiceSpec;
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

    #[command(subcommand)]
    command: Command,
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
    /// Проверить, готова ли система к райсу
    Doctor {
        /// Исправить то, что можно исправить автоматически
        #[arg(long)]
        fix: bool,
    },
    /// Показать встроенные модули
    Modules,
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

    match cli.command {
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
            let report = apply::apply(&plan, &env, &ApplyOptions { skip_packages })?;
            println!();
            match report.generation {
                Some(n) => println!(
                    "Готово: записано файлов {}, поколение #{n}. Откатить: plove rollback",
                    report.files_written
                ),
                None => println!("Готово."),
            }
            if report.reloads_skipped {
                println!(
                    "Графическая сессия не запущена: настройки подхватятся при старте Hyprland."
                );
                println!("Запустить: start-hyprland");
            }
            for command in report.failed_reloads {
                println!("  не сработало «{command}» (программа, скорее всего, не запущена)");
            }
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
        Command::Doctor { fix } => doctor(fix)?,
        Command::Modules => {
            for module in registry.iter() {
                let m = &module.manifest;
                println!(
                    "{:<12} {:<12} пакеты: {}",
                    m.id,
                    m.role,
                    m.packages.join(", ")
                );
            }
        }
    }
    Ok(())
}

/// `plove doctor`: показать все проверки и, с `--fix`, починить что можно.
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
        println!("  {mark} {:<28} {note}", package.name);
    }

    println!("\nКонфиги:");
    for file in &plan.files {
        let (mark, note) = match file.status {
            FileStatus::Create => ("+", "новый"),
            FileStatus::Update => ("~", "изменится"),
            FileStatus::Unchanged => ("=", "без изменений"),
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

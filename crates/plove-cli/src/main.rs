//! Командная строка plove.
//!
//! Здесь только разбор аргументов и красивый вывод. Вся работа
//! делается в plove-core: CLI — тонкий клиент над ядром.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};
use plove_core::apply::{self, ApplyOptions};
use plove_core::modules::Registry;
use plove_core::plan::{self, FileStatus, Plan};
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
        /// Файл райса (.yaml)
        spec: PathBuf,
    },
    /// Применить райс
    Apply {
        /// Файл райса (.yaml)
        spec: PathBuf,
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
    let env = match cli.root {
        Some(root) => Env::sandbox(root),
        None => Env::real()?,
    };
    let registry = Registry::builtin()?;

    match cli.command {
        Command::Plan { spec } => {
            let plan = load_plan(&spec, &registry, &env)?;
            print_plan(&plan, &env);
        }
        Command::Apply {
            spec,
            yes,
            skip_packages,
        } => {
            let plan = load_plan(&spec, &registry, &env)?;
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
            for command in report.failed_reloads {
                println!("  не сработало «{command}» (программа, скорее всего, не запущена)");
            }
        }
        Command::Rollback { yes } => {
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
                if !env.sandbox {
                    for command in &generation.record.reloads {
                        plove_core::system::run_reload(command);
                    }
                }
                println!("Откачено поколение #{}.", generation.number);
            }
        }
        Command::History => {
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

fn load_plan(spec_path: &Path, registry: &Registry, env: &Env) -> Result<Plan> {
    let spec = RiceSpec::load(spec_path)?;
    plan::build(spec, registry, env)
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

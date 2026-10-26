//! Применение плана к системе.
//!
//! Порядок важен:
//! 1. ставим пакеты (если не вышло, конфиги ещё не тронуты);
//! 2. сохраняем копии старых файлов (поколение для отката);
//! 3. пишем новые конфиги;
//! 4. просим программы перечитать конфиги.
//!
//! Повторный запуск с тем же райсом ничего не делает: план покажет,
//! что все файлы без изменений, и поколение не создаётся.

use std::fs::{self, OpenOptions};
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::plan::Plan;
use crate::state;
use crate::system::{self, Env};

pub struct ApplyOptions {
    /// Не ставить пакеты (например, их уже поставили вручную).
    pub skip_packages: bool,
}

pub struct ApplyReport {
    /// Номер созданного поколения, если файлы менялись.
    pub generation: Option<u32>,
    pub files_written: usize,
    /// Команды перезагрузки, которые не сработали (обычно программа не запущена).
    pub failed_reloads: Vec<String>,
    /// Перезагрузку пропустили: графическая сессия не запущена,
    /// конфиги подхватятся при следующем старте Hyprland.
    pub reloads_skipped: bool,
}

pub fn apply(plan: &Plan, env: &Env, options: &ApplyOptions) -> Result<ApplyReport> {
    // 1. Пакеты. В песочнице систему не трогаем.
    if !options.skip_packages && !env.sandbox {
        if plan.packages.iter().any(|p| p.installed.is_none()) {
            bail!("pacman не найден, похоже, это не Arch. Запустите с --skip-packages");
        }
        let missing = plan.missing_packages();
        if !missing.is_empty() {
            system::install_packages(&missing)?;
        }
    }

    // 2–3. Снимок старых файлов и запись новых.
    let changed = plan.changed_files();
    let mut generation = None;
    if !changed.is_empty() {
        let paths: Vec<&Path> = changed.iter().map(|f| f.path.as_path()).collect();
        let snapshot = state::snapshot(env, &plan.spec, &paths, &plan.reloads)?;
        generation = Some(snapshot.number);
        for file in &changed {
            write_atomic(&file.path, &file.content)?;
        }
    }

    for path in &plan.user_files {
        create_empty(path)?;
    }

    // 4. Перезагрузка программ: только на живой системе, внутри графической
    //    сессии и только если что-то поменялось.
    let mut failed_reloads = Vec::new();
    let wants_reload = !env.sandbox && !changed.is_empty();
    let session = if wants_reload {
        system::graphical_session()
    } else {
        None
    };
    let reloads_skipped = wants_reload && session.is_none();
    if let Some(session) = &session {
        for command in &plan.reloads {
            if !system::run_reload(command, session) {
                failed_reloads.push(command.clone());
            }
        }
    }

    Ok(ApplyReport {
        generation,
        files_written: changed.len(),
        failed_reloads,
        reloads_skipped,
    })
}

/// Записать файл «атомарно»: сначала во временный файл рядом, потом
/// переименовать. Если запись прервётся, старый конфиг останется целым,
/// а не обрезанным наполовину.
fn write_atomic(path: &Path, content: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let tmp = path.with_file_name(format!(".{file_name}.plove-tmp"));
    fs::write(&tmp, content).with_context(|| format!("не удалось записать {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("не удалось записать {}", path.display()))?;
    Ok(())
}

/// Создать пустой файл, только если его ещё нет. Существующий не трогаем никогда.
fn create_empty(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    // create_new: ошибка, если файл уже есть. Эту ошибку мы просто игнорируем.
    let _ = OpenOptions::new().write(true).create_new(true).open(path);
    Ok(())
}

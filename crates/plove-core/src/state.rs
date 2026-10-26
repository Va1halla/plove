//! История применений и откат.
//!
//! Перед каждой записью конфигов plove сохраняет «поколение» (generation):
//! копии файлов в том виде, в каком они были ДО применения.
//!
//! ```text
//! ~/.local/state/plove/generations/
//! └── 0001/
//!     ├── generation.json   что было сохранено и как перезагрузить программы
//!     ├── spec.yaml         райс, который применяли
//!     └── backups/0.bak     старое содержимое файлов
//! ```
//!
//! Откат берёт последнее поколение, возвращает файлы как было и удаляет
//! это поколение. Повторный откат идёт ещё на шаг назад.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::spec::RiceSpec;
use crate::system::Env;

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerationRecord {
    /// Время применения, секунды с 1970 года (Unix time).
    pub created_at: u64,
    pub spec_name: String,
    pub files: Vec<FileRecord>,
    pub reloads: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileRecord {
    pub path: PathBuf,
    /// Имя копии в `backups/`. `None` — файла до применения не было,
    /// значит при откате его нужно удалить.
    pub backup: Option<String>,
}

pub struct Generation {
    pub number: u32,
    pub dir: PathBuf,
    pub record: GenerationRecord,
}

fn generations_dir(env: &Env) -> PathBuf {
    env.state_dir().join("generations")
}

/// Все поколения, от старых к новым.
pub fn list(env: &Env) -> Result<Vec<Generation>> {
    let root = generations_dir(env);
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut generations = Vec::new();
    for entry in fs::read_dir(&root)? {
        let dir = entry?.path();
        // Папки называются номерами: 0001, 0002… Всё остальное пропускаем.
        let Some(number) = dir
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.parse::<u32>().ok())
        else {
            continue;
        };
        let json = fs::read_to_string(dir.join("generation.json"))
            .with_context(|| format!("повреждено поколение {}", dir.display()))?;
        let record = serde_json::from_str(&json)?;
        generations.push(Generation {
            number,
            dir,
            record,
        });
    }
    generations.sort_by_key(|g| g.number);
    Ok(generations)
}

/// Сохранить текущее состояние файлов `paths` перед их перезаписью.
pub fn snapshot(
    env: &Env,
    spec: &RiceSpec,
    paths: &[&Path],
    reloads: &[String],
) -> Result<Generation> {
    let number = list(env)?.last().map_or(1, |g| g.number + 1);
    let dir = generations_dir(env).join(format!("{number:04}"));
    let backups = dir.join("backups");
    fs::create_dir_all(&backups)?;

    let mut files = Vec::new();
    // enumerate() даёт пары (индекс, элемент), как enumerate() в Python.
    for (i, path) in paths.iter().enumerate() {
        let backup = if path.exists() {
            let name = format!("{i}.bak");
            fs::copy(path, backups.join(&name))
                .with_context(|| format!("не удалось сохранить копию {}", path.display()))?;
            Some(name)
        } else {
            None
        };
        files.push(FileRecord {
            path: path.to_path_buf(),
            backup,
        });
    }

    let record = GenerationRecord {
        created_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs()),
        spec_name: spec.name.clone(),
        files,
        reloads: reloads.to_vec(),
    };
    fs::write(dir.join("spec.yaml"), spec.to_yaml()?)?;
    fs::write(
        dir.join("generation.json"),
        serde_json::to_string_pretty(&record)?,
    )?;
    Ok(Generation {
        number,
        dir,
        record,
    })
}

/// Откатить последнее применение. `None` — откатывать нечего.
pub fn rollback(env: &Env) -> Result<Option<Generation>> {
    let Some(generation) = list(env)?.pop() else {
        return Ok(None);
    };
    for file in &generation.record.files {
        match &file.backup {
            Some(name) => {
                if let Some(parent) = file.path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(generation.dir.join("backups").join(name), &file.path)
                    .with_context(|| format!("не удалось вернуть {}", file.path.display()))?;
            }
            None => {
                if file.path.exists() {
                    fs::remove_file(&file.path)?;
                }
            }
        }
    }
    fs::remove_dir_all(&generation.dir)?;
    Ok(Some(generation))
}

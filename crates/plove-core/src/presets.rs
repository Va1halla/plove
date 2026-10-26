//! Встроенные райсы: готовые файлы из папки `presets/`, вшитые в бинарник.
//!
//! Встроенный райс — это обычный RiceSpec, ничем не отличающийся от
//! файла пользователя. Имя файла совпадает с полем `name`.

use anyhow::{anyhow, bail, Context, Result};
use include_dir::{include_dir, Dir};

use crate::spec::RiceSpec;

static PRESETS: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../presets");

/// Все встроенные райсы, по алфавиту.
pub fn all() -> Result<Vec<RiceSpec>> {
    let mut presets = Vec::new();
    for file in PRESETS.files() {
        let path = file.path();
        if path.extension().is_none_or(|e| e != "yaml") {
            continue;
        }
        let text = file
            .contents_utf8()
            .ok_or_else(|| anyhow!("{} не в UTF-8", path.display()))?;
        let spec = RiceSpec::from_yaml(text)
            .with_context(|| format!("ошибка во встроенном райсе {}", path.display()))?;
        if path.file_stem().is_none_or(|stem| *stem != *spec.name) {
            bail!(
                "имя файла {} не совпадает с name: {}",
                path.display(),
                spec.name
            );
        }
        presets.push(spec);
    }
    presets.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(presets)
}

/// Встроенный райс по имени.
pub fn get(name: &str) -> Result<Option<RiceSpec>> {
    // into_iter().find(..) — это LINQ FirstOrDefault.
    Ok(all()?.into_iter().find(|spec| spec.name == name))
}

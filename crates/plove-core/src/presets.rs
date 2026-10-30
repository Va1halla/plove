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

/// Основа для райса со своими обоями: ни один из готовых райсов, а
/// нейтральный набор программ из официальных репозиториев. Цвета и
/// картинку подставляет [`from_wallpaper`], стиль выбирается в мастере.
const BLANK: &str = r##"
version: 1
name: blank
theme:
  palette: { bg: "#000000", fg: "#ffffff", accent: "#ffffff" }
  fonts: { ui: "Inter", mono: "JetBrainsMono Nerd Font", size: 11 }
  geometry: { gaps: 10, border: 2, radius: 10, opacity: 0.93 }
components:
  compositor: hyprland
  bar: quickshell
  terminal: kitty
  wallpaper: swaybg
  launcher: fuzzel
  notifications: qs-notifications
  power: power-menu
  lock: hyprlock
  idle: hypridle
  screenshot: grim
  clipboard: cliphist
  polkit: hyprpolkitagent
  gtk: gtk
  qt: qt
  files: thunar
  shell: zsh
  fetch: fastfetch
  monitor: btop
  editor: neovim
overrides:
  hyprland: { kb_layout: "us,ru" }
  quickshell: { style: island }
"##;

/// Райс вокруг своей картинки: палитра из неё, остальное из [`BLANK`].
/// `wallpaper` — путь к картинке, лучше через `~/`, чтобы райс переносился.
pub fn from_wallpaper(
    name: &str,
    wallpaper: &str,
    palette: crate::spec::Palette,
) -> Result<RiceSpec> {
    let mut spec = RiceSpec::from_yaml(BLANK).context("ошибка в основе райса")?;
    spec.name = name.to_string();
    spec.description = Some(format!("Свой райс: цвета из {wallpaper}"));
    spec.theme.palette = palette;
    spec.theme.wallpaper = Some(crate::spec::Wallpaper::File(wallpaper.to_string()));
    spec.validate()?;
    Ok(spec)
}

/// Райс из картинки целиком: цвета, имя по имени файла, путь через `~/`.
pub fn from_image(image: &std::path::Path) -> Result<RiceSpec> {
    from_image_with(image, crate::palette::Options::default())
}

/// То же, но цвета подбираются с подсказками (контрастный акцент и т. п.).
pub fn from_image_with(
    image: &std::path::Path,
    options: crate::palette::Options,
) -> Result<RiceSpec> {
    let image = image
        .canonicalize()
        .with_context(|| format!("картинка {} не найдена", image.display()))?;
    let palette = crate::palette::from_pixels_with(&crate::palette::pixels(&image)?, options);
    let name = image
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_lowercase().replace(' ', "-"))
        .unwrap_or_else(|| "my-rice".into());
    from_wallpaper(&name, &home_relative(&image), palette)
}

/// Путь через «~/», если он внутри домашней папки: так райс можно
/// перенести на машину с другим именем пользователя.
pub fn home_relative(path: &std::path::Path) -> String {
    let home = std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    match path.strip_prefix(&home) {
        Ok(rest) if !home.as_os_str().is_empty() => format!("~/{}", rest.display()),
        _ => path.display().to_string(),
    }
}

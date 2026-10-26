//! Движок тем: превращает токены из RiceSpec в данные для шаблонов.
//!
//! Шаблоны модулей не видят RiceSpec напрямую. Они получают
//! подготовленный «контекст»: цвета сразу в нужных форматах,
//! производные цвета и посчитанные значения геометрии.
//! Так шаблоны остаются простыми, а вся математика живёт здесь.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::spec::{Color, Fonts, RiceSpec};

/// Один цвет в нескольких записях, потому что программы пишут цвета по-разному:
/// Waybar (CSS) хочет `#1a1b26` или `rgba(26, 27, 38, 0.9)`, Hyprland хочет `rgb(1a1b26)`.
#[derive(Debug, Serialize)]
pub struct ColorForms {
    /// `#1a1b26`
    pub hex: String,
    /// `1a1b26`
    pub raw: String,
    /// `26, 27, 38`
    pub rgb: String,
}

impl From<Color> for ColorForms {
    fn from(c: Color) -> Self {
        let hex = c.hex();
        ColorForms {
            raw: hex[1..].to_string(),
            rgb: format!("{}, {}, {}", c.r, c.g, c.b),
            hex,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct GeometryContext {
    pub gaps: u32,
    /// Половина отступа: в Hyprland внутренние отступы складываются от двух окон.
    pub gaps_in: u32,
    pub border: u32,
    pub radius: u32,
    pub opacity: f64,
    /// Неактивные окна чуть прозрачнее активного.
    pub opacity_inactive: f64,
}

/// Всё, что видит шаблон про тему. Поля доступны в шаблоне как
/// `{{ colors.accent.hex }}`, `{{ fonts.mono }}`, `{{ geometry.gaps }}`.
#[derive(Debug, Serialize)]
pub struct ThemeContext {
    pub name: String,
    pub colors: BTreeMap<&'static str, ColorForms>,
    pub ansi: Vec<ColorForms>,
    pub fonts: Fonts,
    pub geometry: GeometryContext,
}

pub fn theme_context(spec: &RiceSpec) -> ThemeContext {
    let palette = &spec.theme.palette;
    let geometry = &spec.theme.geometry;

    let mut colors = BTreeMap::new();
    colors.insert("bg", palette.bg.into());
    colors.insert("fg", palette.fg.into());
    colors.insert("accent", palette.accent.into());
    // Производные цвета, чтобы не заставлять пользователя задавать всё вручную.
    // surface: чуть светлее фона, для рамок и неактивных элементов.
    colors.insert("surface", palette.bg.mix(palette.fg, 0.15).into());
    // muted: приглушённый текст.
    colors.insert("muted", palette.bg.mix(palette.fg, 0.5).into());

    ThemeContext {
        name: spec.name.clone(),
        colors,
        // `.iter().map(..).collect()` — это LINQ `.Select(..).ToList()`.
        ansi: palette.ansi.iter().map(|&c| c.into()).collect(),
        fonts: spec.theme.fonts.clone(),
        geometry: GeometryContext {
            gaps: geometry.gaps,
            gaps_in: geometry.gaps / 2,
            border: geometry.border,
            radius: geometry.radius,
            opacity: geometry.opacity,
            opacity_inactive: round2((geometry.opacity - 0.05).max(0.0)),
        },
    }
}

/// Округлить до двух знаков, чтобы в конфиге было `0.87`, а не `0.8700000000000001`.
fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// Какой пакет Arch ставит этот шрифт. `None` — не знаем,
/// тогда план предупредит, что шрифт нужно поставить вручную.
pub fn font_package(font: &str) -> Option<&'static str> {
    // `match` — это switch из C#, только он обязан покрыть все варианты.
    match font {
        "JetBrainsMono Nerd Font" => Some("ttf-jetbrains-mono-nerd"),
        "FiraCode Nerd Font" => Some("ttf-firacode-nerd"),
        "Iosevka Nerd Font" => Some("ttf-iosevka-nerd"),
        "Hack Nerd Font" => Some("ttf-hack-nerd"),
        "Inter" => Some("inter-font"),
        "Noto Sans" => Some("noto-fonts"),
        "Roboto" => Some("ttf-roboto"),
        "Cantarell" => Some("cantarell-fonts"),
        _ => None,
    }
}

//! Движок тем: превращает токены из RiceSpec в данные для шаблонов.
//!
//! Шаблоны модулей не видят RiceSpec напрямую. Они получают
//! подготовленный «контекст»: цвета сразу в нужных форматах,
//! производные цвета и посчитанные значения геометрии.
//! Так шаблоны остаются простыми, а вся математика живёт здесь.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::spec::{
    Borders, Color, Density, Fonts, Geometry, Glass, RiceSpec, Separators, Shape, Style,
};

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
    pub style: StyleContext,
}

/// Стиль для шаблонов: сами решения (`{{ style.shape }}` = "pill") и готовые
/// числа из них (`{{ style.radius_chip }}`), чтобы каждый модуль не считал сам.
#[derive(Debug, Serialize)]
pub struct StyleContext {
    /// `flatten` кладёт поля Style прямо сюда: `style.shape`, а не `style.raw.shape`.
    #[serde(flatten)]
    pub raw: Style,
    /// Скругление окон.
    pub radius_window: u32,
    /// Скругление крупных плашек: панель, меню, уведомления.
    pub radius_box: u32,
    /// Скругление мелких элементов: кнопки, модули панели, рабочие столы.
    pub radius_chip: u32,
    /// То же, но не больше 18 px: для программ, которые не понимают
    /// «999 px = полукруг» (rofi, fuzzel).
    pub radius_chip_px: u32,
    /// Внутренний отступ элементов, px, и его половина.
    pub pad: u32,
    pub pad_small: u32,
    pub bar_height: u32,
    /// Толщина рамок, px (0 — без рамок).
    pub border: u32,
    /// Непрозрачность фонов панелей и меню, 0.0–1.0.
    pub bg_alpha: f64,
    pub blur: bool,
    pub blur_passes: u32,
    /// Значки-разделители для левой, центральной и правой частей панели.
    pub sep_left: &'static str,
    pub sep_center: &'static str,
    pub sep_right: &'static str,
}

impl StyleContext {
    pub fn new(style: Style, geometry: &Geometry) -> StyleContext {
        let r = geometry.radius;
        let (radius_window, radius_box, radius_chip) = match style.shape {
            Shape::Sharp => (0, 0, 0),
            Shape::Rounded => (r, r, (r / 2).max(r.min(4))),
            // 999 px в CSS — «полностью круглые края», как у таблетки.
            Shape::Pill => (r, (r + r / 2).max(12), 999),
        };
        let (pad, bar_height) = match style.density {
            Density::Compact => (4, 26),
            Density::Normal => (8, 32),
            Density::Spacious => (14, 40),
        };
        let border = match style.borders {
            Borders::None => 0,
            Borders::Thin => 1,
            Borders::Accent | Borders::Gradient => geometry.border.max(1),
        };
        let (bg_alpha, blur, blur_passes) = match style.glass {
            Glass::None => (1.0, false, 1),
            Glass::Light => (0.85, true, 2),
            Glass::Strong => (0.6, true, 3),
        };
        // Значки Powerline из Nerd Font; справа — зеркальные.
        let (sep_left, sep_center, sep_right) = match style.separators {
            Separators::None => ("", "", ""),
            Separators::Dots => ("•", "•", "•"),
            Separators::Line => ("│", "│", "│"),
            Separators::Arrows => ("\u{e0b1}", "\u{e0b1}", "\u{e0b3}"),
            Separators::Slants => ("\u{e0bb}", "\u{e0bb}", "\u{e0b9}"),
            Separators::Round => ("\u{e0b5}", "\u{e0b5}", "\u{e0b7}"),
        };
        StyleContext {
            raw: style,
            radius_window,
            radius_box,
            radius_chip,
            radius_chip_px: radius_chip.min(18),
            pad,
            pad_small: pad / 2,
            bar_height,
            border,
            bg_alpha,
            blur,
            blur_passes,
            sep_left,
            sep_center,
            sep_right,
        }
    }
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
        style: StyleContext::new(spec.theme.style, geometry),
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

#[cfg(test)]
mod style_tests {
    use super::*;

    #[test]
    fn style_tokens_turn_into_numbers() {
        let geometry = Geometry::default();
        let sharp = StyleContext::new(
            Style {
                shape: Shape::Sharp,
                borders: Borders::None,
                glass: Glass::None,
                ..Style::default()
            },
            &geometry,
        );
        assert_eq!(
            (sharp.radius_window, sharp.radius_chip, sharp.border),
            (0, 0, 0)
        );
        // Hyprland не принимает 0 проходов размытия даже при выключенном размытии.
        assert!(!sharp.blur && sharp.blur_passes >= 1);

        let pill = StyleContext::new(
            Style {
                shape: Shape::Pill,
                separators: Separators::Arrows,
                ..Style::default()
            },
            &geometry,
        );
        assert_eq!(pill.radius_chip, 999);
        assert_eq!(pill.radius_chip_px, 18);
        assert_eq!(pill.sep_right, "\u{e0b3}");
    }
}

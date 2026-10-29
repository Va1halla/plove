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
    /// Шрифт для значков рядом с текстом: вариант Nerd Font «Propo».
    /// В обычном Nerd Font значок шире своей клетки и налезает на пробел
    /// («󰅐12:44» вместо «󰅐 12:44»), а у Propo ширина честная.
    pub icon_font: String,
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
        icon_font: icon_font(&spec.theme.fonts.mono),
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

/// `JetBrainsMono Nerd Font` → `JetBrainsMono Nerd Font Propo`. Пакеты
/// ttf-*-nerd ставят Propo вместе с основным шрифтом. Другие шрифты как есть.
fn icon_font(mono: &str) -> String {
    if mono.ends_with("Nerd Font") {
        format!("{mono} Propo")
    } else {
        mono.to_string()
    }
}

/// Округлить до двух знаков, чтобы в конфиге было `0.87`, а не `0.8700000000000001`.
fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// Шрифт, который plove умеет поставить: имя семейства, пакет Arch
/// и моноширинный ли он. Моноширинные — с Nerd Font: в них есть значки.
pub struct FontInfo {
    pub name: &'static str,
    pub package: &'static str,
    pub mono: bool,
}

const fn font(name: &'static str, package: &'static str, mono: bool) -> FontInfo {
    FontInfo {
        name,
        package,
        mono,
    }
}

/// Все пакеты из официальных репозиториев (extra), без AUR.
pub const FONTS: &[FontInfo] = &[
    font("Inter", "inter-font", false),
    font("Noto Sans", "noto-fonts", false),
    font("Roboto", "ttf-roboto", false),
    font("Cantarell", "cantarell-fonts", false),
    font("Ubuntu", "ttf-ubuntu-font-family", false),
    font("Fira Sans", "ttf-fira-sans", false),
    font("IBM Plex Sans", "ttf-ibm-plex", false),
    font("Source Sans 3", "adobe-source-sans-fonts", false),
    font("Open Sans", "ttf-opensans", false),
    font("Lato", "ttf-lato", false),
    font("Montserrat", "ttf-montserrat", false),
    font("Overpass", "otf-overpass", false),
    font("Nunito", "ttf-nunito", false),
    font("DejaVu Sans", "ttf-dejavu", false),
    font("JetBrainsMono Nerd Font", "ttf-jetbrains-mono-nerd", true),
    font("FiraCode Nerd Font", "ttf-firacode-nerd", true),
    font("Iosevka Nerd Font", "ttf-iosevka-nerd", true),
    font("Hack Nerd Font", "ttf-hack-nerd", true),
    font("CaskaydiaCove Nerd Font", "ttf-cascadia-code-nerd", true),
    font("Mononoki Nerd Font", "ttf-mononoki-nerd", true),
    font("VictorMono Nerd Font", "ttf-victor-mono-nerd", true),
    font("SauceCodePro Nerd Font", "ttf-sourcecodepro-nerd", true),
    font("UbuntuMono Nerd Font", "ttf-ubuntu-mono-nerd", true),
    font("GeistMono Nerd Font", "otf-geist-mono-nerd", true),
    font("CommitMono Nerd Font", "otf-commit-mono-nerd", true),
    font("SpaceMono Nerd Font", "ttf-space-mono-nerd", true),
    font("RobotoMono Nerd Font", "ttf-roboto-mono-nerd", true),
    font("ZedMono Nerd Font", "ttf-zed-mono-nerd", true),
];

/// Какой пакет Arch ставит этот шрифт. `None` — не знаем,
/// тогда план предупредит, что шрифт нужно поставить вручную.
pub fn font_package(name: &str) -> Option<&'static str> {
    FONTS.iter().find(|f| f.name == name).map(|f| f.package)
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

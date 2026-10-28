//! Цвета терминала: можно оставить цвета райса, а можно взять другую схему
//! и подправить фон. Настройки `colors` и `background` есть у каждого
//! модуля-терминала (kitty, alacritty, ghostty).
//!
//! Шаблоны терминалов ничего про это не знают: план просто подменяет им
//! `colors` и `ansi` на цвета выбранной схемы.

use std::collections::BTreeMap;

use anyhow::{bail, Result};

use crate::presets;
use crate::spec::{Color, Palette};
use crate::theme::ColorForms;

/// Схемы, которых нет среди готовых райсов: bg, fg, акцент и 16 цветов.
const EXTRA: [(&str, [&str; 19]); 7] = [
    (
        "dracula",
        [
            "#282a36", "#f8f8f2", "#bd93f9", "#21222c", "#ff5555", "#50fa7b", "#f1fa8c", "#bd93f9",
            "#ff79c6", "#8be9fd", "#f8f8f2", "#6272a4", "#ff6e6e", "#69ff94", "#ffffa5", "#d6acff",
            "#ff92df", "#a4ffff", "#ffffff",
        ],
    ),
    (
        "rose-pine",
        [
            "#191724", "#e0def4", "#c4a7e7", "#26233a", "#eb6f92", "#31748f", "#f6c177", "#9ccfd8",
            "#c4a7e7", "#ebbcba", "#e0def4", "#6e6a86", "#eb6f92", "#31748f", "#f6c177", "#9ccfd8",
            "#c4a7e7", "#ebbcba", "#e0def4",
        ],
    ),
    (
        "everforest",
        [
            "#2d353b", "#d3c6aa", "#a7c080", "#475258", "#e67e80", "#a7c080", "#dbbc7f", "#7fbbb3",
            "#d699b6", "#83c092", "#d3c6aa", "#5c6a72", "#e67e80", "#a7c080", "#dbbc7f", "#7fbbb3",
            "#d699b6", "#83c092", "#d3c6aa",
        ],
    ),
    (
        "kanagawa",
        [
            "#1f1f28", "#dcd7ba", "#7e9cd8", "#16161d", "#c34043", "#76946a", "#c0a36e", "#7e9cd8",
            "#957fb8", "#6a9589", "#c8c093", "#727169", "#e82424", "#98bb6c", "#e6c384", "#7fb4ca",
            "#938aa9", "#7aa89f", "#dcd7ba",
        ],
    ),
    (
        "one-dark",
        [
            "#282c34", "#abb2bf", "#61afef", "#282c34", "#e06c75", "#98c379", "#e5c07b", "#61afef",
            "#c678dd", "#56b6c2", "#abb2bf", "#5c6370", "#e06c75", "#98c379", "#e5c07b", "#61afef",
            "#c678dd", "#56b6c2", "#ffffff",
        ],
    ),
    (
        "solarized-dark",
        [
            "#002b36", "#839496", "#268bd2", "#073642", "#dc322f", "#859900", "#b58900", "#268bd2",
            "#d33682", "#2aa198", "#eee8d5", "#586e75", "#cb4b16", "#586e75", "#657b83", "#839496",
            "#6c71c4", "#93a1a1", "#fdf6e3",
        ],
    ),
    (
        "catppuccin-latte",
        [
            "#eff1f5", "#4c4f69", "#8839ef", "#5c5f77", "#d20f39", "#40a02b", "#df8e1d", "#1e66f5",
            "#ea76cb", "#179299", "#acb0be", "#6c6f85", "#d20f39", "#40a02b", "#df8e1d", "#1e66f5",
            "#ea76cb", "#179299", "#bcc0cc",
        ],
    ),
];

/// Палитра схемы: `rice` — цвета самого райса, имя готового райса — его
/// цвета, иначе одна из схем выше.
pub fn scheme(name: &str, rice: &Palette) -> Result<Palette> {
    if name == "rice" {
        return Ok(rice.clone());
    }
    if let Some(preset) = presets::get(name)? {
        return Ok(preset.theme.palette);
    }
    let Some((_, colors)) = EXTRA.iter().find(|(id, _)| *id == name) else {
        bail!("цветовой схемы терминала «{name}» нет");
    };
    let colors = colors
        .iter()
        .map(|c| Color::parse(c))
        .collect::<Result<Vec<_>>>()?;
    Ok(Palette {
        bg: colors[0],
        fg: colors[1],
        accent: colors[2],
        ansi: colors[3..].to_vec(),
    })
}

/// Палитра терминала с поправленным фоном. `background`: scheme — фон
/// схемы, darker и lighter — темнее или светлее его, rice — фон райса.
pub fn palette(name: &str, background: &str, rice: &Palette) -> Result<Palette> {
    let mut palette = scheme(name, rice)?;
    let black = Color { r: 0, g: 0, b: 0 };
    palette.bg = match background {
        "scheme" => palette.bg,
        "darker" => palette.bg.mix(black, 0.35),
        "lighter" => palette.bg.mix(palette.fg, 0.08),
        "rice" => rice.bg,
        other => bail!("фон терминала «{other}»: можно scheme, darker, lighter, rice"),
    };
    Ok(palette)
}

/// Палитра терминала из его настроек; `None`, если у модуля их нет.
pub fn from_options(options: &serde_json::Value, rice: &Palette) -> Result<Option<Palette>> {
    let Some(name) = options.get("colors").and_then(|v| v.as_str()) else {
        return Ok(None);
    };
    let background = options
        .get("background")
        .and_then(|v| v.as_str())
        .unwrap_or("scheme");
    palette(name, background, rice).map(Some)
}

/// `colors` и `ansi` для шаблона терминала.
pub fn context(palette: &Palette) -> (BTreeMap<&'static str, ColorForms>, Vec<ColorForms>) {
    let bg = palette.bg;
    let mut colors = BTreeMap::new();
    colors.insert("bg", bg.into());
    colors.insert("fg", palette.fg.into());
    colors.insert("accent", palette.accent.into());
    colors.insert("surface", bg.mix(palette.fg, 0.15).into());
    colors.insert("muted", bg.mix(palette.fg, 0.5).into());
    let ansi = palette.ansi.iter().map(|&c| c.into()).collect();
    (colors, ansi)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rice() -> Palette {
        Palette {
            bg: Color::parse("#101010").unwrap(),
            fg: Color::parse("#f0f0f0").unwrap(),
            accent: Color::parse("#ff0000").unwrap(),
            ansi: vec![],
        }
    }

    #[test]
    fn every_scheme_has_sixteen_colors() {
        for (name, _) in EXTRA {
            assert_eq!(scheme(name, &rice()).unwrap().ansi.len(), 16, "{name}");
        }
        for preset in presets::all().unwrap() {
            assert_eq!(scheme(&preset.name, &rice()).unwrap().ansi.len(), 16);
        }
    }

    #[test]
    fn background_follows_the_choice() {
        let hex = |name, background| palette(name, background, &rice()).unwrap().bg.hex();
        assert_eq!(hex("dracula", "scheme"), "#282a36");
        assert_eq!(hex("dracula", "rice"), "#101010");
        assert_eq!(hex("rice", "darker"), "#0a0a0a");
        let options = serde_json::json!({ "colors": "dracula", "background": "rice" });
        let (colors, ansi) = context(&from_options(&options, &rice()).unwrap().unwrap());
        assert_eq!(
            (colors["bg"].hex.as_str(), colors["fg"].hex.as_str()),
            ("#101010", "#f8f8f2")
        );
        assert_eq!(ansi.len(), 16);
        assert!(palette("no-such", "scheme", &rice()).is_err());
    }
}

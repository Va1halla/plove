//! RiceSpec: декларативное описание райса.
//!
//! Это единственный контракт между интерфейсами и ядром. Ядро не знает,
//! откуда пришёл файл: написал ли его человек, собрал мастер или ИИ.
//!
//! Аналогия: структуры ниже — это DTO-классы из C# или dataclass из Python,
//! в которые разбирается YAML. `#[derive(Deserialize)]` работает как
//! атрибуты `[JsonProperty]`: библиотека serde сама генерирует код разбора.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

/// Версия формата, которую понимает эта сборка plove.
pub const CURRENT_VERSION: u32 = 1;

/// Корень файла райса.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)] // опечатка в ключе YAML даёт ошибку, а не тихий пропуск
pub struct RiceSpec {
    pub version: u32,
    pub name: String,
    pub theme: Theme,
    /// Роль → id модуля, например `bar: waybar`.
    /// BTreeMap — отсортированный словарь (как SortedDictionary в C#),
    /// поэтому порядок обхода всегда одинаковый.
    pub components: BTreeMap<String, String>,
    /// Тонкие настройки модулей: id модуля → ключи из его `defaults`.
    #[serde(default)]
    pub overrides: BTreeMap<String, serde_yaml::Mapping>,
}

/// Дизайн-токены: всё, что описывает внешний вид, но не конкретные программы.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    pub palette: Palette,
    pub fonts: Fonts,
    #[serde(default)]
    pub geometry: Geometry,
    /// Путь к обоям. В этой версии ещё не применяется.
    #[serde(default)]
    pub wallpaper: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Palette {
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    /// 16 цветов терминала (color0..color15). Можно не задавать,
    /// тогда терминал оставит свои цвета по умолчанию.
    #[serde(default)]
    pub ansi: Vec<Color>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Fonts {
    /// Шрифт интерфейса: панель, уведомления.
    pub ui: String,
    /// Моноширинный шрифт: терминал.
    pub mono: String,
    #[serde(default = "default_font_size")]
    pub size: u32,
}

fn default_font_size() -> u32 {
    11
}

/// Геометрия окон. `#[serde(default)]` на структуре означает:
/// пропущенные поля берутся из `Default` ниже.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct Geometry {
    /// Отступы между окнами, px.
    pub gaps: u32,
    /// Толщина рамки окна, px.
    pub border: u32,
    /// Скругление углов, px.
    pub radius: u32,
    /// Непрозрачность от 0.0 (прозрачно) до 1.0 (непрозрачно).
    pub opacity: f64,
}

// `impl Трейт for Тип` — реализация интерфейса, как `class Geometry : IDefault` в C#.
impl Default for Geometry {
    fn default() -> Self {
        Geometry {
            gaps: 8,
            border: 2,
            radius: 10,
            opacity: 0.95,
        }
    }
}

/// Цвет в формате `#rrggbb`, хранится как три байта.
///
/// `try_from = "String"` говорит serde: сначала прочитай строку,
/// потом преврати её в Color через `TryFrom` (и покажи ошибку, если не вышло).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn parse(text: &str) -> Result<Color> {
        let hex = text.strip_prefix('#').unwrap_or(text);
        if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            bail!("«{text}» не похож на цвет, ожидается формат #rrggbb");
        }
        // from_str_radix(.., 16) — это int.Parse(s, NumberStyles.HexNumber) из C#.
        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).unwrap();
        Ok(Color {
            r: byte(0),
            g: byte(2),
            b: byte(4),
        })
    }

    /// `#1a1b26`
    pub fn hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Смешать два цвета: `t = 0.0` даёт self, `t = 1.0` даёт other.
    pub fn mix(self, other: Color, t: f64) -> Color {
        let channel = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t).round() as u8;
        Color {
            r: channel(self.r, other.r),
            g: channel(self.g, other.g),
            b: channel(self.b, other.b),
        }
    }
}

impl TryFrom<String> for Color {
    type Error = String;
    fn try_from(text: String) -> Result<Self, Self::Error> {
        Color::parse(&text).map_err(|e| e.to_string())
    }
}

impl From<Color> for String {
    fn from(color: Color) -> String {
        color.hex()
    }
}

impl RiceSpec {
    /// Прочитать и проверить файл райса.
    pub fn load(path: &Path) -> Result<RiceSpec> {
        // `?` в конце строки — «если ошибка, сразу верни её наверх».
        // Аналог исключения, только явный: без `?` код не скомпилируется.
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("не удалось прочитать {}", path.display()))?;
        Self::from_yaml(&text).with_context(|| format!("ошибка в файле райса {}", path.display()))
    }

    pub fn from_yaml(text: &str) -> Result<RiceSpec> {
        let spec: RiceSpec = serde_yaml::from_str(text)?;
        spec.validate()?;
        Ok(spec)
    }

    pub fn to_yaml(&self) -> Result<String> {
        Ok(serde_yaml::to_string(self)?)
    }

    /// Проверки, которые нельзя выразить одними типами.
    pub fn validate(&self) -> Result<()> {
        if self.version != CURRENT_VERSION {
            bail!(
                "версия формата {} не поддерживается (эта сборка понимает версию {CURRENT_VERSION})",
                self.version
            );
        }
        if self.name.trim().is_empty() {
            bail!("поле name не может быть пустым");
        }
        let ansi = self.theme.palette.ansi.len();
        if ansi != 0 && ansi != 16 {
            bail!("palette.ansi должен содержать ровно 16 цветов, а сейчас {ansi}");
        }
        let opacity = self.theme.geometry.opacity;
        if !(0.0..=1.0).contains(&opacity) {
            bail!("geometry.opacity должен быть от 0.0 до 1.0, а сейчас {opacity}");
        }
        if !self.components.contains_key("compositor") {
            bail!("в components обязательно должен быть compositor");
        }
        Ok(())
    }
}

// Модульные тесты живут прямо рядом с кодом. `cargo test` их запускает.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_roundtrip() {
        let c = Color::parse("#1A1b26").unwrap();
        assert_eq!(c.hex(), "#1a1b26");
        assert!(Color::parse("#12345").is_err());
        assert!(Color::parse("zzzzzz").is_err());
    }

    #[test]
    fn mix_is_between() {
        let black = Color::parse("#000000").unwrap();
        let white = Color::parse("#ffffff").unwrap();
        assert_eq!(black.mix(white, 0.5).hex(), "#808080");
    }

    #[test]
    fn unknown_key_is_an_error() {
        let yaml = "version: 1\nname: x\ncolour: red\ntheme: {palette: {bg: '#000000', fg: '#ffffff', accent: '#ff0000'}, fonts: {ui: a, mono: b}}\ncomponents: {compositor: hyprland}\n";
        assert!(RiceSpec::from_yaml(yaml).is_err());
    }
}

//! Карточки на рабочем столе: каталог видов (часы, погода, плеер…) и
//! раскладка из файла райса (`desktop:`), которую собирают в `plove studio`.
//!
//! Каждая карточка — отдельное окно eww в своей точке экрана. Точка
//! задаётся в процентах, поэтому раскладка одинаково ложится на любой
//! монитор; размер — в пикселях, умноженных на `scale`.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::spec::{check_font, Color, Palette, RiceSpec, COLOR_NAMES};
use crate::theme::StyleContext;

/// Карточка на рабочем столе, как она записана в райсе:
/// `{ kind: weather, at: [80, 6], with: { city: Москва } }`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeskCard {
    /// Вид из [`CATALOG`].
    pub kind: String,
    /// Левый верхний угол: проценты ширины и высоты экрана.
    pub at: [f64; 2],
    /// Во сколько раз больше обычного размера.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub scale: f64,
    /// Параметры вида; чего нет — берётся по умолчанию.
    #[serde(default, skip_serializing_if = "serde_yaml::Mapping::is_empty")]
    pub with: serde_yaml::Mapping,
    /// Оформление этой карточки поверх стиля райса.
    #[serde(default, skip_serializing_if = "Look::is_empty")]
    pub look: Look,
}

/// Оформление карточки: `look: { font: Orbitron, fill: none, radius: 24 }`.
/// Чего нет — как у остальных карточек по стилю райса. Цвета — имена из
/// палитры (accent, fg, red…) или `#rrggbb`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Look {
    /// Шрифт всей карточки; значки берутся из Nerd Font райса.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    /// Цвет крупных цифр, колец и выделения.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    /// Цвет обычного текста; подписи — он же, бледнее.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Фон: цвет или `none` — без фона.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    /// Непрозрачность фона, от 0 до 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f64>,
    /// Скругление углов, px. Больше половины высоты — «таблетка».
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<u32>,
    /// Рамка цвета accent, px.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<u32>,
}

impl Look {
    pub fn is_empty(&self) -> bool {
        *self == Look::default()
    }
}

fn one() -> f64 {
    1.0
}

fn is_one(x: &f64) -> bool {
    *x == 1.0
}

/// Вид карточки: что показывает и какие у неё параметры.
pub struct Kind {
    pub id: &'static str,
    pub title: &'static str,
    pub about: &'static str,
    /// Размер при scale 1, пиксели.
    pub size: (u32, u32),
    pub params: &'static [Param],
}

/// Параметр вида: строка или переключатель.
pub struct Param {
    pub id: &'static str,
    pub title: &'static str,
    pub default: Value,
}

/// Значение по умолчанию; от него же зависит тип параметра.
pub enum Value {
    Text(&'static str),
    Switch(bool),
}

/// Все виды карточек.
pub const CATALOG: [Kind; 12] = [
    Kind {
        id: "clock",
        title: "Часы",
        about: "Крупное время и дата под ним.",
        size: (440, 190),
        params: &[
            Param {
                id: "seconds",
                title: "Секунды",
                default: Value::Switch(false),
            },
            Param {
                id: "date",
                title: "Дата",
                default: Value::Switch(true),
            },
        ],
    },
    Kind {
        id: "worldclock",
        title: "Мировое время",
        about: "Время в нескольких городах.",
        size: (300, 170),
        params: &[Param {
            id: "zones",
            title: "Часовые пояса через запятую",
            default: Value::Text("Europe/London, America/New_York, Asia/Tokyo"),
        }],
    },
    Kind {
        id: "calendar",
        title: "Календарь",
        about: "Месяц с отмеченным сегодняшним днём.",
        size: (320, 270),
        params: &[],
    },
    Kind {
        id: "system",
        title: "Система",
        about: "Кольца загрузки процессора, памяти и диска.",
        size: (320, 130),
        params: &[],
    },
    Kind {
        id: "player",
        title: "Плеер",
        about: "Обложка, название, исполнитель и кнопки. Прячется, когда ничего не играет.",
        size: (440, 150),
        params: &[],
    },
    Kind {
        id: "weather",
        title: "Погода",
        about: "Погода сейчас и прогноз на три дня с wttr.in, без ключей. Пустой город — по IP.",
        size: (340, 170),
        params: &[Param {
            id: "city",
            title: "Город",
            default: Value::Text(""),
        }],
    },
    Kind {
        id: "countdown",
        title: "Обратный отсчёт",
        about: "Сколько дней осталось до даты.",
        size: (280, 110),
        params: &[
            Param {
                id: "title",
                title: "Подпись",
                default: Value::Text("Новый год"),
            },
            Param {
                id: "date",
                title: "Дата, ГГГГ-ММ-ДД",
                default: Value::Text("2027-01-01"),
            },
        ],
    },
    Kind {
        id: "note",
        title: "Заметка",
        about: "Свой текст на рабочем столе.",
        size: (300, 120),
        params: &[Param {
            id: "text",
            title: "Текст",
            default: Value::Text("Не забыть"),
        }],
    },
    Kind {
        id: "dotclock",
        title: "Точечные часы",
        about: "Крупные цифры из точек, как на табло. Только с панелью Quickshell.",
        size: (480, 170),
        params: &[
            Param {
                id: "seconds",
                title: "Секунды",
                default: Value::Switch(false),
            },
            Param {
                id: "date",
                title: "Дата под часами",
                default: Value::Switch(true),
            },
        ],
    },
    Kind {
        id: "metric",
        title: "Показатель",
        about: "Одно число крупно и полоса под ним: cpu, ram, disk или temp. Только с панелью Quickshell.",
        size: (220, 120),
        params: &[Param {
            id: "source",
            title: "Что показывать: cpu, ram, disk, temp",
            default: Value::Text("cpu"),
        }],
    },
    Kind {
        id: "sparkline",
        title: "График",
        about: "Загрузка за последние две минуты линией: cpu или ram. Только с панелью Quickshell.",
        size: (320, 130),
        params: &[Param {
            id: "source",
            title: "Что показывать: cpu или ram",
            default: Value::Text("cpu"),
        }],
    },
    Kind {
        id: "widget",
        title: "Свой виджет",
        about: "Виджет из раздела widgets райса: вывод своей команды.",
        size: (280, 70),
        params: &[Param {
            id: "id",
            title: "Имя виджета из widgets",
            default: Value::Text(""),
        }],
    },
];

/// Вид по имени.
pub fn kind(id: &str) -> Option<&'static Kind> {
    CATALOG.iter().find(|k| k.id == id)
}

/// Каталог для редактора: виды, размеры и параметры.
pub fn catalog_json() -> serde_json::Value {
    CATALOG
        .iter()
        .map(|k| {
            let params: Vec<serde_json::Value> = k
                .params
                .iter()
                .map(|p| {
                    let default = match p.default {
                        Value::Text(t) => json!(t),
                        Value::Switch(b) => json!(b),
                    };
                    json!({ "id": p.id, "title": p.title, "default": default })
                })
                .collect();
            json!({
                "id": k.id,
                "title": k.title,
                "about": k.about,
                "size": [k.size.0, k.size.1],
                "params": params,
            })
        })
        .collect()
}

/// Проверить карточки райса: известный вид, параметры, место и размер.
pub fn validate(spec: &RiceSpec) -> Result<()> {
    for (i, card) in spec.desktop.iter().enumerate() {
        let n = i + 1;
        let Some(kind) = kind(&card.kind) else {
            let names: Vec<&str> = CATALOG.iter().map(|k| k.id).collect();
            bail!(
                "desktop, карточка {n}: вида «{}» нет. Есть: {}",
                card.kind,
                names.join(", ")
            );
        };
        if card.at.iter().any(|v| !(0.0..=100.0).contains(v)) {
            bail!("desktop, карточка {n}: at — проценты экрана, от 0 до 100");
        }
        if !(0.3..=4.0).contains(&card.scale) {
            bail!("desktop, карточка {n}: scale от 0.3 до 4");
        }
        for (key, value) in &card.with {
            let key = key.as_str().unwrap_or_default();
            let Some(param) = kind.params.iter().find(|p| p.id == key) else {
                bail!("desktop, карточка {n} ({}): параметра «{key}» нет", kind.id);
            };
            let fits = match param.default {
                Value::Text(_) => value.is_string(),
                Value::Switch(_) => value.is_bool(),
            };
            if !fits {
                bail!(
                    "desktop, карточка {n} ({}): {key} должен быть {}",
                    kind.id,
                    match param.default {
                        Value::Text(_) => "строкой",
                        Value::Switch(_) => "true или false",
                    }
                );
            }
        }
        // Значения попадают в команды оболочки: пускаем только то,
        // что там нужно, без кавычек и прочих спецсимволов.
        let text = |key: &str| card.with.get(key).and_then(|v| v.as_str()).unwrap_or("");
        let zones = text("zones");
        if !zones
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_/+-, ".contains(c))
        {
            bail!("desktop, карточка {n}: zones — пояса вида Europe/Moscow через запятую");
        }
        let source = card.with.get("source").and_then(|v| v.as_str());
        let sources: &[&str] = match kind.id {
            "metric" => &["cpu", "ram", "disk", "temp"],
            "sparkline" => &["cpu", "ram"],
            _ => &[],
        };
        if let Some(source) = source.filter(|s| !sources.contains(s)) {
            bail!(
                "desktop, карточка {n} ({}): source «{source}» — можно {}",
                kind.id,
                sources.join(", ")
            );
        }
        if kind.id == "countdown" && card.with.contains_key("date") {
            let date = text("date");
            let ok = date.len() == 10
                && date.chars().enumerate().all(|(i, c)| {
                    if i == 4 || i == 7 {
                        c == '-'
                    } else {
                        c.is_ascii_digit()
                    }
                });
            if !ok {
                bail!("desktop, карточка {n}: date в виде ГГГГ-ММ-ДД, например 2027-01-01");
            }
        }
        if text("city").contains(['\'', '"', '\\', '`', '$']) {
            bail!("desktop, карточка {n}: в названии города не должно быть кавычек и $");
        }
        if kind.id == "widget" {
            let id = card.with.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if !spec.widgets.contains_key(id) {
                bail!("desktop, карточка {n}: в widgets нет виджета «{id}»");
            }
        }
        check_look(&card.look, &spec.theme.palette)
            .map_err(|e| anyhow::anyhow!("desktop, карточка {n} ({}): look.{e}", kind.id))?;
    }
    Ok(())
}

fn check_look(look: &Look, palette: &Palette) -> Result<()> {
    if let Some(font) = &look.font {
        check_font("font", font)?;
    }
    let colors = [
        ("accent", &look.accent),
        ("text", &look.text),
        ("fill", &look.fill),
    ];
    for (what, color) in colors {
        let Some(color) = color else { continue };
        if !(what == "fill" && color == "none") && palette.named(color).is_none() {
            bail!(
                "{what}: «{color}» — не цвет. Можно {} или #rrggbb{}",
                COLOR_NAMES.join(", "),
                if what == "fill" { ", или none" } else { "" }
            );
        }
    }
    if look.opacity.is_some_and(|o| !(0.0..=1.0).contains(&o)) {
        bail!("opacity от 0 до 1");
    }
    if look.radius.is_some_and(|r| r > 200) {
        bail!("radius от 0 до 200");
    }
    if look.border.is_some_and(|b| b > 12) {
        bail!("border от 0 до 12");
    }
    Ok(())
}

/// Оформление для шаблонов: готовые CSS-значения или null, если как в райсе.
fn look_context(look: &Look, spec: &RiceSpec) -> serde_json::Value {
    let palette = &spec.theme.palette;
    let color = |c: &Option<String>| c.as_deref().and_then(|c| palette.named(c));
    let rgba = |c: Color, a: f64| format!("rgba({}, {}, {}, {})", c.r, c.g, c.b, round2(a));
    let fill = match (look.fill.as_deref(), look.opacity) {
        (Some("none"), _) => Some("transparent".to_string()),
        (None, None) => None,
        (_, opacity) => {
            let base = color(&look.fill).unwrap_or(palette.bg);
            // Без opacity — как у остальных карточек; у сплошного цвета — непрозрачный.
            let style = StyleContext::new(spec.theme.style, &spec.theme.geometry);
            let theme = (style.bg_alpha - 0.25).max(0.15);
            let alpha = opacity.unwrap_or(if look.fill.is_some() { 1.0 } else { theme });
            Some(rgba(base, alpha))
        }
    };
    let text = color(&look.text);
    // Для QML: #AARRGGBB вместо rgba(…) из CSS.
    let argb = |c: Color, a: f64| {
        format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            (a.clamp(0.0, 1.0) * 255.0).round() as u8,
            c.r,
            c.g,
            c.b
        )
    };
    let fill_argb = match (look.fill.as_deref(), look.opacity) {
        (Some("none"), _) => Some("#00000000".to_string()),
        (None, None) => None,
        (_, opacity) => {
            let base = color(&look.fill).unwrap_or(palette.bg);
            let style = StyleContext::new(spec.theme.style, &spec.theme.geometry);
            let theme = (style.bg_alpha - 0.25).max(0.15);
            Some(argb(
                base,
                opacity.unwrap_or(if look.fill.is_some() { 1.0 } else { theme }),
            ))
        }
    };
    json!({
        "font": look.font,
        "accent": color(&look.accent).map(|c| c.hex()),
        "text": text.map(|c| c.hex()),
        "muted": text.map(|c| rgba(c, 0.65)),
        "muted_argb": text.map(|c| argb(c, 0.65)),
        "fill": fill,
        "fill_argb": fill_argb,
        "radius": look.radius,
        "border": look.border,
    })
}

/// Карточки для шаблонов eww: имя окна, размер в пикселях и параметры
/// с подставленными значениями по умолчанию.
pub fn context(spec: &RiceSpec) -> Vec<serde_json::Value> {
    spec.desktop
        .iter()
        .enumerate()
        .filter_map(|(i, card)| {
            let kind = kind(&card.kind)?;
            let mut opts = serde_json::Map::new();
            for param in kind.params {
                let value = card
                    .with
                    .get(param.id)
                    .and_then(|v| serde_json::to_value(v).ok())
                    .unwrap_or_else(|| match param.default {
                        Value::Text(t) => json!(t),
                        Value::Switch(b) => json!(b),
                    });
                opts.insert(param.id.into(), value);
            }
            // Пояса для мировых часов: список без пробелов по краям.
            let zones: Vec<String> = opts
                .get("zones")
                .and_then(|z| z.as_str())
                .map(|z| {
                    z.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            let mut size = kind.size;
            if kind.id == "worldclock" {
                size.1 = 40 + 40 * zones.len().max(1) as u32;
            }
            // Первое окно называется desktop, как у раскладки без studio:
            // по нему plove находит и останавливает этот eww.
            let window = if i == 0 {
                "desktop".to_string()
            } else {
                format!("desktop{i}")
            };
            Some(json!({
                "index": i,
                "window": window,
                "kind": kind.id,
                "x": round1(card.at[0]),
                "y": round1(card.at[1]),
                "scale": card.scale,
                "w": (size.0 as f64 * card.scale).round() as u32,
                "h": (size.1 as f64 * card.scale).round() as u32,
                "opts": opts,
                "zones": zones,
                "look": look_context(&card.look, spec),
            }))
        })
        .collect()
}

fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// Относительная яркость цвета по WCAG.
fn luminance(c: Color) -> f64 {
    let ch = |v: u8| {
        let v = v as f64 / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * ch(c.r) + 0.7152 * ch(c.g) + 0.0722 * ch(c.b)
}

/// Контраст двух цветов по WCAG: от 1 до 21.
pub fn contrast(a: Color, b: Color) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// Что лежит под каждой карточкой: средний цвет обоев в её прямоугольнике
/// (обои растянуты на экран с обрезкой, как swaybg -m fill) и цвет для
/// текста прямо на обоях — акцент, если он читается (контраст от 3),
/// иначе светлый или тёмный цвет палитры, что контрастнее. Нужно часам
/// без фона: розовые цифры на голубых обоях иначе не читаются.
pub fn add_backdrops(
    cards: &mut [serde_json::Value],
    wallpaper: &image::RgbImage,
    screen: (f64, f64),
    palette: &Palette,
) {
    let screen_aspect = screen.0 / screen.1;
    let (iw, ih) = (wallpaper.width() as f64, wallpaper.height() as f64);
    if iw < 1.0 || ih < 1.0 {
        return;
    }
    // Видимая часть картинки при заполнении экрана с обрезкой.
    let (vw, vh) = if iw / ih > screen_aspect {
        (ih * screen_aspect, ih)
    } else {
        (iw, iw / screen_aspect)
    };
    let (ox, oy) = ((iw - vw) / 2.0, (ih - vh) / 2.0);
    for card in cards.iter_mut() {
        let (Some(x), Some(y)) = (card["x"].as_f64(), card["y"].as_f64()) else {
            continue;
        };
        // Ширина и высота карточки в долях экрана.
        let w = card["w"].as_f64().unwrap_or(300.0) / screen.0;
        let h = card["h"].as_f64().unwrap_or(100.0) / screen.1;
        let x0 = (ox + vw * x / 100.0) as u32;
        let y0 = (oy + vh * y / 100.0) as u32;
        let x1 = ((ox + vw * (x / 100.0 + w)) as u32).min(wallpaper.width());
        let y1 = ((oy + vh * (y / 100.0 + h)) as u32).min(wallpaper.height());
        let (mut sum, mut n) = ([0u64; 3], 0u64);
        for py in (y0..y1).step_by(2) {
            for px in (x0..x1).step_by(2) {
                let p = wallpaper.get_pixel(px, py);
                sum[0] += p[0] as u64;
                sum[1] += p[1] as u64;
                sum[2] += p[2] as u64;
                n += 1;
            }
        }
        if n == 0 {
            continue;
        }
        let under = Color {
            r: (sum[0] / n) as u8,
            g: (sum[1] / n) as u8,
            b: (sum[2] / n) as u8,
        };
        let ink = if contrast(palette.accent, under) >= 3.0 {
            palette.accent
        } else if contrast(palette.fg, under) >= contrast(palette.bg, under) {
            palette.fg
        } else {
            palette.bg
        };
        // Тень — противоположная по яркости.
        let shade = if luminance(ink) > 0.4 {
            palette.bg
        } else {
            palette.fg
        };
        card["backdrop"] = json!({
            "under": under.hex(),
            "ink": ink.hex(),
            "shade": shade.hex(),
            "contrast": round2(contrast(ink, under)),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(desktop: &str) -> Result<RiceSpec> {
        let yaml = format!(
            "version: 1\nname: x\ntheme: {{palette: {{bg: '#000000', fg: '#ffffff', accent: '#ff0000'}}, fonts: {{ui: a, mono: b}}}}\ncomponents: {{compositor: hyprland}}\nwidgets: {{vpn: {{command: 'echo on'}}}}\ndesktop: {desktop}\n"
        );
        RiceSpec::from_yaml(&yaml)
    }

    /// Цвет текста на обоях: акцент, если читается, иначе светлый или
    /// тёмный цвет палитры — что контрастнее. Область — под карточкой.
    #[test]
    fn ink_on_wallpaper_is_readable() {
        let spec = spec("[{kind: clock, at: [0, 0]}, {kind: clock, at: [50, 0]}]").unwrap();
        let mut cards = context(&spec);
        // Левая половина светло-голубая, правая почти чёрная.
        let image = image::RgbImage::from_fn(160, 90, |x, _| {
            if x < 80 {
                image::Rgb([111, 182, 201])
            } else {
                image::Rgb([10, 10, 12])
            }
        });
        add_backdrops(&mut cards, &image, (1920.0, 1080.0), &spec.theme.palette);
        for card in &cards {
            let under = Color::parse(card["backdrop"]["under"].as_str().unwrap()).unwrap();
            let ink = Color::parse(card["backdrop"]["ink"].as_str().unwrap()).unwrap();
            assert!(contrast(ink, under) >= 3.0, "{card}");
        }
        assert_ne!(cards[0]["backdrop"]["ink"], cards[1]["backdrop"]["ink"]);
        assert!(
            (contrast(
                Color::parse("#000000").unwrap(),
                Color::parse("#ffffff").unwrap()
            ) - 21.0)
                .abs()
                < 0.01
        );
    }

    #[test]
    fn cards_are_checked() {
        assert!(spec("[{kind: clock, at: [10, 10]}]").is_ok());
        assert!(spec("[{kind: weather, at: [80, 5], with: {city: Москва}}]").is_ok());
        assert!(spec("[{kind: widget, at: [5, 5], with: {id: vpn}}]").is_ok());
        assert!(spec("[{kind: radio, at: [10, 10]}]").is_err());
        assert!(spec("[{kind: clock, at: [120, 10]}]").is_err());
        assert!(spec("[{kind: clock, at: [10, 10], with: {city: x}}]").is_err());
        assert!(spec("[{kind: clock, at: [10, 10], with: {seconds: yes please}}]").is_err());
        assert!(spec("[{kind: widget, at: [5, 5], with: {id: nope}}]").is_err());
        assert!(
            spec("[{kind: worldclock, at: [5, 5], with: {zones: \"x'; rm -rf ~ #\"}}]").is_err()
        );
        assert!(spec("[{kind: countdown, at: [5, 5], with: {date: 'завтра'}}]").is_err());
        assert!(spec("[{kind: weather, at: [5, 5], with: {city: \"a'b\"}}]").is_err());
    }

    #[test]
    fn context_fills_defaults_and_windows() {
        let s = spec(
            "[{kind: clock, at: [10, 10]}, {kind: worldclock, at: [70, 10], scale: 1.5, with: {zones: 'Europe/Moscow, Asia/Tokyo'}}]",
        )
        .unwrap();
        let cards = context(&s);
        assert_eq!(cards[0]["window"], "desktop");
        assert_eq!(cards[0]["opts"]["date"], true);
        assert_eq!(cards[1]["window"], "desktop1");
        assert_eq!(cards[1]["zones"], json!(["Europe/Moscow", "Asia/Tokyo"]));
        assert_eq!(cards[1]["w"], 450);
        assert_eq!(cards[1]["h"], 180);
    }

    #[test]
    fn roundtrip_keeps_layout() {
        let s = spec("[{kind: note, at: [12.5, 40], scale: 0.8, with: {text: привет}}]").unwrap();
        let back = RiceSpec::from_yaml(&s.to_yaml().unwrap()).unwrap();
        assert_eq!(back.desktop, s.desktop);
    }
}

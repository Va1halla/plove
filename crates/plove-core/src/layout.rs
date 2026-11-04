//! Конструктор оболочки: из каких поверхностей (панели у краёв экрана,
//! выезжающие панели) и блоков (часы, кнопка, вывод команды…) состоит
//! интерфейс на Quickshell. Это только данные — их собирают мышью в
//! `plove studio` (страница «Оболочка»), а панель читает их из
//! settings.json и строит окна сама, QML писать не нужно.
//!
//! В райсе: `shell: [{ id: top, kind: bar, edge: top, sections: [[…], […], […]] }]`.
//! Пустой список — прежняя панель по `modules_left/center/right`.

use std::collections::BTreeSet;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::spec::RiceSpec;

/// Поверхность: окно у края экрана со своими блоками.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Surface {
    /// Имя, по нему кнопка открывает выезжающую панель.
    pub id: String,
    /// `bar` — всегда на экране; `drawer` — выезжает по кнопке.
    pub kind: String,
    /// У какого края: top, bottom, left, right.
    pub edge: String,
    /// Вдоль края: `fill` — во всю длину, `center` — островок посередине,
    /// `start` / `end` — у начала или конца края.
    #[serde(default = "fill", skip_serializing_if = "is_fill")]
    pub align: String,
    /// Толщина панели или ширина выезжающей, px; 0 — по умолчанию.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub size: u32,
    /// Блоки: у панели три группы (начало, середина, конец), у выезжающей — одна.
    #[serde(default)]
    pub sections: Vec<Vec<Block>>,
}

/// Блок: `{ kind: button, with: { command: kitty } }`, без параметров —
/// просто имя: `clock`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(from = "BlockRepr")]
pub struct Block {
    pub kind: String,
    pub with: serde_yaml::Mapping,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum BlockRepr {
    Name(String),
    Full {
        kind: String,
        #[serde(default)]
        with: serde_yaml::Mapping,
    },
}

impl From<BlockRepr> for Block {
    fn from(r: BlockRepr) -> Self {
        match r {
            BlockRepr::Name(kind) => Block {
                kind,
                with: Default::default(),
            },
            BlockRepr::Full { kind, with } => Block { kind, with },
        }
    }
}

impl Serialize for Block {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        if self.with.is_empty() {
            return s.serialize_str(&self.kind);
        }
        let mut map = s.serialize_map(Some(2))?;
        map.serialize_entry("kind", &self.kind)?;
        map.serialize_entry("with", &self.with)?;
        map.end()
    }
}

fn fill() -> String {
    "fill".into()
}

fn is_fill(s: &String) -> bool {
    s == "fill"
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

pub const KINDS: [&str; 2] = ["bar", "drawer"];
pub const EDGES: [&str; 4] = ["top", "bottom", "left", "right"];
pub const ALIGNS: [&str; 4] = ["fill", "center", "start", "end"];

/// Вид блока: что показывает, где может стоять, какие параметры.
pub struct BlockKind {
    pub id: &'static str,
    pub title: &'static str,
    /// Значок Nerd Font для палитры в studio.
    pub icon: &'static str,
    pub about: &'static str,
    /// `bar`, `drawer` или обе.
    pub fits: &'static [&'static str],
    pub params: &'static [Param],
}

pub struct Param {
    pub id: &'static str,
    pub title: &'static str,
    pub default: Value,
}

/// Значение по умолчанию; от него же зависит, каким полем его править.
pub enum Value {
    Text(&'static str),
    Switch(bool),
    Number(i64, i64, i64),
    Choice(&'static str, &'static [&'static str]),
}

const BAR: &[&str] = &["bar"];
const ANY: &[&str] = &["bar", "drawer"];
const DRAWER: &[&str] = &["drawer"];

/// Все виды блоков. Модули панели — те же, что в `modules_left` и т.д.
pub const BLOCKS: [BlockKind; 17] = [
    BlockKind {
        id: "workspaces",
        title: "Рабочие места",
        icon: "\u{f0ea9}",
        about: "Точки рабочих мест, активное — цветом акцента.",
        fits: BAR,
        params: &[],
    },
    BlockKind {
        id: "clock",
        title: "Часы",
        icon: "\u{f0954}",
        about: "Время; щелчок — календарь.",
        fits: BAR,
        params: &[],
    },
    BlockKind {
        id: "tray",
        title: "Трей",
        icon: "\u{f0b2c}",
        about: "Значки программ в фоне.",
        fits: BAR,
        params: &[],
    },
    BlockKind {
        id: "volume",
        title: "Громкость",
        icon: "\u{f057e}",
        about: "Уровень звука; колесо меняет, щелчок — меню.",
        fits: BAR,
        params: &[],
    },
    BlockKind {
        id: "network",
        title: "Сеть",
        icon: "\u{f05a9}",
        about: "Подключение; щелчок — меню сети.",
        fits: BAR,
        params: &[],
    },
    BlockKind {
        id: "battery",
        title: "Батарея",
        icon: "\u{f0079}",
        about: "Заряд; без батареи прячется.",
        fits: BAR,
        params: &[],
    },
    BlockKind {
        id: "calculator",
        title: "Калькулятор",
        icon: "\u{f00ec}",
        about: "Кнопка калькулятора.",
        fits: BAR,
        params: &[],
    },
    BlockKind {
        id: "bell",
        title: "Уведомления",
        icon: "\u{f009a}",
        about: "Колокольчик со счётчиком новых.",
        fits: BAR,
        params: &[],
    },
    BlockKind {
        id: "text",
        title: "Надпись",
        icon: "\u{f0284}",
        about: "Любой текст, например имя или девиз.",
        fits: ANY,
        params: &[Param {
            id: "text",
            title: "Текст",
            default: Value::Text("Привет"),
        }],
    },
    BlockKind {
        id: "button",
        title: "Кнопка",
        icon: "\u{f0a79}",
        about: "Значок с подписью: запускает команду или открывает выезжающую панель.",
        fits: ANY,
        params: &[
            Param {
                id: "icon",
                title: "Значок",
                default: Value::Text("\u{f018d}"),
            },
            Param {
                id: "label",
                title: "Подпись",
                default: Value::Text(""),
            },
            Param {
                id: "command",
                title: "Команда",
                default: Value::Text("kitty"),
            },
            Param {
                id: "opens",
                title: "Или открыть панель (её имя)",
                default: Value::Text(""),
            },
        ],
    },
    BlockKind {
        id: "command",
        title: "Вывод команды",
        icon: "\u{f018d}",
        about: "Последняя строка вывода команды, обновляется сама.",
        fits: ANY,
        params: &[
            Param {
                id: "icon",
                title: "Значок",
                default: Value::Text("\u{f0954}"),
            },
            Param {
                id: "command",
                title: "Команда",
                default: Value::Text("uptime -p"),
            },
            Param {
                id: "every",
                title: "Раз в сколько секунд",
                default: Value::Number(30, 1, 3600),
            },
        ],
    },
    BlockKind {
        id: "spacer",
        title: "Отступ",
        icon: "\u{f0e8e}",
        about: "Пустое место между блоками.",
        fits: ANY,
        params: &[Param {
            id: "size",
            title: "Размер, px",
            default: Value::Number(16, 2, 400),
        }],
    },
    BlockKind {
        id: "heading",
        title: "Заголовок",
        icon: "\u{f0274}",
        about: "Крупная строка, чтобы разделить панель на части.",
        fits: DRAWER,
        params: &[Param {
            id: "text",
            title: "Текст",
            default: Value::Text("Раздел"),
        }],
    },
    BlockKind {
        id: "slider",
        title: "Ползунок",
        icon: "\u{f0a5e}",
        about: "Громкость или яркость.",
        fits: DRAWER,
        params: &[Param {
            id: "what",
            title: "Что меняет",
            default: Value::Choice("volume", &["volume", "brightness"]),
        }],
    },
    BlockKind {
        id: "media",
        title: "Плеер",
        icon: "\u{f075a}",
        about: "Обложка, название, кнопки; прячется, когда ничего не играет.",
        fits: DRAWER,
        params: &[],
    },
    BlockKind {
        id: "calendar",
        title: "Календарь",
        icon: "\u{f00ed}",
        about: "Месяц с сегодняшним днём.",
        fits: DRAWER,
        params: &[],
    },
    BlockKind {
        id: "clockbig",
        title: "Крупные часы",
        icon: "\u{f0150}",
        about: "Время и дата крупно.",
        fits: DRAWER,
        params: &[Param {
            id: "seconds",
            title: "Секунды",
            default: Value::Switch(false),
        }],
    },
];

pub fn block(id: &str) -> Option<&'static BlockKind> {
    BLOCKS.iter().find(|b| b.id == id)
}

/// Каталог для studio: виды блоков с параметрами и тем, где они могут стоять.
pub fn catalog_json() -> serde_json::Value {
    BLOCKS
        .iter()
        .map(|b| {
            let params: Vec<serde_json::Value> = b
                .params
                .iter()
                .map(|p| match p.default {
                    Value::Text(t) => {
                        json!({ "id": p.id, "title": p.title, "type": "text", "default": t })
                    }
                    Value::Switch(v) => {
                        json!({ "id": p.id, "title": p.title, "type": "switch", "default": v })
                    }
                    Value::Number(v, min, max) => json!({
                        "id": p.id, "title": p.title, "type": "number",
                        "default": v, "min": min, "max": max,
                    }),
                    Value::Choice(v, choices) => json!({
                        "id": p.id, "title": p.title, "type": "choice",
                        "default": v, "choices": choices,
                    }),
                })
                .collect();
            json!({
                "id": b.id,
                "title": b.title,
                "icon": b.icon,
                "about": b.about,
                "fits": b.fits,
                "params": params,
            })
        })
        .collect()
}

/// Сколько групп блоков у поверхности.
fn section_count(kind: &str) -> usize {
    if kind == "drawer" {
        1
    } else {
        3
    }
}

pub fn validate(spec: &RiceSpec) -> Result<()> {
    let mut ids = BTreeSet::new();
    for (i, s) in spec.shell.iter().enumerate() {
        let n = i + 1;
        let simple = !s.id.is_empty()
            && s.id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !simple {
            bail!(
                "shell, поверхность {n}: имя «{}» — латиница, цифры и -",
                s.id
            );
        }
        if !ids.insert(s.id.as_str()) {
            bail!("shell: имя «{}» встречается дважды", s.id);
        }
        let one_of = |what: &str, value: &str, allowed: &[&str]| -> Result<()> {
            if !allowed.contains(&value) {
                bail!(
                    "shell, «{}»: {what} «{value}» — есть {}",
                    s.id,
                    allowed.join(", ")
                );
            }
            Ok(())
        };
        one_of("kind", &s.kind, &KINDS)?;
        one_of("edge", &s.edge, &EDGES)?;
        one_of("align", &s.align, &ALIGNS)?;
        if s.size > 1200 {
            bail!("shell, «{}»: size не больше 1200 px", s.id);
        }
        if s.sections.len() > section_count(&s.kind) {
            bail!(
                "shell, «{}»: у {} групп блоков не больше {}",
                s.id,
                s.kind,
                section_count(&s.kind)
            );
        }
        for b in s.sections.iter().flatten() {
            check_block(spec, s, b)?;
        }
    }
    // Кнопка может открыть только существующую выезжающую панель.
    for s in &spec.shell {
        for b in s.sections.iter().flatten() {
            let opens = b.with.get("opens").and_then(|v| v.as_str()).unwrap_or("");
            if !opens.is_empty()
                && !spec
                    .shell
                    .iter()
                    .any(|d| d.id == opens && d.kind == "drawer")
            {
                bail!(
                    "shell, «{}»: кнопка открывает «{opens}», а такой выезжающей панели нет",
                    s.id
                );
            }
        }
    }
    Ok(())
}

fn check_block(spec: &RiceSpec, s: &Surface, b: &Block) -> Result<()> {
    // Свои виджеты райса ставятся на панель по имени, как готовые.
    if spec.widgets.contains_key(&b.kind) {
        if s.kind != "bar" {
            bail!(
                "shell, «{}»: свой виджет «{}» — только на панель",
                s.id,
                b.kind
            );
        }
        return Ok(());
    }
    let Some(kind) = block(&b.kind) else {
        let names: Vec<&str> = BLOCKS.iter().map(|k| k.id).collect();
        bail!(
            "shell, «{}»: блока «{}» нет. Есть: {}",
            s.id,
            b.kind,
            names.join(", ")
        );
    };
    if !kind.fits.contains(&s.kind.as_str()) {
        bail!(
            "shell, «{}»: «{}» не ставится на {}",
            s.id,
            kind.title,
            s.kind
        );
    }
    for (key, value) in &b.with {
        let key = key.as_str().unwrap_or_default();
        let Some(param) = kind.params.iter().find(|p| p.id == key) else {
            bail!("shell, «{}», {}: параметра «{key}» нет", s.id, kind.id);
        };
        let fits = match param.default {
            Value::Text(_) => value.is_string(),
            Value::Switch(_) => value.is_bool(),
            Value::Number(_, min, max) => value.as_i64().is_some_and(|v| (min..=max).contains(&v)),
            Value::Choice(_, choices) => value.as_str().is_some_and(|v| choices.contains(&v)),
        };
        if !fits {
            bail!("shell, «{}», {}: неверное значение {key}", s.id, kind.id);
        }
    }
    Ok(())
}

/// Раскладка для settings.json: умолчания подставлены, у панели всегда
/// три группы, у каждого блока — все его параметры.
pub fn context(spec: &RiceSpec) -> serde_json::Value {
    spec.shell
        .iter()
        .map(|s| {
            let count = section_count(&s.kind);
            let sections: Vec<serde_json::Value> = (0..count)
                .map(|i| {
                    let blocks: Vec<serde_json::Value> = s
                        .sections
                        .get(i)
                        .into_iter()
                        .flatten()
                        .map(|b| {
                            let mut opts = serde_json::Map::new();
                            if let Some(kind) = block(&b.kind) {
                                for p in kind.params {
                                    let default = match p.default {
                                        Value::Text(t) => json!(t),
                                        Value::Switch(v) => json!(v),
                                        Value::Number(v, _, _) => json!(v),
                                        Value::Choice(v, _) => json!(v),
                                    };
                                    opts.insert(p.id.into(), default);
                                }
                            }
                            for (k, v) in &b.with {
                                if let (Some(k), Ok(v)) = (k.as_str(), serde_json::to_value(v)) {
                                    opts.insert(k.into(), v);
                                }
                            }
                            json!({ "kind": b.kind, "opts": opts })
                        })
                        .collect();
                    json!(blocks)
                })
                .collect();
            let size = match (s.size, s.kind.as_str()) {
                (0, "drawer") => 380,
                (0, _) => 40,
                (n, _) => n,
            };
            json!({
                "id": s.id,
                "kind": s.kind,
                "edge": s.edge,
                "align": s.align,
                "size": size,
                "sections": sections,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(yaml: &str) -> RiceSpec {
        let base = format!(
            "version: 1\nname: t\ntheme: {{palette: {{bg: '#000000', fg: '#ffffff', accent: '#ff0000'}}, fonts: {{ui: a, mono: b}}}}\ncomponents: {{compositor: hyprland}}\n{yaml}"
        );
        serde_yaml::from_str(&base).expect("райс разбирается")
    }

    #[test]
    fn layout_is_checked() {
        let ok = spec(
            "shell:\n  - { id: top, kind: bar, edge: top, sections: [[workspaces], [clock], [{ kind: button, with: { opens: side } }]] }\n  - { id: side, kind: drawer, edge: right, sections: [[{ kind: slider, with: { what: brightness } }, media]] }\n",
        );
        validate(&ok).unwrap();

        let bad = |yaml: &str| validate(&spec(yaml)).unwrap_err().to_string();
        assert!(bad("shell: [{ id: Top, kind: bar, edge: top }]").contains("латиница"));
        assert!(bad("shell: [{ id: a, kind: bar, edge: middle }]").contains("edge"));
        assert!(
            bad("shell: [{ id: a, kind: bar, edge: top, sections: [[media]] }]")
                .contains("не ставится")
        );
        assert!(bad("shell: [{ id: a, kind: bar, edge: top, sections: [[{ kind: button, with: { opens: nope } }]] }]").contains("nope"));
        assert!(
            bad("shell: [{ id: a, kind: drawer, edge: top, sections: [[], []] }]")
                .contains("не больше 1")
        );
        assert!(bad("shell: [{ id: a, kind: drawer, edge: top, sections: [[{ kind: slider, with: { what: bass } }]] }]").contains("what"));
    }

    #[test]
    fn context_fills_defaults() {
        let s = spec("shell:\n  - { id: top, kind: bar, edge: top, sections: [[], [{ kind: command, with: { command: date } }]] }\n");
        let c = context(&s);
        assert_eq!(c[0]["size"], 40);
        assert_eq!(c[0]["align"], "fill");
        assert_eq!(c[0]["sections"].as_array().unwrap().len(), 3);
        let block = &c[0]["sections"][1][0];
        assert_eq!(block["opts"]["command"], "date");
        assert_eq!(block["opts"]["every"], 30);
    }

    #[test]
    fn short_block_names_are_kept() {
        // `[clock]` — то же, что `[{ kind: clock }]`.
        let s = spec("shell: [{ id: top, kind: bar, edge: top, sections: [[clock]] }]");
        assert_eq!(s.shell[0].sections[0][0].kind, "clock");
        let yaml = s.to_yaml().unwrap();
        assert!(yaml.contains("shell:"), "{yaml}");
    }
}

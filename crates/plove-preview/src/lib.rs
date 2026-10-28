//! Превью райса: одна картинка, собранная из слоёв.
//!
//! ```text
//! 1. обои          (нарисованные из палитры или файл пользователя)
//! 2. окна          (два терминала kitty с размытием фона, рамками, текстом)
//! 3. панель        (Waybar с рабочими столами, часами и модулями)
//! ```
//!
//! Это не настоящий скриншот, а макет по тем же токенам темы: цвета,
//! отступы, скругления, прозрачность и расположение панели совпадают,
//! но шрифт один на всё превью (JetBrains Mono), а содержимое окон выдумано.

use anyhow::{anyhow, Result};
use fontdue::{Font, FontSettings};
use plove_core::modules::Registry;
use plove_core::plan::module_options;
use plove_core::spec::{BarStyle, Color, Labels, Palette, RiceSpec, Separators, Wallpaper, Widget};
use plove_core::system::Env;
use plove_core::terminal;
use plove_core::theme::StyleContext;
use plove_core::wallpaper::{self, paint};
use std::collections::BTreeMap;
pub use tiny_skia::Pixmap;
use tiny_skia::{
    FillRule, FilterQuality, Mask, Path, PathBuilder, PixmapPaint, Rect, Stroke, Transform,
};

/// Размер превью: обычный Full HD монитор.
pub const WIDTH: u32 = 1920;
pub const HEIGHT: u32 = 1080;

/// Нарисовать превью и сохранить в PNG.
pub fn render_png(spec: &RiceSpec, registry: &Registry, env: &Env) -> Result<Vec<u8>> {
    render(spec, registry, env)?
        .encode_png()
        .map_err(|e| anyhow!("не удалось сохранить превью в PNG: {e}"))
}

/// Нарисовать превью.
pub fn render(spec: &RiceSpec, registry: &Registry, env: &Env) -> Result<Pixmap> {
    // Настройки композитора (тени, размытие) — с тонкой настройки райса.
    let compositor = match spec
        .components
        .get("compositor")
        .and_then(|id| registry.get(id))
    {
        Some(module) => module_options(module, spec.overrides.get(&module.manifest.id))?,
        None => serde_json::Value::Null,
    };
    let look = Look::new(spec, &compositor);
    let fonts = Fonts::load()?;
    let mut canvas = background(spec, env)?;
    // Виджеты eww лежат на обоях под окнами (stacking "bg").
    if let Some(id) = spec.components.get("widgets") {
        if let Some(module) = registry.get(id) {
            let options = module_options(module, spec.overrides.get(id))?;
            draw_desktop(&mut canvas, &look, &fonts, &options, spec);
        }
    }

    let bar = match spec.components.get("bar") {
        Some(id) => Some(Bar::from_spec(spec, registry, id, &look)?),
        None => None,
    };
    if let Some(id) = spec.components.get("terminal") {
        // Своя цветовая схема терминала: окна рисуются её цветами.
        let mut term_spec = spec.clone();
        if let Some(module) = registry.get(id) {
            let options = module_options(module, spec.overrides.get(id))?;
            if let Some(palette) = terminal::from_options(&options, &spec.theme.palette)? {
                term_spec.theme.palette = palette;
            }
        }
        let term_look = Look::new(&term_spec, &compositor);
        draw_windows(&mut canvas, &look, &term_look, &fonts, bar.as_ref(), spec);
    }
    if let Some(bar) = &bar {
        draw_bar(&mut canvas, &look, &fonts, bar, spec);
    }
    Ok(canvas)
}

/// Уменьшить превью до `width`×`height` точек для показа в терминале.
///
/// Каждая точка — средний цвет своего прямоугольника исходной картинки,
/// поэтому тонкие линии (рамки, текст) не пропадают, а смешиваются с фоном.
pub fn thumbnail(image: &Pixmap, width: u32, height: u32) -> Vec<[u8; 3]> {
    let (iw, ih) = (image.width(), image.height());
    let pixels = image.pixels();
    let mut out = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        let (y0, y1) = (
            y * ih / height,
            ((y + 1) * ih / height).max(y * ih / height + 1),
        );
        for x in 0..width {
            let (x0, x1) = (
                x * iw / width,
                ((x + 1) * iw / width).max(x * iw / width + 1),
            );
            let mut sum = [0u32; 3];
            for row in y0..y1.min(ih) {
                for p in &pixels[(row * iw + x0) as usize..(row * iw + x1.min(iw)) as usize] {
                    // Превью непрозрачное, так что премультипликация не мешает.
                    sum[0] += p.red() as u32;
                    sum[1] += p.green() as u32;
                    sum[2] += p.blue() as u32;
                }
            }
            let n = ((y1.min(ih) - y0) * (x1.min(iw) - x0)).max(1);
            out.push(sum.map(|c| (c / n) as u8));
        }
    }
    out
}

// ───────────────────────── тема ─────────────────────────

/// Всё, что нужно для рисования, в одном месте и в пикселях.
struct Look {
    bg: Color,
    fg: Color,
    accent: Color,
    surface: Color,
    muted: Color,
    ansi: Vec<Color>,
    gaps: f32,
    border: f32,
    radius: f32,
    opacity: f32,
    opacity_inactive: f32,
    /// Стиль райса: форма, вид панели, отступы — как в шаблонах.
    style: StyleContext,
    /// Размер шрифта в пикселях: пункты × 4/3 при обычных 96 dpi.
    font_px: f32,
    /// Тень под окнами: размер и смещение вниз, None — без тени.
    shadow: Option<(f32, f32)>,
    /// Радиус размытия под прозрачными окнами, 0 — без размытия.
    blur: usize,
}

impl Look {
    fn new(spec: &RiceSpec, compositor: &serde_json::Value) -> Look {
        let p = &spec.theme.palette;
        let g = &spec.theme.geometry;
        // Если цвета терминала не заданы, берём нейтральный набор.
        let ansi = if p.ansi.len() == 16 {
            p.ansi.clone()
        } else {
            let base = [
                "#3b4048", "#e06c75", "#98c379", "#e5c07b", "#61afef", "#c678dd", "#56b6c2",
                "#abb2bf",
            ];
            let colors: Vec<Color> = base.iter().map(|c| Color::parse(c).unwrap()).collect();
            colors.iter().chain(colors.iter()).copied().collect()
        };
        let style = StyleContext::new(spec.theme.style, g);
        // blur_size в Hyprland — радиус одного прохода; у нас проходов три.
        let blur = if style.blur {
            (compositor["blur_size"].as_f64().unwrap_or(6.0) * 1.6).round() as usize
        } else {
            0
        };
        Look {
            bg: p.bg,
            fg: p.fg,
            accent: p.accent,
            // Те же производные цвета, что и в theme.rs.
            surface: p.bg.mix(p.fg, 0.15),
            muted: p.bg.mix(p.fg, 0.5),
            ansi,
            gaps: g.gaps as f32,
            border: style.border as f32,
            radius: style.radius_window as f32,
            style,
            opacity: g.opacity as f32,
            opacity_inactive: (g.opacity as f32 - 0.05).max(0.0),
            font_px: spec.theme.fonts.size as f32 * 4.0 / 3.0,
            shadow: match compositor["shadows"].as_bool() {
                Some(false) => None,
                _ => Some((
                    compositor["shadow_range"].as_f64().unwrap_or(18.0) as f32,
                    compositor["shadow_offset"].as_f64().unwrap_or(0.0) as f32,
                )),
            },
            blur,
        }
    }
}

// ───────────────────────── слой 1: обои ─────────────────────────

fn background(spec: &RiceSpec, env: &Env) -> Result<Pixmap> {
    let palette = &spec.theme.palette;
    let shown = spec.components.contains_key("wallpaper");
    match (&spec.theme.wallpaper, shown) {
        (Some(Wallpaper::Generated { generate }), true) => {
            wallpaper::generate(*generate, palette, WIDTH, HEIGHT)
        }
        (Some(Wallpaper::File(file)), true) => {
            let path = env.expand(file);
            let image = wallpaper::load(&path)?;
            Ok(cover(&image))
        }
        // Обоев нет: swaybg или Hyprland покажут сплошной цвет фона.
        _ => {
            let mut canvas = Pixmap::new(WIDTH, HEIGHT).ok_or_else(|| anyhow!("пустой холст"))?;
            canvas.fill(wallpaper::sk(palette.bg, 1.0));
            Ok(canvas)
        }
    }
}

/// Сама картинка обоев в размере превью — пока райса из неё ещё нет.
pub fn wallpaper_only(path: &std::path::Path) -> Result<Pixmap> {
    Ok(cover(&wallpaper::load(path)?))
}

/// Растянуть картинку на весь экран с обрезкой краёв, как `swaybg -m fill`.
fn cover(image: &Pixmap) -> Pixmap {
    let mut canvas = Pixmap::new(WIDTH, HEIGHT).expect("размер превью ненулевой");
    let (iw, ih) = (image.width() as f32, image.height() as f32);
    let scale = (WIDTH as f32 / iw).max(HEIGHT as f32 / ih);
    let dx = (WIDTH as f32 - iw * scale) / 2.0;
    let dy = (HEIGHT as f32 - ih * scale) / 2.0;
    let quality = PixmapPaint {
        quality: FilterQuality::Bilinear,
        ..PixmapPaint::default()
    };
    let transform = Transform::from_row(scale, 0.0, 0.0, scale, dx, dy);
    canvas.draw_pixmap(0, 0, image.as_ref(), &quality, transform, None);
    canvas
}

// ───────────────────────── слой 2: окна ─────────────────────────

/// Два плавающих окна kitty поверх обоев, как на скриншотах с r/unixporn:
/// слева fastfetch (активное окно), справа снизу оболочка с файлом райса.
/// Плавающие, а не тайловые, чтобы обои было видно.
/// `look` — рамки и отступы райса, `term` — цвета внутри терминала.
fn draw_windows(
    canvas: &mut Pixmap,
    look: &Look,
    term_look: &Look,
    fonts: &Fonts,
    bar: Option<&Bar>,
    spec: &RiceSpec,
) {
    // Панель резервирует место у края экрана вместе со своим отступом.
    let (mut top, mut bottom, mut left, mut right) = (0.0, 0.0, 0.0, 0.0);
    if let Some(bar) = bar {
        let reserved = bar.margin + bar.height;
        if let Some(on_left) = bar.side {
            let reserved = bar.margin + bar.height + 12.0;
            if on_left {
                left = reserved;
            } else {
                right = reserved;
            }
        } else if bar.top {
            top = reserved;
        } else {
            bottom = reserved;
        }
    }
    let (x0, y0) = (left + look.gaps, top + look.gaps);
    let (x1, y1) = (
        WIDTH as f32 - right - look.gaps,
        HEIGHT as f32 - bottom - look.gaps,
    );
    let (margin_x, margin_y) = ((x1 - x0) * 0.06, (y1 - y0) * 0.08);

    let (w, h) = window_size(fonts, look, 60, 17);
    let session_area = rect(x1 - margin_x - w, y1 - margin_y - h, w, h);
    let mut term = window(canvas, look, term_look, fonts, session_area, false);
    session(canvas, &mut term, term_look, spec);

    let (w, h) = window_size(fonts, look, 78, 24);
    let fetch_area = rect(x0 + margin_x, y0 + margin_y, w, h);
    let mut term = window(canvas, look, term_look, fonts, fetch_area, true);
    fastfetch(canvas, &mut term, term_look, spec);
}

/// Отступ текста от краёв окна: как `window_padding_width` в kitty плюс рамка.
fn window_padding(look: &Look) -> f32 {
    look.gaps.max(6.0) + 6.0
}

/// Размер окна, в которое влезает `cols` × `rows` символов.
fn window_size(fonts: &Fonts, look: &Look, cols: usize, rows: usize) -> (f32, f32) {
    let padding = window_padding(look);
    (
        cols as f32 * fonts.char_width(look.font_px) + 2.0 * padding,
        rows as f32 * line_height(look.font_px) + 2.0 * padding,
    )
}

fn line_height(px: f32) -> f32 {
    (px * 1.45).round()
}

/// Нарисовать рамку окна и вернуть «терминал» для текста внутри.
fn window<'a>(
    canvas: &mut Pixmap,
    look: &Look,
    term_look: &Look,
    fonts: &'a Fonts,
    area: Rect,
    active: bool,
) -> Term<'a> {
    let shape = rounded_rect(area, look.radius);
    let opacity = if active {
        look.opacity
    } else {
        look.opacity_inactive
    };
    if let Some((range, offset)) = look.shadow {
        draw_shadow(canvas, area, look.radius, range, offset, term_look.bg);
    }
    // Полупрозрачное окно в Hyprland размывает то, что под ним.
    if opacity < 1.0 && look.blur > 0 {
        blur_under(canvas, area, &shape, look.blur);
    }
    canvas.fill_path(
        &shape,
        &paint(term_look.bg, opacity),
        FillRule::Winding,
        Transform::identity(),
        None,
    );

    let border = if active { look.accent } else { look.surface };
    let line = Stroke {
        width: look.border,
        ..Stroke::default()
    };
    if look.border > 0.0 {
        canvas.stroke_path(
            &shape,
            &paint(border, 1.0),
            &line,
            Transform::identity(),
            None,
        );
    }

    Term::new(fonts, look.font_px, area, window_padding(look))
}

fn fastfetch(canvas: &mut Pixmap, term: &mut Term, look: &Look, spec: &RiceSpec) {
    let a = &look.ansi;
    term.line(canvas, &[("❯ ", a[2]), ("fastfetch", look.fg)]);
    term.blank();

    let mono = spec.theme.fonts.mono.as_str();
    let wm = spec
        .components
        .get("compositor")
        .map_or("—", String::as_str);
    let bar = spec.components.get("bar").map_or("—", String::as_str);
    let terminal = spec.components.get("terminal").map_or("—", String::as_str);
    let info: Vec<(&str, &str)> = vec![
        ("OS", "Arch Linux x86_64"),
        ("Kernel", "6.17.5-arch1-1"),
        ("WM", wm),
        ("Bar", bar),
        ("Terminal", terminal),
        ("Font", mono),
        ("Rice", spec.name.as_str()),
    ];

    // Логотип Arch слева, сведения справа, как у настоящего fastfetch.
    let logo_cols = ARCH_LOGO
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0)
        + 3;
    let top = term.row;
    for (i, art) in ARCH_LOGO.iter().enumerate() {
        term.row = top + i;
        term.put(canvas, 0, art, look.accent, false);
    }
    term.row = top + 2;
    term.put(canvas, logo_cols, "user", look.accent, true);
    term.put(canvas, logo_cols + 4, "@", look.fg, false);
    term.put(canvas, logo_cols + 5, "plove", look.accent, true);
    term.row += 1;
    term.put(canvas, logo_cols, "----------", look.muted, false);
    for (key, value) in info {
        term.row += 1;
        term.put(canvas, logo_cols, key, look.accent, true);
        term.put(canvas, logo_cols + 10, value, look.fg, false);
    }
    term.row += 2;
    term.blocks(canvas, logo_cols, &a[0..8]);
    term.row += 1;
    term.blocks(canvas, logo_cols, &a[8..16]);
    term.row = (top + ARCH_LOGO.len() + 1).max(term.row + 1);
    term.line(canvas, &[("❯ ", a[2])]);
    term.cursor(canvas, 2, look.fg);
}

/// Оболочка: применение райса и начало его файла с подсветкой YAML.
fn session(canvas: &mut Pixmap, term: &mut Term, look: &Look, spec: &RiceSpec) {
    let a = &look.ansi;
    let p = &spec.theme.palette;
    let name = spec.name.as_str();
    term.line(
        canvas,
        &[("❯ ", a[2]), ("plove apply ", look.fg), (name, a[5])],
    );
    term.line(
        canvas,
        &[
            ("Готово: ", a[2]),
            ("записано файлов 5, поколение #1", look.fg),
        ],
    );
    term.blank();
    term.line(
        canvas,
        &[
            ("❯ ", a[2]),
            ("cat ", look.fg),
            (name, a[5]),
            (".yaml", a[5]),
        ],
    );
    let (bg, fg, accent) = (p.bg.hex(), p.fg.hex(), p.accent.hex());
    let mut lines: Vec<(String, String)> = vec![
        ("name".into(), name.into()),
        ("theme".into(), String::new()),
        ("  bg".into(), bg),
        ("  fg".into(), fg),
        ("  accent".into(), accent),
        ("components".into(), String::new()),
    ];
    for (role, id) in &spec.components {
        lines.push((format!("  {role}"), id.clone()));
    }
    for (key, value) in &lines {
        term.line(
            canvas,
            &[
                (key, a[4]),
                (":", look.muted),
                (" ", look.fg),
                (value, a[2]),
            ],
        );
    }
    term.line(canvas, &[("❯ ", a[2])]);
    term.cursor(canvas, 2, look.fg);
}

const ARCH_LOGO: [&str; 19] = [
    "                  -`",
    "                 .o+`",
    "                `ooo/",
    "               `+oooo:",
    "              `+oooooo:",
    "              -+oooooo+:",
    "            `/:-:++oooo+:",
    "           `/++++/+++++++:",
    "          `/++++++++++++++:",
    "         `/+++ooooooooooooo/`",
    "        ./ooosssso++osssssso+`",
    "       .oossssso-````/ossssss+`",
    "      -osssssso.      :ssssssso.",
    "     :osssssss/        osssso+++.",
    "    /ossssssss/        +ssssooo/-",
    "  `/ossssso+/:-        -:/+osssso+-",
    " `+sso+:-`                 `.-/+oso:",
    "`++:.                           `-/+/",
    ".`                                 `/",
];

// ───────────────────────── слой 3: панель ─────────────────────────

struct Bar {
    top: bool,
    /// Вертикальная панель: Some(true) — слева, Some(false) — справа.
    side: Option<bool>,
    floating: bool,
    style: BarStyle,
    height: f32,
    /// Отступ панели от края экрана (только у плавающей).
    margin: f32,
    left: Vec<String>,
    center: Vec<String>,
    right: Vec<String>,
    /// Фон панели (настройка background): цвета начала и конца градиента,
    /// у сплошного фона они одинаковые. Картинки превью не рисует.
    fill: (Color, Color),
}

impl Bar {
    /// Кисть фона панели для плашки `area`: градиент слева направо, как
    /// linear-gradient(90deg, …) в style.css.tera.
    fn paint(&self, area: Rect, alpha: f32) -> tiny_skia::Paint<'static> {
        let (from, to) = self.fill;
        let shader = tiny_skia::LinearGradient::new(
            tiny_skia::Point::from_xy(area.left(), 0.0),
            tiny_skia::Point::from_xy(area.right(), 0.0),
            vec![
                tiny_skia::GradientStop::new(0.0, wallpaper::sk(from, alpha)),
                tiny_skia::GradientStop::new(1.0, wallpaper::sk(to, alpha)),
            ],
            tiny_skia::SpreadMode::Pad,
            Transform::identity(),
        );
        match shader {
            Some(shader) if from != to => tiny_skia::Paint {
                shader,
                anti_alias: true,
                ..tiny_skia::Paint::default()
            },
            _ => paint(from, alpha),
        }
    }
}

impl Bar {
    /// Прочитать настройки панели так же, как их видит шаблон Waybar.
    fn from_spec(spec: &RiceSpec, registry: &Registry, id: &str, look: &Look) -> Result<Bar> {
        let module = registry
            .get(id)
            .ok_or_else(|| anyhow!("модуль «{id}» не найден"))?;
        let options = module_options(module, spec.overrides.get(id))?;
        let list = |key: &str| -> Vec<String> {
            options[key]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default()
        };
        let p = &spec.theme.palette;
        let color = |key: &str| {
            options[key]
                .as_str()
                .filter(|s| !s.is_empty())
                .and_then(|s| Color::parse(s).ok())
        };
        let magenta = p.ansi.get(5).copied().unwrap_or(p.accent);
        let fill = match options["background"].as_str() {
            Some("tint") => {
                let c = p.bg.mix(p.accent, 0.22);
                (c, c)
            }
            Some("gradient") => (
                color("background_color").unwrap_or(p.bg.mix(p.accent, 0.35)),
                color("background_color2").unwrap_or(p.bg.mix(magenta, 0.35)),
            ),
            Some("color") => {
                let c = color("background_color").unwrap_or(p.bg);
                (c, c)
            }
            _ => (p.bg, p.bg),
        };
        let style = spec.theme.style.bar;
        let floating = style != BarStyle::Solid || options["floating"].as_bool().unwrap_or(false);
        Ok(Bar {
            top: options["position"].as_str() != Some("bottom"),
            side: match options["position"].as_str() {
                Some("left") => Some(true),
                Some("right") => Some(false),
                _ => None,
            },
            floating,
            style,
            height: look.style.bar_height as f32,
            margin: if floating { look.gaps } else { 0.0 },
            left: list("modules_left"),
            center: list("modules_center"),
            right: list("modules_right"),
            fill,
        })
    }
}

fn draw_bar(canvas: &mut Pixmap, look: &Look, fonts: &Fonts, bar: &Bar, spec: &RiceSpec) {
    if let Some(on_left) = bar.side {
        draw_vertical_bar(canvas, look, fonts, bar, on_left);
        return;
    }
    let x = bar.margin;
    let y = if bar.top {
        bar.margin
    } else {
        HEIGHT as f32 - bar.margin - bar.height
    };
    let area = rect(x, y, WIDTH as f32 - 2.0 * bar.margin, bar.height);
    let alpha = look.style.bg_alpha as f32;
    let fill = |canvas: &mut Pixmap, area: Rect, radius: f32| {
        canvas.fill_path(
            &rounded_rect(area, radius.min(area.height() / 2.0)),
            &bar.paint(area, alpha),
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    };
    match bar.style {
        BarStyle::Solid | BarStyle::Floating => {
            let radius = if bar.floating {
                look.style.radius_box as f32
            } else {
                0.0
            };
            fill(canvas, area, radius);
        }
        // Острова рисуются ниже, под каждой частью панели. У минимализма фона нет.
        BarStyle::Islands | BarStyle::Minimal => {}
    }

    let px = look.font_px;
    let baseline = y + bar.height / 2.0 + px * 0.35;
    let ctx = BarText {
        fonts,
        px,
        baseline,
        y,
        height: bar.height,
        labels: look.style.raw.labels,
        widgets: &spec.widgets,
        palette: &spec.theme.palette,
    };

    // Острова: плашка под каждой непустой частью панели.
    let island = |canvas: &mut Pixmap, start: f32, width: f32| {
        if bar.style == BarStyle::Islands && width > 0.0 {
            let radius = if look.style.radius_chip > 100 {
                bar.height / 2.0
            } else {
                look.style.radius_box as f32
            };
            fill(
                canvas,
                rect(start - 6.0, y, width + 12.0, bar.height),
                radius,
            );
        }
    };
    // Разделители между модулями, как custom/sep#… в config.jsonc.tera.
    let separators = look.style.raw.separators != Separators::None;
    let sep_width = fonts.char_width(px) * 2.0;
    let width = |list: &[String]| -> f32 {
        let gaps = if separators {
            list.len().saturating_sub(1) as f32 * sep_width
        } else {
            0.0
        };
        list.iter().map(|n| ctx.module_width(n)).sum::<f32>() + gaps
    };
    let part = |canvas: &mut Pixmap, list: &[String], mut pen: f32, glyph: &str| {
        for (i, name) in list.iter().enumerate() {
            if i > 0 && separators {
                fonts.text(
                    canvas,
                    &fonts.regular,
                    glyph,
                    px,
                    pen + sep_width / 4.0,
                    baseline,
                    look.muted,
                );
                pen += sep_width;
            }
            pen = ctx.module(canvas, look, name, pen);
        }
    };

    let left_width = width(&bar.left);
    island(canvas, x + 8.0, left_width);
    part(canvas, &bar.left, x + 8.0, look.style.sep_left);
    let center_width = width(&bar.center);
    let pen = x + (area.width() - center_width) / 2.0;
    island(canvas, pen, center_width);
    part(canvas, &bar.center, pen, look.style.sep_center);
    let right_width = width(&bar.right);
    let pen = x + area.width() - 8.0 - right_width;
    island(canvas, pen, right_width);
    part(canvas, &bar.right, pen, look.style.sep_right);
}

/// Вертикальная панель: рабочие столы сверху, часы в две строки внизу.
fn draw_vertical_bar(canvas: &mut Pixmap, look: &Look, fonts: &Fonts, bar: &Bar, on_left: bool) {
    let width = bar.height + 12.0;
    let x = if on_left {
        bar.margin
    } else {
        WIDTH as f32 - bar.margin - width
    };
    let area = rect(x, bar.margin, width, HEIGHT as f32 - 2.0 * bar.margin);
    if bar.style != BarStyle::Minimal {
        let radius = if bar.floating {
            look.style.radius_box as f32
        } else {
            0.0
        };
        canvas.fill_path(
            &rounded_rect(area, radius),
            &bar.paint(area, look.style.bg_alpha as f32),
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
    let px = look.font_px;
    let cell = fonts.char_width(px);
    let center = |text: &str| x + (width - cell * text.chars().count() as f32) / 2.0;
    let mut y = bar.margin + 16.0;
    for i in 1..=5 {
        let label = i.to_string();
        let color = if i == 1 {
            let pill = rect(x + 6.0, y, width - 12.0, px * 2.0);
            let r = (look.style.radius_chip as f32).min(px);
            canvas.fill_path(
                &rounded_rect(pill, r),
                &paint(look.accent, 1.0),
                FillRule::Winding,
                Transform::identity(),
                None,
            );
            look.bg
        } else {
            look.muted
        };
        fonts.text(
            canvas,
            &fonts.regular,
            &label,
            px,
            center(&label),
            y + px * 1.35,
            color,
        );
        y += px * 2.4;
    }
    let bottom = HEIGHT as f32 - bar.margin - 20.0;
    fonts.text(
        canvas,
        &fonts.bold,
        "21",
        px,
        center("21"),
        bottom - px * 1.4,
        look.accent,
    );
    fonts.text(
        canvas,
        &fonts.bold,
        "37",
        px,
        center("37"),
        bottom,
        look.accent,
    );
}

/// Помощник для текста на панели: шрифт, размер и вертикальное положение.
struct BarText<'a> {
    fonts: &'a Fonts,
    px: f32,
    baseline: f32,
    y: f32,
    height: f32,
    /// Как подписаны модули: значки, слова или оба (style.labels).
    labels: Labels,
    /// Свои виджеты райса: модуль custom/<id>.
    widgets: &'a BTreeMap<String, Widget>,
    palette: &'a Palette,
}

impl BarText<'_> {
    const PADDING: f32 = 10.0;
    const WORKSPACES: usize = 5;

    /// Что показывает модуль Waybar и каким цветом (None — обычным).
    /// Совпадает с форматами в config.jsonc.tera: значок, слово, значение.
    fn label(&self, name: &str) -> (String, Option<Color>) {
        let (glyph, word, value) = match name {
            "clock" => ("\u{f0150}", "", "21:37"),
            "pulseaudio" => ("\u{f057e}", "vol", "64%"),
            "network" => ("\u{f05a9}", "", "home-wifi"),
            "battery" => ("\u{f0082}", "бат", "87%"),
            "cpu" => ("\u{f0ee0}", "cpu", "12%"),
            "memory" => ("\u{f035b}", "ram", "41%"),
            "media" => ("\u{f075a}", "", "lofi beats"),
            "language" => ("\u{f030c}", "", "us"),
            "backlight" => ("\u{f00df}", "свет", "70%"),
            "window" => ("", "", "nvim ~/plove"),
            "tray" => ("", "", ""),
            "launcher" => {
                let text = if self.labels == Labels::Text {
                    "apps"
                } else {
                    "\u{f303}"
                };
                return (text.to_string(), None);
            }
            "power" => {
                let text = if self.labels == Labels::Text {
                    "off"
                } else {
                    "\u{f0425}"
                };
                return (text.to_string(), None);
            }
            id => match self.widgets.get(id) {
                Some(w) => {
                    let text = w.text.replace("{value}", "42");
                    let text = if w.icon.is_empty() || self.labels == Labels::Text {
                        text
                    } else {
                        format!("{} {text}", w.icon)
                    };
                    return (text, w.resolve_color(self.palette).ok());
                }
                None => ("", "", id),
            },
        };
        let text = match self.labels {
            _ if glyph.is_empty() => value.to_string(),
            Labels::Glyphs => format!("{glyph} {value}"),
            Labels::Text if word.is_empty() => value.to_string(),
            Labels::Text => format!("{word} {value}"),
            Labels::Both if word.is_empty() => format!("{glyph} {value}"),
            Labels::Both => format!("{glyph} {word} {value}"),
        };
        (text, None)
    }

    fn workspace_width(&self) -> f32 {
        self.fonts.char_width(self.px) + 16.0
    }

    fn module_width(&self, name: &str) -> f32 {
        if name == "workspaces" {
            return self.workspace_width() * Self::WORKSPACES as f32 + 2.0 * 4.0;
        }
        self.fonts.char_width(self.px) * self.label(name).0.chars().count() as f32
            + 2.0 * Self::PADDING
    }

    /// Нарисовать модуль, начиная с `x`. Возвращает x после модуля.
    fn module(&self, canvas: &mut Pixmap, look: &Look, name: &str, x: f32) -> f32 {
        if name == "workspaces" {
            let mut pen = x + 4.0;
            let w = self.workspace_width();
            for i in 1..=Self::WORKSPACES {
                let label = i.to_string();
                let color = if i == 1 {
                    let pill = rect(pen, self.y + 4.0, w, self.height - 8.0);
                    let r = (look.style.radius_chip as f32).min((self.height - 8.0) / 2.0);
                    canvas.fill_path(
                        &rounded_rect(pill, r),
                        &paint(look.accent, 1.0),
                        FillRule::Winding,
                        Transform::identity(),
                        None,
                    );
                    look.bg
                } else {
                    look.muted
                };
                self.fonts.text(
                    canvas,
                    &self.fonts.regular,
                    &label,
                    self.px,
                    pen + 8.0,
                    self.baseline,
                    color,
                );
                pen += w;
            }
            return pen + 4.0;
        }
        let (text, custom) = self.label(name);
        let color = match name {
            "clock" => look.accent,
            "power" => look.ansi[1],
            _ => custom.unwrap_or(look.fg),
        };
        let font = if name == "clock" {
            &self.fonts.bold
        } else {
            &self.fonts.regular
        };
        self.fonts.text(
            canvas,
            font,
            &text,
            self.px,
            x + Self::PADDING,
            self.baseline,
            color,
        );
        x + self.module_width(name)
    }
}

// ───────────────────────── виджеты на рабочем столе ─────────────────────────

/// Карточки eww: часы, загрузка системы, музыка, свои виджеты.
/// Размеры и расположение — как в eww.yuck.tera.
fn draw_desktop(
    canvas: &mut Pixmap,
    look: &Look,
    fonts: &Fonts,
    options: &serde_json::Value,
    spec: &RiceSpec,
) {
    let cards: Vec<&str> = options["cards"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();
    if cards.is_empty() {
        return;
    }
    let clock_px = options["clock_size"].as_f64().unwrap_or(110.0) as f32;
    let card_height = |card: &str| match card {
        "clock" => clock_px * 1.6,
        "system" => 140.0,
        "media" => 100.0,
        _ => 60.0,
    };
    let spacing = 18.0;
    let width = (clock_px * 3.6).max(380.0);
    let height: f32 = cards.iter().map(|c| card_height(c) + spacing).sum();
    let (sw, sh) = (WIDTH as f32, HEIGHT as f32);
    let anchor = options["anchor"].as_str().unwrap_or("center");
    let x = if anchor.ends_with("left") {
        sw * if anchor == "left" { 0.04 } else { 0.03 }
    } else if anchor.ends_with("right") {
        sw - width - sw * if anchor == "right" { 0.04 } else { 0.03 }
    } else {
        (sw - width) / 2.0
    };
    let y = if anchor.starts_with("top") {
        sh * 0.06
    } else if anchor.starts_with("bottom") {
        sh - height - sh * 0.06
    } else if anchor == "center" {
        (sh - height) / 2.0 - sh * 0.10
    } else {
        (sh - height) / 2.0
    };

    let center_text =
        |canvas: &mut Pixmap, font: &Font, text: &str, px: f32, baseline: f32, color| {
            let w = fonts.char_width(px) * text.chars().count() as f32;
            fonts.text(
                canvas,
                font,
                text,
                px,
                x + (width - w) / 2.0,
                baseline,
                color,
            );
        };
    let mut top = y;
    for card in cards {
        let h = card_height(card);
        // Подложка карточки — только со «стеклом», у часов её нет вовсе.
        if card != "clock" && look.style.raw.glass != plove_core::spec::Glass::None {
            let area = rect(x, top, width, h - 10.0);
            canvas.fill_path(
                &rounded_rect(area, look.style.radius_box as f32),
                &paint(look.bg, (look.style.bg_alpha as f32 - 0.25).max(0.15)),
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
        match card {
            "clock" => {
                center_text(
                    canvas,
                    &fonts.bold,
                    "21:37",
                    clock_px,
                    top + clock_px,
                    look.accent,
                );
                let date_px = (clock_px / 5.0).round();
                center_text(
                    canvas,
                    &fonts.regular,
                    "Среда, 28 октября",
                    date_px,
                    top + clock_px * 1.15 + date_px,
                    look.fg,
                );
            }
            "system" => {
                let gap = 22.0;
                let ring = 64.0;
                let start = x + (width - 3.0 * ring - 2.0 * gap) / 2.0;
                for (i, (label, value)) in [("cpu", 0.12), ("ram", 0.41), ("disk", 0.63)]
                    .into_iter()
                    .enumerate()
                {
                    let cx = start + i as f32 * (ring + gap) + ring / 2.0;
                    let cy = top + 14.0 + ring / 2.0;
                    draw_ring(canvas, cx, cy, ring / 2.0 - 3.0, value, look);
                    let number = format!("{}", (value * 100.0) as u32);
                    let px = 15.0;
                    let w = fonts.char_width(px) * number.len() as f32;
                    fonts.text(
                        canvas,
                        &fonts.regular,
                        &number,
                        px,
                        cx - w / 2.0,
                        cy + 5.0,
                        look.fg,
                    );
                    let w = fonts.char_width(px) * label.len() as f32;
                    fonts.text(
                        canvas,
                        &fonts.regular,
                        label,
                        px,
                        cx - w / 2.0,
                        cy + ring / 2.0 + 24.0,
                        look.muted,
                    );
                }
            }
            "media" => {
                center_text(
                    canvas,
                    &fonts.regular,
                    "Nujabes — Aruarian Dance",
                    15.0,
                    top + 34.0,
                    look.muted,
                );
                center_text(
                    canvas,
                    &fonts.bold,
                    "\u{f04ae}    \u{f040e}    \u{f04ad}",
                    22.0,
                    top + 72.0,
                    look.fg,
                );
            }
            id => {
                let text = match spec.widgets.get(id) {
                    Some(w) if !w.icon.is_empty() => {
                        format!("{} {}", w.icon, w.text.replace("{value}", "42"))
                    }
                    Some(w) => w.text.replace("{value}", "42"),
                    None => id.to_string(),
                };
                let color = spec
                    .widgets
                    .get(id)
                    .and_then(|w| w.resolve_color(&spec.theme.palette).ok())
                    .unwrap_or(look.fg);
                center_text(canvas, &fonts.regular, &text, 16.0, top + 32.0, color);
            }
        }
        top += h + spacing;
    }
}

/// Кольцо загрузки: серый круг и дуга цвета акцента от «12 часов» по часовой.
fn draw_ring(canvas: &mut Pixmap, cx: f32, cy: f32, r: f32, value: f32, look: &Look) {
    let stroke = Stroke {
        width: 6.0,
        ..Stroke::default()
    };
    let arc = |part: f32| {
        let mut pb = PathBuilder::new();
        let steps = (64.0 * part).max(2.0) as usize;
        for i in 0..=steps {
            let a = -std::f32::consts::FRAC_PI_2
                + std::f32::consts::TAU * part * i as f32 / steps as f32;
            let (px, py) = (cx + r * a.cos(), cy + r * a.sin());
            if i == 0 {
                pb.move_to(px, py);
            } else {
                pb.line_to(px, py);
            }
        }
        pb.finish()
    };
    for (part, color) in [(1.0, look.surface), (value, look.accent)] {
        if let Some(path) = arc(part) {
            canvas.stroke_path(
                &path,
                &paint(color, 1.0),
                &stroke,
                Transform::identity(),
                None,
            );
        }
    }
}

// ───────────────────────── текст ─────────────────────────

struct Fonts {
    regular: Font,
    bold: Font,
    /// Значки Nerd Font: их нет в JetBrains Mono, берём отсюда.
    symbols: Font,
}

impl Fonts {
    fn load() -> Result<Fonts> {
        let load = |bytes: &[u8]| {
            Font::from_bytes(bytes, FontSettings::default())
                .map_err(|e| anyhow!("ошибка шрифта: {e}"))
        };
        Ok(Fonts {
            regular: load(include_bytes!("../fonts/JetBrainsMono-Regular.ttf"))?,
            bold: load(include_bytes!("../fonts/JetBrainsMono-Bold.ttf"))?,
            symbols: load(include_bytes!("../fonts/SymbolsNerdFontMono-Regular.ttf"))?,
        })
    }

    /// Ширина одной клетки моноширинного шрифта.
    fn char_width(&self, px: f32) -> f32 {
        self.regular.metrics('M', px).advance_width
    }

    /// Нарисовать строку. `baseline` — линия, на которой «стоят» буквы.
    #[allow(clippy::too_many_arguments)]
    fn text(
        &self,
        canvas: &mut Pixmap,
        font: &Font,
        text: &str,
        px: f32,
        x: f32,
        baseline: f32,
        color: Color,
    ) {
        let mut pen = x;
        for ch in text.chars() {
            // Буквы — из основного шрифта, значки — из шрифта значков.
            let font =
                if font.lookup_glyph_index(ch) == 0 && self.symbols.lookup_glyph_index(ch) != 0 {
                    &self.symbols
                } else {
                    font
                };
            glyph(canvas, font, ch, px, pen, baseline, color);
            pen += self.char_width(px);
        }
    }
}

/// Нарисовать одну букву: fontdue даёт маску покрытия (0..255 на пиксель),
/// и мы смешиваем цвет буквы с тем, что уже нарисовано.
fn glyph(canvas: &mut Pixmap, font: &Font, ch: char, px: f32, x: f32, baseline: f32, color: Color) {
    let (metrics, coverage) = font.rasterize(ch, px);
    let left = (x + metrics.xmin as f32).round() as i32;
    let top = (baseline - metrics.height as f32 - metrics.ymin as f32).round() as i32;
    let (cw, ch_) = (canvas.width() as i32, canvas.height() as i32);
    let data = canvas.data_mut();
    for row in 0..metrics.height as i32 {
        for col in 0..metrics.width as i32 {
            let (px_x, px_y) = (left + col, top + row);
            if px_x < 0 || px_y < 0 || px_x >= cw || px_y >= ch_ {
                continue;
            }
            let a = coverage[(row * metrics.width as i32 + col) as usize] as f32 / 255.0;
            if a == 0.0 {
                continue;
            }
            // Пиксели tiny-skia хранятся как RGBA с premultiplied alpha.
            // Буква непрозрачная, поэтому формула простая: new = цвет·a + старое·(1−a).
            let i = ((px_y * cw + px_x) * 4) as usize;
            let src = [color.r, color.g, color.b, 255];
            for c in 0..4 {
                data[i + c] = (src[c] as f32 * a + data[i + c] as f32 * (1.0 - a)).round() as u8;
            }
        }
    }
}

/// Текстовая сетка внутри окна, как в настоящем терминале:
/// строки и столбцы фиксированного размера, всё, что не влезает, обрезается.
struct Term<'a> {
    fonts: &'a Fonts,
    px: f32,
    x: f32,
    y: f32,
    char_w: f32,
    line_h: f32,
    cols: usize,
    rows: usize,
    /// Текущая строка.
    row: usize,
}

impl<'a> Term<'a> {
    fn new(fonts: &'a Fonts, px: f32, area: Rect, padding: f32) -> Term<'a> {
        let char_w = fonts.char_width(px);
        let line_h = line_height(px);
        let inner_w = area.width() - 2.0 * padding;
        let inner_h = area.height() - 2.0 * padding;
        Term {
            fonts,
            px,
            x: area.x() + padding,
            y: area.y() + padding,
            char_w,
            line_h,
            cols: (inner_w / char_w).max(0.0) as usize,
            rows: (inner_h / line_h).max(0.0) as usize,
            row: 0,
        }
    }

    /// Написать текст в текущей строке начиная со столбца `col`.
    fn put(&self, canvas: &mut Pixmap, col: usize, text: &str, color: Color, bold: bool) {
        if self.row >= self.rows {
            return;
        }
        let font = if bold {
            &self.fonts.bold
        } else {
            &self.fonts.regular
        };
        let baseline = self.y + self.row as f32 * self.line_h + self.px;
        for (i, ch) in text.chars().enumerate() {
            let c = col + i;
            if c >= self.cols {
                break;
            }
            if ch != ' ' {
                glyph(
                    canvas,
                    font,
                    ch,
                    self.px,
                    self.x + c as f32 * self.char_w,
                    baseline,
                    color,
                );
            }
        }
    }

    /// Строка из цветных кусков, затем переход на следующую.
    fn line(&mut self, canvas: &mut Pixmap, parts: &[(&str, Color)]) {
        let mut col = 0;
        for (text, color) in parts {
            self.put(canvas, col, text, *color, false);
            col += text.chars().count();
        }
        self.row += 1;
    }

    fn blank(&mut self) {
        self.row += 1;
    }

    /// Ряд цветных квадратов: так fastfetch показывает палитру терминала.
    fn blocks(&self, canvas: &mut Pixmap, col: usize, colors: &[Color]) {
        if self.row >= self.rows {
            return;
        }
        for (i, color) in colors.iter().enumerate() {
            let c = col + i * 3;
            if c + 3 > self.cols {
                break;
            }
            let cell = rect(
                self.x + c as f32 * self.char_w,
                self.y + self.row as f32 * self.line_h,
                3.0 * self.char_w,
                self.line_h,
            );
            canvas.fill_rect(cell, &paint(*color, 1.0), Transform::identity(), None);
        }
    }

    /// Курсор-прямоугольник в предыдущей строке, в столбце `col`.
    fn cursor(&self, canvas: &mut Pixmap, col: usize, color: Color) {
        let row = self.row.saturating_sub(1);
        if row >= self.rows || col >= self.cols {
            return;
        }
        let cell = rect(
            self.x + col as f32 * self.char_w,
            self.y + row as f32 * self.line_h + 2.0,
            self.char_w,
            self.line_h - 4.0,
        );
        canvas.fill_rect(cell, &paint(color, 0.85), Transform::identity(), None);
    }
}

// ───────────────────────── геометрия ─────────────────────────

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_xywh(x, y, w.max(1.0), h.max(1.0)).expect("размеры прямоугольника конечные")
}

/// Прямоугольник со скруглёнными углами радиуса `r`.
fn rounded_rect(area: Rect, r: f32) -> Path {
    let r = r.min(area.width() / 2.0).min(area.height() / 2.0);
    if r <= 0.0 {
        return PathBuilder::from_rect(area);
    }
    let (l, t, rt, b) = (area.left(), area.top(), area.right(), area.bottom());
    let mut pb = PathBuilder::new();
    pb.move_to(l + r, t);
    pb.line_to(rt - r, t);
    pb.quad_to(rt, t, rt, t + r);
    pb.line_to(rt, b - r);
    pb.quad_to(rt, b, rt - r, b);
    pb.line_to(l + r, b);
    pb.quad_to(l, b, l, b - r);
    pb.line_to(l, t + r);
    pb.quad_to(l, t, l + r, t);
    pb.close();
    pb.finish().expect("путь прямоугольника не пустой")
}

/// Мягкая тень: несколько всё более широких полупрозрачных подложек
/// цвета фона (как `shadow { color = bg }` в hyprland.conf).
fn draw_shadow(
    canvas: &mut Pixmap,
    area: Rect,
    radius: f32,
    range: f32,
    offset: f32,
    color: Color,
) {
    let steps = 8;
    for i in (1..=steps).rev() {
        let grow = range * i as f32 / steps as f32;
        let r = rect(
            area.x() - grow,
            area.y() - grow + offset,
            area.width() + 2.0 * grow,
            area.height() + 2.0 * grow,
        );
        canvas.fill_path(
            &rounded_rect(r, radius + grow),
            &paint(color, 0.8 / steps as f32),
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}

/// Размыть то, что уже нарисовано под окном, и вернуть на место
/// только внутри его скруглённой формы.
fn blur_under(canvas: &mut Pixmap, area: Rect, shape: &Path, radius: usize) {
    let (x, y) = (area.x().floor() as i32, area.y().floor() as i32);
    let Some(mut piece) = Pixmap::new(
        area.width().ceil() as u32 + 1,
        area.height().ceil() as u32 + 1,
    ) else {
        return;
    };
    piece.draw_pixmap(
        -x,
        -y,
        canvas.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
    for _ in 0..3 {
        box_blur(&mut piece, radius);
    }
    let Some(mut mask) = Mask::new(canvas.width(), canvas.height()) else {
        return;
    };
    mask.fill_path(shape, FillRule::Winding, true, Transform::identity());
    canvas.draw_pixmap(
        x,
        y,
        piece.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        Some(&mask),
    );
}

/// Размытие «скользящим средним»: каждый пиксель становится средним
/// соседей в радиусе `r`, сначала по строкам, потом по столбцам.
/// Три прохода подряд дают почти гауссово размытие.
fn box_blur(image: &mut Pixmap, r: usize) {
    let (w, h) = (image.width() as usize, image.height() as usize);
    let mut tmp = vec![0u8; w * h * 4];
    blur_pass(image.data(), &mut tmp, w, h, r, true);
    blur_pass(&tmp, image.data_mut(), w, h, r, false);
}

fn blur_pass(src: &[u8], dst: &mut [u8], w: usize, h: usize, r: usize, horizontal: bool) {
    let (lines, len) = if horizontal { (h, w) } else { (w, h) };
    let index = |line: usize, i: usize| {
        if horizontal {
            (line * w + i) * 4
        } else {
            (i * w + line) * 4
        }
    };
    let clamp = |i: isize| i.clamp(0, len as isize - 1) as usize;
    let window = (2 * r + 1) as u32;
    for line in 0..lines {
        let mut sum = [0u32; 4];
        for k in -(r as isize)..=(r as isize) {
            let p = index(line, clamp(k));
            for c in 0..4 {
                sum[c] += src[p + c] as u32;
            }
        }
        for i in 0..len {
            let p = index(line, i);
            for c in 0..4 {
                dst[p + c] = (sum[c] / window) as u8;
            }
            // Сдвигаем окно на один пиксель: добавляем новый, убираем старый.
            let add = index(line, clamp(i as isize + r as isize + 1));
            let remove = index(line, clamp(i as isize - r as isize));
            for c in 0..4 {
                sum[c] = sum[c] + src[add + c] as u32 - src[remove + c] as u32;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plove_core::presets;

    #[test]
    fn every_preset_renders() {
        let home = std::env::temp_dir();
        let env = Env::sandbox(home);
        let registry = Registry::builtin().unwrap();
        for spec in presets::all().unwrap() {
            let image = render(&spec, &registry, &env).unwrap();
            assert_eq!((image.width(), image.height()), (WIDTH, HEIGHT));
        }
    }
}

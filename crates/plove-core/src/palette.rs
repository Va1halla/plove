//! Палитра из обоев: даёшь картинку, получаешь цвета райса (как pywal).
//!
//! Как это работает:
//! 1. Уменьшаем картинку до ~96 точек по длинной стороне: для цветов
//!    больше не нужно, а считать в сотни раз быстрее.
//! 2. Делим все точки на 8 групп похожих цветов (алгоритм k-средних,
//!    k-means: как `KMeans` из scikit-learn в Python).
//! 3. Из групп выбираем фон (самая тёмная), текст (самая светлая)
//!    и акцент (самая насыщенная из заметных), а потом подправляем их,
//!    чтобы текст хорошо читался на фоне.
//! 4. 16 цветов терминала строим вокруг акцента: красный, зелёный и т. д.
//!    берут оттенок из обоев, если там есть похожий.
//!
//! Тема всегда тёмная: светлые темы plove пока не поддерживает.
//! Результат детерминированный: одна картинка всегда даёт одну палитру.
//!
//! Подбор можно направить [`Options`]: взять акцент контрастным к картинке
//! (на обоях из оттенков одного цвета иначе всё сливается), сделать фон
//! нейтральным или глубже, а цвета терминала ярче.

use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::spec::{Color, Palette};

/// Сколько групп цветов ищем в картинке.
const CLUSTERS: usize = 8;

/// Как подбирать цвета. По умолчанию — как раньше.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Options {
    pub accent: Accent,
    pub background: Background,
    pub terminal: Terminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Accent {
    /// Самый яркий из заметных цветов картинки.
    #[default]
    Auto,
    /// Другой оттенок, чем у большей части картинки: детали вроде цветов
    /// на голубых досках, а если их нет — дополнительный цвет.
    Contrast,
    /// Основной цвет картинки.
    Dominant,
    /// Свой цвет, как есть.
    Exact(Color),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Background {
    /// Тёмный с оттенком обоев.
    #[default]
    Tinted,
    /// Почти серый.
    Neutral,
    /// Темнее, почти чёрный.
    Deep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Terminal {
    /// Приглушённые, под насыщенность акцента.
    #[default]
    Soft,
    /// Яркие.
    Vivid,
}

/// Варианты для мастера и подсказок: имя в командной строке и подпись.
pub const ACCENTS: [(&str, &str); 3] = [
    ("auto", "самый яркий"),
    ("contrast", "контрастный"),
    ("dominant", "основной цвет"),
];
pub const BACKGROUNDS: [(&str, &str); 3] = [
    ("tinted", "с оттенком"),
    ("neutral", "нейтральный"),
    ("deep", "глубокий"),
];
pub const TERMINALS: [(&str, &str); 2] = [("soft", "мягкие"), ("vivid", "яркие")];

impl Accent {
    /// `auto`, `contrast`, `dominant` или цвет `#rrggbb`.
    pub fn parse(text: &str) -> Result<Accent> {
        Ok(match text {
            // keep — старое имя из первой версии `--accent`.
            "auto" | "keep" => Accent::Auto,
            "contrast" => Accent::Contrast,
            "dominant" => Accent::Dominant,
            hex if hex.starts_with('#') => Accent::Exact(Color::parse(hex)?),
            other => bail!("акцент «{other}»: можно auto, contrast, dominant или #rrggbb"),
        })
    }

    /// Номер в [`ACCENTS`] (у своего цвета номера нет).
    pub fn index(self) -> Option<usize> {
        match self {
            Accent::Auto => Some(0),
            Accent::Contrast => Some(1),
            Accent::Dominant => Some(2),
            Accent::Exact(_) => None,
        }
    }

    pub fn from_index(i: usize) -> Accent {
        [Accent::Auto, Accent::Contrast, Accent::Dominant][i % 3]
    }
}

impl Background {
    pub fn parse(text: &str) -> Result<Background> {
        Ok(match text {
            "tinted" => Background::Tinted,
            "neutral" => Background::Neutral,
            "deep" => Background::Deep,
            other => bail!("фон «{other}»: можно tinted, neutral, deep"),
        })
    }

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn from_index(i: usize) -> Background {
        [Background::Tinted, Background::Neutral, Background::Deep][i % 3]
    }
}

impl Terminal {
    pub fn parse(text: &str) -> Result<Terminal> {
        Ok(match text {
            "soft" => Terminal::Soft,
            "vivid" => Terminal::Vivid,
            other => bail!("цвета терминала «{other}»: можно soft, vivid"),
        })
    }

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn from_index(i: usize) -> Terminal {
        [Terminal::Soft, Terminal::Vivid][i % 2]
    }
}

/// Палитра из файла картинки (PNG, JPEG или WebP).
pub fn from_image(path: &Path) -> Result<Palette> {
    Ok(from_pixels(&pixels(path)?))
}

/// Точки уменьшенной картинки: их хватает, чтобы подобрать цвета, и их
/// можно держать в памяти, чтобы быстро пересчитывать палитру (мастер).
pub fn pixels(path: &Path) -> Result<Vec<Color>> {
    let image = image::open(path)
        .with_context(|| format!("не удалось открыть картинку {}", path.display()))?;
    let small = image.thumbnail(96, 96).to_rgb8();
    Ok(small
        .pixels()
        .map(|p| Color {
            r: p[0],
            g: p[1],
            b: p[2],
        })
        .collect())
}

/// Палитра из набора точек. Отдельная функция, чтобы тестировать без файлов.
pub fn from_pixels(pixels: &[Color]) -> Palette {
    from_pixels_with(pixels, Options::default())
}

/// Палитра из набора точек с подсказками, как подбирать цвета.
pub fn from_pixels_with(pixels: &[Color], options: Options) -> Palette {
    let clusters = kmeans(pixels, CLUSTERS);
    // Группы меньше 2% картинки — это мелкие детали, на роль фона
    // или текста они не годятся.
    let notable: Vec<&Cluster> = clusters.iter().filter(|c| c.share >= 0.02).collect();
    let notable = if notable.is_empty() {
        clusters.iter().collect()
    } else {
        notable
    };

    // Фон: самая тёмная группа, затемнённая до тёмной темы, с приглушённым цветом.
    let darkest = notable
        .iter()
        .min_by(|a, b| luminance(a.color).total_cmp(&luminance(b.color)))
        .map_or(Color { r: 0, g: 0, b: 0 }, |c| c.color);
    let dark = Hsl::from(darkest);
    let bg = match options.background {
        Background::Tinted => Hsl {
            s: dark.s.min(0.35),
            l: dark.l.clamp(0.06, 0.12),
            ..dark
        },
        Background::Neutral => Hsl {
            s: dark.s.min(0.04),
            l: dark.l.clamp(0.07, 0.11),
            ..dark
        },
        Background::Deep => Hsl {
            s: dark.s.min(0.3),
            l: 0.045,
            ..dark
        },
    }
    .into();

    // Текст: самая светлая группа, осветлённая почти до белого.
    let lightest = notable
        .iter()
        .max_by(|a, b| luminance(a.color).total_cmp(&luminance(b.color)))
        .map_or(
            Color {
                r: 255,
                g: 255,
                b: 255,
            },
            |c| c.color,
        );
    let light = Hsl::from(lightest);
    let fg: Color = Hsl {
        s: light.s.min(0.25),
        l: 0.88,
        ..light
    }
    .into();

    // Цветные группы: из них берутся акцент и основной цвет картинки.
    let colorful: Vec<(&Cluster, Hsl)> = clusters
        .iter()
        .map(|c| (c, Hsl::from(c.color)))
        .filter(|(_, hsl)| hsl.s >= 0.2 && hsl.l > 0.15)
        .collect();
    // Основной цвет: самая большая цветная группа.
    let dominant = colorful
        .iter()
        .max_by(|(a, _), (b, _)| a.share.total_cmp(&b.share))
        .map(|(_, hsl)| *hsl);
    let accent_source = match options.accent {
        // Насыщенная группа, и чем она больше, тем лучше. Корень из доли,
        // чтобы крупная серая группа не перебила яркую мелкую.
        Accent::Auto => colorful
            .iter()
            .max_by(|(a, ha), (b, hb)| (ha.s * a.share.sqrt()).total_cmp(&(hb.s * b.share.sqrt())))
            .map(|(_, hsl)| *hsl),
        Accent::Dominant => dominant,
        Accent::Contrast => dominant.map(|d| contrast_hue(pixels, d)),
        Accent::Exact(_) => None,
    };
    let accent = match options.accent {
        Accent::Exact(color) => color,
        _ => {
            // В чёрно-белой картинке акцента нет: берём оттенок фона и делаем его ярче.
            let accent_hsl = accent_source.unwrap_or(Hsl { s: 0.5, ..dark });
            readable(
                Hsl {
                    s: accent_hsl.s.clamp(0.45, 0.85),
                    l: accent_hsl.l.clamp(0.6, 0.75),
                    ..accent_hsl
                },
                bg,
                4.5,
            )
        }
    };

    let terminal_s = match options.terminal {
        Terminal::Soft => (Hsl::from(accent).s * 0.85).clamp(0.4, 0.75),
        Terminal::Vivid => 0.75,
    };
    let ansi = terminal_colors(&clusters, bg, fg, terminal_s);
    Palette {
        bg,
        fg,
        accent,
        ansi,
    }
}

/// 16 цветов терминала: color0–7 обычные, color8–15 яркие.
/// `s` — насыщенность цветов.
fn terminal_colors(clusters: &[Cluster], bg: Color, fg: Color, s: f64) -> Vec<Color> {
    // Стандартные оттенки в порядке ANSI: красный, зелёный, жёлтый,
    // синий, пурпурный, голубой (в градусах на цветовом круге).
    let hues = [0.0, 120.0, 50.0, 220.0, 300.0, 185.0];
    let tones: Vec<f64> = hues
        .iter()
        .map(|&target| {
            // Если в обоях есть насыщенный цвет рядом с нужным оттенком,
            // берём его оттенок: так терминал выглядит частью картинки.
            clusters
                .iter()
                .map(|c| Hsl::from(c.color))
                .filter(|hsl| hsl.s >= 0.25 && hue_distance(hsl.h, target) <= 25.0)
                .min_by(|a, b| hue_distance(a.h, target).total_cmp(&hue_distance(b.h, target)))
                .map_or(target, |hsl| hsl.h)
        })
        .collect();

    let mut ansi = Vec::with_capacity(16);
    for bright in [false, true] {
        ansi.push(if bright {
            bg.mix(fg, 0.35)
        } else {
            bg.mix(fg, 0.12)
        });
        for &h in &tones {
            let l = if bright { 0.72 } else { 0.62 };
            ansi.push(readable(Hsl { h, s, l }, bg, 4.5));
        }
        ansi.push(if bright { fg } else { fg.mix(bg, 0.15) });
    }
    ansi
}

/// Цвет, далёкий по оттенку от основного `dominant`. Ищем по всем точкам,
/// а не по группам: мелкие детали (бутоны на голубых досках) в отдельную
/// группу k-средних не попадают. Нет такого — дополнительный цвет.
fn contrast_hue(pixels: &[Color], dominant: Hsl) -> Hsl {
    // 24 корзины по 15°: вес (сумма насыщенности), суммы s и l и
    // направление оттенка (среднее по кругу, чтобы 350° и 10° дали 0°).
    let mut bins = [(0.0f64, 0usize, 0.0f64, 0.0f64, 0.0f64, 0.0f64); 24];
    for &c in pixels {
        let hsl = Hsl::from(c);
        if hsl.s < 0.3 || !(0.2..=0.85).contains(&hsl.l) || hue_distance(hsl.h, dominant.h) < 60.0 {
            continue;
        }
        let bin = &mut bins[(hsl.h / 15.0) as usize % 24];
        let rad = hsl.h.to_radians();
        bin.0 += hsl.s;
        bin.1 += 1;
        bin.2 += hsl.s;
        bin.3 += hsl.l;
        bin.4 += rad.cos();
        bin.5 += rad.sin();
    }
    // Хотя бы 0,3% картинки, иначе это шум.
    let enough = (pixels.len() as f64 * 0.003).max(3.0) as usize;
    bins.iter()
        .filter(|b| b.1 >= enough)
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|&(_, n, s, l, cos, sin)| Hsl {
            h: sin.atan2(cos).to_degrees().rem_euclid(360.0),
            s: s / n as f64,
            l: l / n as f64,
        })
        .unwrap_or(Hsl {
            h: (dominant.h + 180.0).rem_euclid(360.0),
            ..dominant
        })
}

/// Осветлять цвет, пока контраст с фоном не станет не меньше `min`.
fn readable(mut hsl: Hsl, bg: Color, min: f64) -> Color {
    while contrast(hsl.into(), bg) < min && hsl.l < 0.95 {
        hsl.l += 0.02;
    }
    hsl.into()
}

/// Группа похожих цветов: средний цвет и доля картинки (от 0 до 1).
struct Cluster {
    color: Color,
    share: f64,
}

/// k-средних: разбить точки на `k` групп так, чтобы каждая точка
/// была ближе к центру своей группы, чем к другим.
fn kmeans(pixels: &[Color], k: usize) -> Vec<Cluster> {
    if pixels.is_empty() {
        return Vec::new();
    }
    let points: Vec<[f64; 3]> = pixels
        .iter()
        .map(|c| [c.r as f64, c.g as f64, c.b as f64])
        .collect();

    // Начальные центры: точки, равномерно взятые из списка, отсортированного
    // по яркости. Без случайности, поэтому результат всегда одинаковый.
    let mut by_brightness = points.clone();
    by_brightness.sort_by(|a, b| (a[0] + a[1] + a[2]).total_cmp(&(b[0] + b[1] + b[2])));
    let mut centers: Vec<[f64; 3]> = (0..k)
        .map(|i| by_brightness[(2 * i + 1) * by_brightness.len() / (2 * k)])
        .collect();

    let mut owner = vec![0usize; points.len()];
    for _ in 0..12 {
        // Каждую точку отдаём ближайшему центру…
        for (point, own) in points.iter().zip(owner.iter_mut()) {
            *own = (0..k)
                .min_by(|&a, &b| {
                    distance(point, &centers[a]).total_cmp(&distance(point, &centers[b]))
                })
                .unwrap_or(0);
        }
        // …и переносим центр в среднее своих точек.
        let mut sums = vec![[0.0; 3]; k];
        let mut counts = vec![0usize; k];
        for (point, &own) in points.iter().zip(&owner) {
            for channel in 0..3 {
                sums[own][channel] += point[channel];
            }
            counts[own] += 1;
        }
        for i in 0..k {
            if counts[i] > 0 {
                centers[i] = sums[i].map(|sum| sum / counts[i] as f64);
            }
        }
    }

    let mut counts = vec![0usize; k];
    for &own in &owner {
        counts[own] += 1;
    }
    let mut clusters: Vec<Cluster> = centers
        .iter()
        .zip(&counts)
        .filter(|(_, &count)| count > 0)
        .map(|(center, &count)| Cluster {
            color: Color {
                r: center[0].round() as u8,
                g: center[1].round() as u8,
                b: center[2].round() as u8,
            },
            share: count as f64 / points.len() as f64,
        })
        .collect();
    clusters.sort_by(|a, b| b.share.total_cmp(&a.share));
    clusters
}

fn distance(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum()
}

/// Насколько далеко два оттенка на цветовом круге, от 0 до 180 градусов.
fn hue_distance(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

/// Яркость цвета для глаза, от 0 (чёрный) до 1 (белый). Формула из WCAG.
fn luminance(c: Color) -> f64 {
    let channel = |v: u8| {
        let v = v as f64 / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b)
}

/// Контраст двух цветов по WCAG: от 1 (одинаковые) до 21 (чёрный на белом).
/// Для обычного текста нужно хотя бы 4.5.
pub fn contrast(a: Color, b: Color) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// Цвет в виде «оттенок, насыщенность, светлота»: с ним удобнее
/// говорить «тот же цвет, только светлее».
/// h — от 0 до 360 градусов, s и l — от 0 до 1.
#[derive(Debug, Clone, Copy)]
struct Hsl {
    h: f64,
    s: f64,
    l: f64,
}

impl From<Color> for Hsl {
    fn from(c: Color) -> Hsl {
        let (r, g, b) = (c.r as f64 / 255.0, c.g as f64 / 255.0, c.b as f64 / 255.0);
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let l = (max + min) / 2.0;
        let d = max - min;
        if d == 0.0 {
            return Hsl { h: 0.0, s: 0.0, l };
        }
        let s = d / (1.0 - (2.0 * l - 1.0).abs());
        let h = if max == r {
            60.0 * ((g - b) / d).rem_euclid(6.0)
        } else if max == g {
            60.0 * ((b - r) / d + 2.0)
        } else {
            60.0 * ((r - g) / d + 4.0)
        };
        Hsl { h, s, l }
    }
}

impl From<Hsl> for Color {
    fn from(hsl: Hsl) -> Color {
        let (h, s, l) = (
            hsl.h.rem_euclid(360.0),
            hsl.s.clamp(0.0, 1.0),
            hsl.l.clamp(0.0, 1.0),
        );
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
        let m = l - c / 2.0;
        let (r, g, b) = match (h / 60.0) as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let byte = |v: f64| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
        Color {
            r: byte(r),
            g: byte(g),
            b: byte(b),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repeat(hex: &str, n: usize) -> Vec<Color> {
        vec![Color::parse(hex).unwrap(); n]
    }

    /// Тёмно-синие обои с оранжевым закатом и светлыми облаками.
    fn sunset() -> Vec<Color> {
        [
            repeat("#1a1b26", 700),
            repeat("#ff8800", 100),
            repeat("#e0e0e0", 200),
        ]
        .concat()
    }

    #[test]
    fn picks_dark_bg_and_orange_accent() {
        let palette = from_pixels(&sunset());
        assert!(luminance(palette.bg) < 0.03, "{}", palette.bg.hex());
        let accent = Hsl::from(palette.accent);
        assert!(
            hue_distance(accent.h, 32.0) < 10.0,
            "{}",
            palette.accent.hex()
        );
        assert!(contrast(palette.fg, palette.bg) >= 7.0);
        assert!(contrast(palette.accent, palette.bg) >= 4.5);
        assert_eq!(palette.ansi.len(), 16);
    }

    #[test]
    fn grayscale_still_gets_an_accent() {
        let pixels = [repeat("#202020", 500), repeat("#d0d0d0", 500)].concat();
        let palette = from_pixels(&pixels);
        assert!(contrast(palette.accent, palette.bg) >= 4.5);
    }

    #[test]
    fn same_image_same_palette() {
        let a = from_pixels(&sunset());
        let b = from_pixels(&sunset());
        assert_eq!(a.accent, b.accent);
        assert_eq!(a.ansi, b.ansi);
    }

    /// Голубые доски почти на всю картинку, белые цветы и немного красных бутонов.
    fn boards() -> Vec<Color> {
        [
            repeat("#3fa9c9", 600),
            repeat("#2f8fae", 250),
            repeat("#123c48", 60),
            repeat("#f2f0ee", 80),
            repeat("#9c3b2c", 10),
        ]
        .concat()
    }

    #[test]
    fn contrast_accent_finds_the_buds() {
        let auto = Hsl::from(from_pixels(&boards()).accent);
        assert!(hue_distance(auto.h, 194.0) < 15.0, "auto: {}", auto.h);
        let options = Options {
            accent: Accent::Contrast,
            ..Options::default()
        };
        let palette = from_pixels_with(&boards(), options);
        let accent = Hsl::from(palette.accent);
        assert!(
            hue_distance(accent.h, 8.0) < 15.0,
            "{}",
            palette.accent.hex()
        );
        assert!(contrast(palette.accent, palette.bg) >= 4.5);
    }

    #[test]
    fn contrast_accent_without_details_is_complementary() {
        let pixels = [repeat("#3fa9c9", 900), repeat("#123c48", 100)].concat();
        let options = Options {
            accent: Accent::Contrast,
            ..Options::default()
        };
        let accent = Hsl::from(from_pixels_with(&pixels, options).accent);
        assert!(hue_distance(accent.h, 14.0) < 15.0, "{}", accent.h);
    }

    #[test]
    fn background_and_exact_accent() {
        let neutral = from_pixels_with(
            &boards(),
            Options {
                background: Background::Neutral,
                accent: Accent::parse("#ff00aa").unwrap(),
                ..Options::default()
            },
        );
        assert!(Hsl::from(neutral.bg).s <= 0.1, "{}", neutral.bg.hex());
        assert_eq!(neutral.accent.hex(), "#ff00aa");
        let deep = from_pixels_with(
            &boards(),
            Options {
                background: Background::Deep,
                ..Options::default()
            },
        );
        assert!(luminance(deep.bg) < luminance(from_pixels(&boards()).bg));
        assert!(Accent::parse("blue").is_err());
    }

    #[test]
    fn hsl_roundtrip() {
        for hex in ["#7aa2f7", "#fe8019", "#2e3440", "#000000", "#ffffff"] {
            let color = Color::parse(hex).unwrap();
            assert_eq!(Color::from(Hsl::from(color)), color, "{hex}");
        }
    }
}

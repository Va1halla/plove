//! Обои, нарисованные из палитры райса.
//!
//! Чужие картинки в репозиторий класть нельзя из-за лицензий, поэтому
//! встроенные райсы рисуют обои сами: холмы, волны или кольца в цветах темы.
//! Рисование детерминированное: один и тот же райс всегда даёт байт в байт
//! одинаковый PNG, поэтому повторный `apply` не видит изменений.
//!
//! Рисуем библиотекой tiny-skia: это 2D-движок, похожий на System.Drawing
//! из C# или Pillow из Python: пути, заливки, градиенты.

use anyhow::{anyhow, Result};
use tiny_skia::{
    FillRule, GradientStop, LinearGradient, Paint, Path, PathBuilder, Pixmap, Point, Rect,
    SpreadMode, Stroke, Transform,
};

use crate::spec::{Color, Palette, Pattern};

/// Размер обоев для настоящей системы. swaybg сам подгонит под экран.
pub const WIDTH: u32 = 2560;
pub const HEIGHT: u32 = 1440;

const BLACK: Color = Color { r: 0, g: 0, b: 0 };

/// Нарисовать обои заданного размера.
pub fn generate(pattern: Pattern, palette: &Palette, width: u32, height: u32) -> Result<Pixmap> {
    let mut canvas = Pixmap::new(width, height).ok_or_else(|| anyhow!("неверный размер обоев"))?;
    match pattern {
        Pattern::Hills => hills(&mut canvas, palette),
        Pattern::Waves => waves(&mut canvas, palette),
        Pattern::Rings => rings(&mut canvas, palette),
    }
    Ok(canvas)
}

/// То же, но сразу в виде PNG-файла.
pub fn generate_png(pattern: Pattern, palette: &Palette) -> Result<Vec<u8>> {
    let canvas = generate(pattern, palette, WIDTH, HEIGHT)?;
    canvas
        .encode_png()
        .map_err(|e| anyhow!("не удалось сохранить обои в PNG: {e}"))
}

/// Открыть свои обои (PNG, JPEG или WebP) как холст tiny-skia.
pub fn load(path: &std::path::Path) -> Result<Pixmap> {
    let image = image::open(path)
        .map_err(|e| anyhow!("не удалось открыть {}: {e}", path.display()))?
        .to_rgba8();
    let (width, height) = image.dimensions();
    // tiny-skia хранит цвета, уже умноженные на прозрачность
    // («premultiplied alpha»), поэтому пересчитываем каждую точку.
    let mut data = image.into_raw();
    for pixel in data.chunks_exact_mut(4) {
        let alpha = pixel[3] as u16;
        for channel in &mut pixel[..3] {
            *channel = ((*channel as u16 * alpha + 127) / 255) as u8;
        }
    }
    let size =
        tiny_skia::IntSize::from_wh(width, height).ok_or_else(|| anyhow!("пустая картинка"))?;
    Pixmap::from_vec(data, size).ok_or_else(|| anyhow!("не удалось прочитать {}", path.display()))
}

/// Ночное небо, луна, звёзды и четыре слоя холмов: дальние светлее, ближние темнее.
fn hills(canvas: &mut Pixmap, p: &Palette) {
    let (w, h) = (canvas.width() as f32, canvas.height() as f32);
    vertical_gradient(canvas, p.bg, p.bg.mix(p.accent, 0.35));

    let mut random = Lcg(7);
    for _ in 0..90 {
        let (x, y) = (random.next() * w, random.next() * h * 0.55);
        let r = 0.6 + random.next() * h * 0.0015;
        fill(canvas, &circle(x, y, r), p.fg, 0.25 + random.next() * 0.45);
    }

    let (mx, my, mr) = (w * 0.76, h * 0.26, h * 0.055);
    // Мягкое свечение вокруг луны: несколько полупрозрачных кругов друг на друге.
    for k in 1..=16 {
        fill(
            canvas,
            &circle(mx, my, mr * (1.0 + 0.11 * k as f32)),
            p.accent,
            0.012,
        );
    }
    fill(canvas, &circle(mx, my, mr), p.fg.mix(p.accent, 0.25), 0.95);

    for i in 0..4 {
        let t = i as f32 / 3.0;
        let color =
            p.bg.mix(p.accent, (0.45 * (1.0 - t) + 0.06) as f64)
                .mix(BLACK, (0.18 * t) as f64);
        let base = h * (0.58 + 0.1 * t);
        let shape = wave_path(
            w,
            h,
            base,
            h * (0.07 - 0.015 * t),
            1.6 + i as f32 * 0.7,
            i as f32 * 1.9,
        );
        fill(canvas, &shape, color, 1.0);
    }
}

/// Пять полупрозрачных волн снизу, в цветах палитры.
fn waves(canvas: &mut Pixmap, p: &Palette) {
    let (w, h) = (canvas.width() as f32, canvas.height() as f32);
    vertical_gradient(canvas, p.bg, p.bg.mix(p.accent, 0.12));

    // Если заданы цвета терминала, берём из них розовый, голубой, синий и зелёный.
    let colors: Vec<Color> = if p.ansi.len() == 16 {
        vec![p.ansi[5], p.ansi[6], p.ansi[4], p.accent, p.ansi[2]]
    } else {
        vec![
            p.accent,
            p.accent.mix(p.fg, 0.3),
            p.accent,
            p.accent.mix(p.bg, 0.3),
            p.accent,
        ]
    };
    for (i, color) in colors.iter().enumerate() {
        let t = i as f32 / 4.0;
        let base = h * (0.52 + 0.1 * t);
        let shape = wave_path(w, h, base, h * 0.05, 1.2 + t * 1.3, i as f32 * 2.3);
        // Дальние волны светлее, ближние темнее и ближе к фону.
        fill(
            canvas,
            &shape,
            color.mix(p.bg, (0.45 + 0.3 * t) as f64),
            0.9,
        );
    }
}

/// Кольца, расходящиеся из правого нижнего угла.
fn rings(canvas: &mut Pixmap, p: &Palette) {
    let (w, h) = (canvas.width() as f32, canvas.height() as f32);
    vertical_gradient(canvas, p.bg, p.bg.mix(BLACK, 0.15));

    let (cx, cy) = (w * 0.78, h * 0.72);
    for k in 1..16 {
        let alpha = 0.55 - k as f32 * 0.03;
        let ring = circle(cx, cy, h * 0.075 * k as f32);
        let color = p.accent.mix(p.bg, 0.2 + k as f64 * 0.04);
        stroke(canvas, &ring, color, alpha, h * 0.01);
    }
    fill(canvas, &circle(cx, cy, h * 0.045), p.accent, 1.0);
    fill(canvas, &circle(w * 0.12, h * 0.18, h * 0.02), p.fg, 0.35);
}

/// Залить весь холст вертикальным градиентом сверху вниз.
fn vertical_gradient(canvas: &mut Pixmap, top: Color, bottom: Color) {
    let h = canvas.height() as f32;
    let shader = LinearGradient::new(
        Point::from_xy(0.0, 0.0),
        Point::from_xy(0.0, h),
        vec![
            GradientStop::new(0.0, sk(top, 1.0)),
            GradientStop::new(1.0, sk(bottom, 1.0)),
        ],
        SpreadMode::Pad,
        Transform::identity(),
    );
    let rect = Rect::from_xywh(0.0, 0.0, canvas.width() as f32, h);
    if let (Some(shader), Some(rect)) = (shader, rect) {
        let paint = Paint {
            shader,
            ..Paint::default()
        };
        canvas.fill_rect(rect, &paint, Transform::identity(), None);
    }
}

/// Фигура «волнистая линия до низа экрана»: сумма двух синусоид.
fn wave_path(w: f32, h: f32, base: f32, amplitude: f32, frequency: f32, phase: f32) -> Path {
    let mut pb = PathBuilder::new();
    pb.move_to(0.0, h);
    let steps = 200;
    for s in 0..=steps {
        let x = w * s as f32 / steps as f32;
        let u = x / w * std::f32::consts::TAU;
        let y = base
            + amplitude
                * (0.65 * (u * frequency + phase).sin()
                    + 0.35 * (u * frequency * 2.3 + phase * 1.7).sin());
        pb.line_to(x, y);
    }
    pb.line_to(w, h);
    pb.close();
    pb.finish().expect("путь волны не пустой")
}

pub(crate) fn circle(x: f32, y: f32, r: f32) -> Path {
    PathBuilder::from_circle(x, y, r).expect("радиус круга больше нуля")
}

fn fill(canvas: &mut Pixmap, path: &Path, color: Color, alpha: f32) {
    canvas.fill_path(
        path,
        &paint(color, alpha),
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

fn stroke(canvas: &mut Pixmap, path: &Path, color: Color, alpha: f32, width: f32) {
    let line = Stroke {
        width,
        ..Stroke::default()
    };
    canvas.stroke_path(
        path,
        &paint(color, alpha),
        &line,
        Transform::identity(),
        None,
    );
}

/// Кисть сплошного цвета со сглаживанием краёв.
pub fn paint(color: Color, alpha: f32) -> Paint<'static> {
    let mut paint = Paint {
        anti_alias: true,
        ..Paint::default()
    };
    paint.set_color(sk(color, alpha));
    paint
}

/// Наш Color → цвет tiny-skia с прозрачностью.
pub fn sk(color: Color, alpha: f32) -> tiny_skia::Color {
    let a = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
    tiny_skia::Color::from_rgba8(color.r, color.g, color.b, a)
}

/// Простейший генератор псевдослучайных чисел с фиксированным зерном:
/// звёзды каждый раз на тех же местах, и PNG не меняется между запусками.
struct Lcg(u64);

impl Lcg {
    /// Число от 0.0 до 1.0.
    fn next(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_palette_gives_same_png() {
        let palette = Palette {
            bg: Color::parse("#1a1b26").unwrap(),
            fg: Color::parse("#c0caf5").unwrap(),
            accent: Color::parse("#7aa2f7").unwrap(),
            ansi: Vec::new(),
        };
        for pattern in [Pattern::Hills, Pattern::Waves, Pattern::Rings] {
            let a = generate(pattern, &palette, 320, 180)
                .unwrap()
                .encode_png()
                .unwrap();
            let b = generate(pattern, &palette, 320, 180)
                .unwrap()
                .encode_png()
                .unwrap();
            assert_eq!(a, b);
        }
    }
}

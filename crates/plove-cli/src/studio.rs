//! `plove studio`: редактор карточек рабочего стола в браузере.
//!
//! Маленький HTTP-сервер на 127.0.0.1 отдаёт одну страницу (studio.html),
//! картинку рабочего стола без карточек и состояние райса. Карточки
//! двигаются мышью прямо на странице; «Сохранить» записывает раскладку
//! в файл райса (раздел `desktop`), «Применить» ещё и запускает
//! `plove apply`. Сервер без библиотек: запросов мало, и все они от одного
//! человека. Запрос читается в своём потоке: браузер заранее открывает
//! соединения «про запас» и ничего в них не шлёт, и одно такое соединение
//! не должно задерживать остальные. Отвечает на запросы один поток по очереди.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use plove_core::desk::{self, DeskCard};
use plove_core::modules::Registry;
use plove_core::spec::{RiceSpec, COLOR_NAMES};
use plove_core::system::Env;
use plove_core::theme;
use serde_json::json;

const PAGE: &str = include_str!("studio.html");

/// Что редактируем и куда сохраняем.
struct Studio<'a> {
    spec: RiceSpec,
    file: PathBuf,
    env: &'a Env,
    registry: &'a Registry,
    /// Картинка рабочего стола без карточек, PNG.
    desktop_png: Vec<u8>,
    /// Размер экрана в логических точках: в них считаются размеры окон eww.
    screen: (u32, u32),
}

/// Запустить редактор. `rice` — файл райса или имя встроенного райса:
/// встроенный сохраняется копией в ~/.config/plove/rices/.
pub fn run(rice: &str, registry: &Registry, env: &Env, port: u16, open: bool) -> Result<()> {
    let (mut spec, file) = load(rice, env)?;
    let screen = screen_size(env);
    if spec.desktop.is_empty() {
        spec.desktop = from_eww_settings(&spec, registry, screen)?;
    }
    let desktop_png = desktop_png(&spec, registry, env)?;
    let mut studio = Studio {
        spec,
        file,
        env,
        registry,
        desktop_png,
        screen,
    };

    let listener = TcpListener::bind(("127.0.0.1", port))
        .with_context(|| format!("порт {port} занят, укажите другой: --port"))?;
    let url = format!("http://127.0.0.1:{}/", listener.local_addr()?.port());
    println!("plove studio: {url}");
    println!("Райс: {}", env.display(&studio.file));
    println!("Остановить: Ctrl+C");
    if open {
        // xdg-open сам выберет браузер; если его нет, ссылка выше.
        let _ = Command::new("xdg-open")
            .arg(&url)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }
    let (requests, incoming) = mpsc::channel();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let requests = requests.clone();
            std::thread::spawn(move || {
                // Соединение «про запас» так и закроется, ничего не прислав.
                let _ = stream.set_read_timeout(Some(Duration::from_secs(60)));
                if let Ok(request) = read_request(&mut stream) {
                    let _ = requests.send((stream, request));
                }
            });
        }
    });
    for (mut stream, request) in incoming {
        if let Err(e) = studio.serve(&mut stream, request) {
            let _ = respond_json(&mut stream, 500, &json!({ "error": format!("{e:#}") }));
        }
    }
    Ok(())
}

/// Картинка стола без карточек. Карточки рисует сама страница, а окна
/// терминала только мешали бы видеть обои: остаются обои и панель.
fn desktop_png(spec: &RiceSpec, registry: &Registry, env: &Env) -> Result<Vec<u8>> {
    let mut background = spec.clone();
    background.desktop.clear();
    background.components.remove("widgets");
    background.components.remove("terminal");
    plove_preview::render_png(&background, registry, env)
}

/// Чем рисовать карточки: при панели Quickshell — ею самой (qs-desktop),
/// иначе eww.
fn desk_engine(spec: &RiceSpec) -> &'static str {
    if spec.components.get("bar").map(String::as_str) == Some("quickshell") {
        "qs-desktop"
    } else {
        "eww"
    }
}

fn load(rice: &str, env: &Env) -> Result<(RiceSpec, PathBuf)> {
    let path = Path::new(rice);
    if path.exists() {
        return Ok((RiceSpec::load(path)?, path.to_path_buf()));
    }
    match plove_core::presets::get(rice)? {
        Some(spec) => {
            let file = env.expand(&format!("~/.config/plove/rices/{}.yaml", spec.name));
            // Свою копию встроенного райса уже правили: открываем её.
            if file.exists() {
                return Ok((RiceSpec::load(&file)?, file));
            }
            Ok((spec, file))
        }
        None => bail!("нет ни файла «{rice}», ни встроенного райса с таким именем"),
    }
}

/// Раскладка, которая была до studio: карточки из настроек eww (cards,
/// anchor, clock_size) столбиком у выбранного края. Так в редакторе сразу
/// видно то же, что сейчас на столе.
fn from_eww_settings(
    spec: &RiceSpec,
    registry: &Registry,
    screen: (u32, u32),
) -> Result<Vec<DeskCard>> {
    let (Some(id), Some(module)) = (spec.components.get("widgets"), registry.get("eww")) else {
        return Ok(Vec::new());
    };
    if id != "eww" {
        return Ok(Vec::new());
    }
    let options = plove_core::plan::module_options(module, spec.overrides.get("eww"))?;
    let names: Vec<String> = options["cards"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let clock_scale = options["clock_size"].as_f64().unwrap_or(110.0) / 96.0;
    let mut cards: Vec<DeskCard> = names
        .iter()
        .map(|name| {
            let (kind, scale, with) = match name.as_str() {
                "clock" => ("clock", (clock_scale * 20.0).round() / 20.0, None),
                "system" => ("system", 1.0, None),
                "media" => ("player", 1.0, None),
                widget => ("widget", 1.0, Some(widget)),
            };
            let mut map = serde_yaml::Mapping::new();
            if let Some(widget) = with {
                map.insert("id".into(), widget.into());
            }
            DeskCard {
                kind: kind.into(),
                at: [0.0, 0.0],
                scale,
                with: map,
                look: Default::default(),
            }
        })
        .filter(|c| {
            c.kind != "widget"
                || spec
                    .widgets
                    .contains_key(c.with["id"].as_str().unwrap_or(""))
        })
        .collect();
    // Размеры в процентах экрана, как их считает eww.
    let (sw, sh) = (screen.0 as f64, screen.1 as f64);
    let sizes: Vec<(f64, f64)> = cards
        .iter()
        .map(|c| {
            let kind = desk::kind(&c.kind).map_or((300, 100), |k| k.size);
            (
                kind.0 as f64 * c.scale / sw * 100.0,
                kind.1 as f64 * c.scale / sh * 100.0,
            )
        })
        .collect();
    let gap = 18.0 / sh * 100.0;
    let total: f64 =
        sizes.iter().map(|s| s.1).sum::<f64>() + gap * sizes.len().saturating_sub(1) as f64;
    let anchor = options["anchor"].as_str().unwrap_or("center");
    let mut y = if anchor.starts_with("top") {
        6.0
    } else if anchor.starts_with("bottom") {
        94.0 - total
    } else {
        (100.0 - total) / 2.0 - if anchor == "center" { 10.0 } else { 0.0 }
    };
    for (card, (w, h)) in cards.iter_mut().zip(&sizes) {
        let x = if anchor.ends_with("left") {
            4.0
        } else if anchor.ends_with("right") {
            96.0 - w
        } else {
            50.0 - w / 2.0
        };
        let round = |v: f64| (v.clamp(0.0, 100.0) * 10.0).round() / 10.0;
        card.at = [round(x), round(y)];
        y += h + gap;
    }
    Ok(cards)
}

/// Размер первого монитора из `hyprctl monitors -j`, с учётом масштаба.
/// В песочнице и без Hyprland — 1920×1080.
fn screen_size(env: &Env) -> (u32, u32) {
    let fallback = (1920, 1080);
    if env.sandbox {
        return fallback;
    }
    let Ok(out) = Command::new("hyprctl").args(["monitors", "-j"]).output() else {
        return fallback;
    };
    let Ok(monitors) = serde_json::from_slice::<serde_json::Value>(&out.stdout) else {
        return fallback;
    };
    let Some(m) = monitors.get(0) else {
        return fallback;
    };
    let scale = m["scale"].as_f64().unwrap_or(1.0).max(0.1);
    // Повёрнутый монитор (transform 1, 3, 5, 7): ширина и высота меняются местами.
    let rotated = m["transform"].as_u64().unwrap_or(0) % 2 == 1;
    let (w, h) = (m["width"].as_f64(), m["height"].as_f64());
    match (w, h) {
        (Some(w), Some(h)) => {
            let (w, h) = ((w / scale).round() as u32, (h / scale).round() as u32);
            if rotated {
                (h, w)
            } else {
                (w, h)
            }
        }
        _ => fallback,
    }
}

impl Studio<'_> {
    fn serve(&mut self, stream: &mut TcpStream, request: Request) -> Result<()> {
        let (method, path, body) = (request.method, request.path, request.body);
        // Любой сайт в браузере может отправить запрос на 127.0.0.1.
        // Своего заголовка без разрешения CORS чужая страница не добавит,
        // а проверка Host отсекает подмену адреса через DNS.
        if method == "POST" && !request.ours {
            return respond(stream, 403, "text/plain; charset=utf-8", b"forbidden");
        }
        match (method.as_str(), path.as_str()) {
            ("GET", "/") => respond(stream, 200, "text/html; charset=utf-8", PAGE.as_bytes()),
            ("GET", "/desktop.png") => respond(stream, 200, "image/png", &self.desktop_png),
            ("GET", "/api/state") => respond_json(stream, 200, &self.state()),
            ("POST", "/api/save") => {
                let result = self.save(&body);
                self.reply(stream, result.map(|_| "Сохранено".to_string()))
            }
            ("POST", "/api/apply") => {
                let result = self.save(&body).and_then(|_| self.apply());
                self.reply(stream, result)
            }
            _ => respond(stream, 404, "text/plain; charset=utf-8", b"not found"),
        }
    }

    fn reply(&self, stream: &mut TcpStream, result: Result<String>) -> Result<()> {
        match result {
            Ok(message) => respond_json(stream, 200, &json!({ "ok": true, "message": message })),
            Err(e) => respond_json(
                stream,
                400,
                &json!({ "ok": false, "error": format!("{e:#}") }),
            ),
        }
    }

    /// Шрифты на выбор: те, что plove умеет поставить, и все, что уже стоят.
    fn fonts(&self) -> serde_json::Value {
        let installed = plove_core::system::font_families();
        let known: Vec<serde_json::Value> = theme::FONTS
            .iter()
            .map(|f| {
                json!({
                    "name": f.name,
                    "mono": f.mono,
                    "installed": installed.iter().any(|i| i == f.name),
                })
            })
            .collect();
        json!({ "known": known, "installed": installed })
    }

    /// Всё, что нужно странице: цвета и стиль райса, каталог, карточки.
    fn state(&self) -> serde_json::Value {
        let palette = &self.spec.theme.palette;
        let colors: serde_json::Map<String, serde_json::Value> = COLOR_NAMES
            .iter()
            .filter_map(|name| Some((name.to_string(), json!(palette.named(name)?.hex()))))
            .collect();
        let widgets: serde_json::Map<String, serde_json::Value> = self
            .spec
            .widgets
            .iter()
            .map(|(id, w)| {
                let color = w
                    .resolve_color(&self.spec.theme.palette)
                    .map(|c| c.hex())
                    .unwrap_or_default();
                (
                    id.clone(),
                    json!({ "icon": w.icon, "text": w.text, "color": color }),
                )
            })
            .collect();
        json!({
            "name": self.spec.name,
            "file": self.env.display(&self.file),
            "screen": [self.screen.0, self.screen.1],
            "theme": theme::theme_context(&self.spec),
            "catalog": desk::catalog_json(),
            "desktop": self.spec.desktop,
            "widgets": widgets,
            "eww": self.spec.components.contains_key("widgets"),
            "engine": desk_engine(&self.spec),
            "card_style": self
                .spec
                .overrides
                .get("qs-desktop")
                .and_then(|o| o.get("style"))
                .and_then(|v| v.as_str())
                .unwrap_or("glass"),
            "colors": colors,
            "fonts": self.fonts(),
        })
    }

    /// Записать раскладку со страницы в файл райса (старый — рядом, .bak).
    fn save(&mut self, body: &[u8]) -> Result<()> {
        let request: serde_json::Value =
            serde_json::from_slice(body).context("страница прислала не JSON")?;
        let cards: Vec<DeskCard> = serde_json::from_value(request["desktop"].clone())
            .context("страница прислала карточки в неверном виде")?;
        let mut spec = self.spec.clone();
        spec.desktop = cards;
        // Вид карточек — настройка style модуля qs-desktop.
        if let Some(style) = request["card_style"].as_str() {
            if desk_engine(&spec) == "qs-desktop" {
                spec.overrides
                    .entry("qs-desktop".into())
                    .or_default()
                    .insert("style".into(), style.into());
            }
        }
        if !request["fonts"].is_null() {
            spec.theme.fonts = serde_json::from_value(request["fonts"].clone())
                .context("страница прислала шрифты в неверном виде")?;
        }
        // Карточки рисует панель Quickshell (qs-desktop) или eww: без них
        // раскладка ничего бы не показала.
        if !spec.desktop.is_empty() {
            let engine = desk_engine(&spec);
            spec.components
                .entry("widgets".into())
                .or_insert_with(|| engine.into());
        }
        spec.validate()?;
        if let Some(dir) = self.file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        if self.file.exists() {
            let mut backup = self.file.as_os_str().to_owned();
            backup.push(".bak");
            std::fs::copy(&self.file, &backup)?;
        }
        let text = format!(
            "# Райс plove. Карточки рабочего стола расставлены в `plove studio`.\n# Применить: plove apply {}\n{}",
            self.env.display(&self.file),
            spec.to_yaml()?
        );
        std::fs::write(&self.file, text)
            .with_context(|| format!("не удалось записать {}", self.file.display()))?;
        // Шрифт видно на панели в картинке стола: рисуем её заново.
        let fonts_changed = spec.theme.fonts != self.spec.theme.fonts;
        self.spec = spec;
        if fonts_changed {
            self.desktop_png = desktop_png(&self.spec, self.registry, self.env)?;
        }
        Ok(())
    }

    /// `plove apply` этого файла. Вывод — в терминал, где запущен studio:
    /// там же sudo спросит пароль, если нужно поставить пакеты.
    fn apply(&self) -> Result<String> {
        let exe = std::env::current_exe()?;
        let mut command = Command::new(exe);
        if self.env.sandbox {
            command.arg("--root").arg(&self.env.home);
        }
        command.arg("apply").arg(&self.file).arg("-y");
        println!("\nplove apply {}", self.env.display(&self.file));
        let status = command
            .status()
            .context("не удалось запустить plove apply")?;
        if !status.success() {
            bail!("plove apply завершился с ошибкой, подробности в терминале");
        }
        Ok("Применено".into())
    }
}

struct Request {
    method: String,
    /// Путь без `?query`.
    path: String,
    body: Vec<u8>,
    /// Запрос со страницы studio: есть заголовок X-Plove, Host — 127.0.0.1.
    ours: bool,
}

fn read_request(stream: &mut TcpStream) -> Result<Request> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("/");
    let path = target.split('?').next().unwrap_or("/").to_string();
    let mut length = 0usize;
    let (mut marked, mut local) = (false, false);
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 || header == "\r\n" || header == "\n" {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                length = value.trim().parse().unwrap_or(0);
            } else if name.eq_ignore_ascii_case("x-plove") {
                marked = true;
            } else if name.eq_ignore_ascii_case("host") {
                let host = value.trim();
                local = host.starts_with("127.0.0.1:") || host.starts_with("localhost:");
            }
        }
    }
    // Раскладка — несколько килобайт; больше мегабайта — явно не от нас.
    if length > 1 << 20 {
        bail!("слишком большой запрос");
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    Ok(Request {
        method,
        path,
        body,
        ours: marked && local,
    })
}

fn respond(stream: &mut TcpStream, code: u16, kind: &str, body: &[u8]) -> Result<()> {
    let status = match code {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        _ => "Internal Server Error",
    };
    write!(
        stream,
        "HTTP/1.1 {code} {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    Ok(())
}

fn respond_json(stream: &mut TcpStream, code: u16, value: &serde_json::Value) -> Result<()> {
    respond(
        stream,
        code,
        "application/json; charset=utf-8",
        value.to_string().as_bytes(),
    )
}

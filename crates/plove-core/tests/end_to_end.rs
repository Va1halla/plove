//! Сквозной тест: план → применение → повтор → откат, всё в песочнице.
//! Запуск: `cargo test`.

use std::fs;

use plove_core::apply::{apply, ApplyOptions};
use plove_core::desk;
use plove_core::modules::Registry;
use plove_core::plan::{self, FileStatus};
use plove_core::presets;
use plove_core::spec::RiceSpec;
use plove_core::state;
use plove_core::system::Env;

/// tokyo-night с панелью Waybar и её меню eww-popups: многие тесты
/// проверяют их конфиги.
/// Панель Quickshell (по умолчанию в готовых райсах) — в своих тестах.
fn example_spec() -> RiceSpec {
    let mut spec = presets::get("tokyo-night").unwrap().unwrap();
    spec.components.insert("bar".into(), "waybar".into());
    spec.components.insert("popups".into(), "eww-popups".into());
    spec
}

#[test]
fn apply_is_idempotent_and_rollback_restores() {
    // tempdir — временная папка, которая удалится после теста.
    let home = tempfile::tempdir().unwrap();
    let env = Env::sandbox(home.path().to_path_buf());
    let registry = Registry::builtin().unwrap();
    let options = ApplyOptions {
        skip_packages: true,
        no_confirm: true,
    };

    // Пусть у пользователя уже есть свой конфиг kitty.
    let kitty = home.path().join(".config/kitty/kitty.conf");
    fs::create_dir_all(kitty.parent().unwrap()).unwrap();
    fs::write(&kitty, "font_size 20\n").unwrap();

    // Пусть у пользователя уже есть свой ~/.zshrc.
    let zshrc = home.path().join(".zshrc");
    fs::write(&zshrc, "export EDITOR=nvim").unwrap();

    // Первое применение: все конфиги новые, kitty и ~/.zshrc меняются.
    let first = plan::build(example_spec(), &registry, &env).unwrap();
    assert!(first.warnings.iter().all(|w| !w.contains("занята")));
    assert_eq!(first.changed_files().len(), 38);
    let report = apply(&first, &env, &options).unwrap();
    assert_eq!(report.generation, Some(1));

    let hypr = fs::read_to_string(home.path().join(".config/hypr/hyprland.conf")).unwrap();
    assert!(hypr.contains("col.active_border = rgb(7aa2f7)"));
    assert!(hypr.contains("bind = $mod, Return, exec, kitty"));
    assert!(hypr.contains("kb_layout = us,ru"));
    assert!(hypr.contains("exec-once = sh ~/.local/share/plove/swww.sh"));
    assert!(hypr.contains("bind = $mod SHIFT, F, exec, kitty yazi"));
    // Строка дописана в конец, свои настройки пользователя на месте.
    assert_eq!(
        fs::read_to_string(&zshrc).unwrap(),
        "export EDITOR=nvim\nsource ~/.config/zsh/plove.zsh\n"
    );
    assert!(home
        .path()
        .join(".local/share/plove/wallpaper.png")
        .exists());
    assert!(fs::read_to_string(&kitty)
        .unwrap()
        .contains("color15 #c0caf5"));
    assert!(home.path().join(".config/hypr/user.conf").exists());

    // Повтор с тем же райсом ничего не меняет.
    let second = plan::build(example_spec(), &registry, &env).unwrap();
    assert!(second.is_noop());
    assert!(second
        .files
        .iter()
        .all(|f| f.status == FileStatus::Unchanged));

    // Откат возвращает старый kitty и удаляет созданные файлы.
    state::rollback(&env).unwrap().unwrap();
    assert_eq!(fs::read_to_string(&kitty).unwrap(), "font_size 20\n");
    assert!(!home.path().join(".config/hypr/hyprland.conf").exists());
    assert!(state::list(&env).unwrap().is_empty());
}

#[test]
fn incompatible_and_unknown_are_rejected() {
    let home = tempfile::tempdir().unwrap();
    let env = Env::sandbox(home.path().to_path_buf());
    let registry = Registry::builtin().unwrap();

    let mut spec = example_spec();
    spec.components.insert("terminal".into(), "foot".into());
    let error = plan::build(spec, &registry, &env).err().unwrap();
    assert!(error.to_string().contains("foot"));

    let mut spec = example_spec();
    spec.overrides
        .get_mut("waybar")
        .unwrap()
        .insert("positon".into(), "bottom".into());
    let error = plan::build(spec, &registry, &env).err().unwrap();
    assert!(error.to_string().contains("positon"));
}

#[test]
fn all_presets_build() {
    let home = tempfile::tempdir().unwrap();
    let env = Env::sandbox(home.path().to_path_buf());
    let registry = Registry::builtin().unwrap();
    let all = presets::all().unwrap();
    assert_eq!(all.len(), 4);
    for spec in all {
        let name = spec.name.clone();
        let plan = plan::build(spec, &registry, &env).unwrap_or_else(|e| panic!("{name}: {e:#}"));
        assert!(
            plan.warnings.iter().all(|w| !w.contains("components")),
            "{name}"
        );
        // Ни одна клавиша не занята двумя модулями.
        assert!(
            plan.warnings.iter().all(|w| !w.contains("занята")),
            "{name}: {:?}",
            plan.warnings
        );
    }
}

/// Убранная из райса фоновая программа останавливается при следующем применении.
#[test]
fn removed_daemon_is_stopped() {
    let home = tempfile::tempdir().unwrap();
    let env = Env::sandbox(home.path().to_path_buf());
    let registry = Registry::builtin().unwrap();
    let options = ApplyOptions {
        skip_packages: true,
        no_confirm: true,
    };
    let first = plan::build(example_spec(), &registry, &env).unwrap();
    apply(&first, &env, &options).unwrap();

    let mut spec = example_spec();
    spec.components.remove("widgets");
    let second = plan::build(spec, &registry, &env).unwrap();
    assert!(second
        .reloads
        .contains(&"pkill -f '[e]ww open.* desktop'; true".to_string()));
    // Оставшиеся программы не трогаем.
    assert!(!second.reloads.iter().any(|r| r.contains("'waybar'")));
}

/// Мониторы: строка Hyprland как есть, описание — с позицией и поворотом.
#[test]
fn monitors_above_and_rotated() {
    let home = tempfile::tempdir().unwrap();
    let env = Env::sandbox(home.path().to_path_buf());
    let registry = Registry::builtin().unwrap();
    let mut spec = example_spec();
    let monitors: serde_yaml::Value = serde_yaml::from_str(
        "[\"eDP-1, 1920x1080@60, 0x0, 1\", {name: DP-1, position: above}, {name: HDMI-A-1, position: left, rotate: 90}]",
    )
    .unwrap();
    spec.overrides
        .get_mut("hyprland")
        .unwrap()
        .insert("monitors".into(), monitors);
    let plan = plan::build(spec, &registry, &env).unwrap();
    let hypr = plan
        .files
        .iter()
        .find(|f| f.path.ends_with(".config/hypr/hyprland.conf"))
        .unwrap();
    let text = String::from_utf8(hypr.content.clone()).unwrap();
    assert!(text.contains("monitor = eDP-1, 1920x1080@60, 0x0, 1\n"));
    assert!(text.contains("monitor = DP-1, preferred, auto-up, 1\n"));
    assert!(text.contains("monitor = HDMI-A-1, preferred, auto-left, 1, transform, 1\n"));
}

/// Несколько панелей: главная на одном мониторе, вертикальная на другом.
#[test]
fn several_bars() {
    let home = tempfile::tempdir().unwrap();
    let env = Env::sandbox(home.path().to_path_buf());
    let registry = Registry::builtin().unwrap();
    let mut spec = example_spec();
    let waybar = spec.overrides.get_mut("waybar").unwrap();
    waybar.insert("monitors".into(), serde_yaml::from_str("[eDP-1]").unwrap());
    waybar.insert(
        "bars".into(),
        serde_yaml::from_str(
            "[{position: left, monitors: [HDMI-A-1], center: [workspaces, clock]}]",
        )
        .unwrap(),
    );
    let plan = plan::build(spec, &registry, &env).unwrap();
    let config = plan
        .files
        .iter()
        .find(|f| f.path.ends_with(".config/waybar/config.jsonc"))
        .unwrap();
    let text = String::from_utf8(config.content.clone()).unwrap();
    assert!(text.contains("\"output\": [\"eDP-1\"]"));
    assert!(text.contains("\"output\": [\"HDMI-A-1\"]"));
    assert!(text.contains("\"position\": \"left\""));
    // Часы на вертикальной панели — в две строки.
    assert!(text.contains("\"format\": \"{:%H\\n%M}\""));
}

/// Свой виджет попадает и на панель, и на рабочий стол; опечатка в списке — ошибка.
#[test]
fn custom_widgets() {
    let home = tempfile::tempdir().unwrap();
    let env = Env::sandbox(home.path().to_path_buf());
    let registry = Registry::builtin().unwrap();
    let mut spec = example_spec();
    spec.widgets.insert(
        "vpn".into(),
        serde_yaml::from_str(
            "{command: \"echo on\", icon: X, text: \"VPN {value}\", color: green}",
        )
        .unwrap(),
    );
    let waybar = spec.overrides.get_mut("waybar").unwrap();
    waybar.insert(
        "modules_right".into(),
        serde_yaml::from_str("[vpn, clock]").unwrap(),
    );
    spec.overrides.insert(
        "eww".into(),
        serde_yaml::from_str("{cards: [clock, vpn]}").unwrap(),
    );
    let plan = plan::build(spec.clone(), &registry, &env).unwrap();
    let text = |end: &str| {
        let file = plan.files.iter().find(|f| f.path.ends_with(end)).unwrap();
        String::from_utf8(file.content.clone()).unwrap()
    };
    assert!(text(".config/waybar/config.jsonc").contains("\"custom/vpn\": {"));
    assert!(text(".config/waybar/style.css").contains("#custom-vpn {\n    color: #9ece6a;"));
    assert!(text(".config/eww/eww.yuck").contains("(label :text \"X VPN ${w_vpn}\")"));

    let waybar = spec.overrides.get_mut("waybar").unwrap();
    waybar.insert(
        "modules_right".into(),
        serde_yaml::from_str("[vnp]").unwrap(),
    );
    let error = plan::build(spec, &registry, &env).err().unwrap();
    assert!(error.to_string().contains("vnp"));
}

/// Настройки модулей попадают не только в конфиги, но и в команды,
/// клавиши и переменные окружения.
#[test]
fn settings_reach_commands() {
    let home = tempfile::tempdir().unwrap();
    let env = Env::sandbox(home.path().to_path_buf());
    let registry = Registry::builtin().unwrap();
    let mut spec = example_spec();
    spec.components.insert("launcher".into(), "fuzzel".into());
    for (module, options) in [
        ("grim", "{folder: ~/Shots}"),
        ("cliphist", "{max_items: 100}"),
        ("gtk", "{cursor_size: 32}"),
        ("recorder", "{audio: true}"),
        ("fuzzel", "{width: 60}"),
    ] {
        spec.overrides
            .insert(module.into(), serde_yaml::from_str(options).unwrap());
    }
    let plan = plan::build(spec, &registry, &env).unwrap();
    let file = |name: &str| {
        let f = plan.files.iter().find(|f| f.path.ends_with(name)).unwrap();
        String::from_utf8(f.content.clone()).unwrap()
    };
    let hypr = file("hypr/hyprland.conf");
    assert!(hypr.contains("tee ~/Shots/"), "{hypr}");
    assert!(hypr.contains("cliphist -max-items 100 store"));
    assert!(hypr.contains("env = XCURSOR_SIZE,32"));
    assert!(hypr.contains("wf-recorder --audio"));
    assert!(file("fuzzel/fuzzel.ini").contains("width=60"));
    assert!(plan.on_apply.iter().any(|c| c.ends_with("cursor-size 32")));
}

/// Конфиг убранной из райса программы удаляется, а откат его возвращает.
#[test]
fn removed_module_config_is_deleted() {
    let home = tempfile::tempdir().unwrap();
    let env = Env::sandbox(home.path().to_path_buf());
    let registry = Registry::builtin().unwrap();
    let options = ApplyOptions {
        skip_packages: true,
        no_confirm: true,
    };
    let first = plan::build(example_spec(), &registry, &env).unwrap();
    apply(&first, &env, &options).unwrap();
    let yazi = home.path().join(".config/yazi/theme.toml");
    assert!(yazi.exists());

    let mut spec = example_spec();
    spec.components.remove("terminal_files");
    let second = plan::build(spec, &registry, &env).unwrap();
    let removed: Vec<_> = second
        .files
        .iter()
        .filter(|f| f.status == FileStatus::Remove)
        .map(|f| f.path.clone())
        .collect();
    assert_eq!(removed, vec![yazi.clone()]);
    apply(&second, &env, &options).unwrap();
    assert!(!yazi.exists());

    state::rollback(&env).unwrap();
    assert!(yazi.exists());
}

/// Раскладка из plove studio: каждая карточка — своё окно eww,
/// и все окна открываются при входе.
#[test]
fn desk_cards_become_eww_windows() {
    let home = tempfile::tempdir().unwrap();
    let env = Env::sandbox(home.path().to_path_buf());
    let registry = Registry::builtin().unwrap();
    let mut spec = example_spec();
    spec.desktop = serde_yaml::from_str(
        "[{kind: clock, at: [5, 8]}, {kind: weather, at: [70, 6], scale: 1.5, with: {city: Москва}}]",
    )
    .unwrap();
    spec.validate().unwrap();
    let plan = plan::build(spec, &registry, &env).unwrap();
    let file = |name: &str| {
        let f = plan.files.iter().find(|f| f.path.ends_with(name)).unwrap();
        String::from_utf8(f.content.clone()).unwrap()
    };
    let yuck = file("eww/eww.yuck");
    assert!(yuck.contains("(defwindow desktop\n"), "{yuck}");
    assert!(yuck.contains("(defwindow desktop1\n"));
    assert!(yuck.contains(":x \"70%\" :y \"6%\" :width \"480px\" :height \"165px\""));
    assert!(yuck.contains("desk-weather.sh 'Москва'"));
    let scss = file("eww/eww.scss");
    assert!(scss.contains(".c1 .temp { font-size: 45px; }"), "{scss}");
    let hypr = file("hypr/hyprland.conf");
    assert!(hypr.contains("exec-once = eww open-many desktop desktop1"));
}

/// Каждый вид карточки даёт целое окно eww: скобки сходятся. Лишняя
/// или потерянная скобка ломает весь eww.yuck, и на столе пусто.
#[test]
fn every_desk_card_closes_its_window() {
    let home = tempfile::tempdir().unwrap();
    let env = Env::sandbox(home.path().to_path_buf());
    let registry = Registry::builtin().unwrap();
    let mut spec = example_spec();
    spec.widgets.insert(
        "vpn".into(),
        serde_yaml::from_str("{command: \"echo on\", text: \"VPN {value}\"}").unwrap(),
    );
    let catalog = desk::catalog_json();
    let kinds: Vec<&str> = catalog
        .as_array()
        .unwrap()
        .iter()
        .map(|k| k["id"].as_str().unwrap())
        .collect();
    for kind in kinds {
        let with = match kind {
            "weather" => "{city: Meppel}",
            "widget" => "{id: vpn}",
            _ => "{}",
        };
        spec.desktop =
            serde_yaml::from_str(&format!("[{{kind: {kind}, at: [5, 5], with: {with}}}]")).unwrap();
        spec.validate().unwrap();
        let plan = plan::build(spec.clone(), &registry, &env).unwrap();
        let file = plan
            .files
            .iter()
            .find(|f| f.path.ends_with("eww/eww.yuck"))
            .unwrap();
        let yuck = String::from_utf8(file.content.clone()).unwrap();
        assert_eq!(yuck_depth(&yuck), Ok(0), "карточка {kind}:\n{yuck}");
        // Кириллица в /* */ доживает до CSS, grass ставит перед ним
        // @charset, а GTK такого правила не знает и не грузит стили.
        let file = plan
            .files
            .iter()
            .find(|f| f.path.ends_with("eww/eww.scss"))
            .unwrap();
        let scss = String::from_utf8(file.content.clone()).unwrap();
        for comment in scss.split("/*").skip(1) {
            let comment = comment.split("*/").next().unwrap();
            assert!(comment.is_ascii(), "не ASCII в /* */ в eww.scss: {comment}");
        }
    }
}

/// Глубина скобок в конце файла yuck, без строк и комментариев.
/// `Err` — закрывающая скобка без открывающей.
fn yuck_depth(text: &str) -> Result<i32, usize> {
    let (mut depth, mut string, mut escaped, mut comment) = (0, false, false, false);
    for (i, ch) in text.char_indices() {
        if comment {
            comment = ch != '\n';
        } else if string {
            match ch {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => string = false,
                _ => {}
            }
        } else {
            match ch {
                ';' => comment = true,
                '"' => string = true,
                '(' => depth += 1,
                ')' if depth == 0 => return Err(i),
                ')' => depth -= 1,
                _ => {}
            }
        }
    }
    Ok(depth)
}

/// Оформление карточки из look попадает в eww.scss, ошибка в цвете — понятная.
#[test]
fn desk_card_look() {
    let home = tempfile::tempdir().unwrap();
    let env = Env::sandbox(home.path().to_path_buf());
    let registry = Registry::builtin().unwrap();
    let mut spec = example_spec();
    spec.desktop = serde_yaml::from_str(
        "[{kind: clock, at: [5, 8], look: {font: Fancy Sans, accent: red, fill: '#000000', opacity: 0.5, radius: 30, border: 2}},
          {kind: note, at: [50, 50], look: {fill: none, text: fg}}]",
    )
    .unwrap();
    spec.validate().unwrap();
    let plan = plan::build(spec.clone(), &registry, &env).unwrap();
    let file = plan
        .files
        .iter()
        .find(|f| f.path.ends_with("eww/eww.scss"))
        .unwrap();
    let scss = String::from_utf8(file.content.clone()).unwrap();
    let red = spec.theme.palette.named("red").unwrap().hex();
    assert!(
        scss.contains(".c0, .c0 * { font-family: \"Fancy Sans\""),
        "{scss}"
    );
    assert!(scss.contains(".c0 .time, .c0 .zone-time"));
    assert!(scss.contains(&format!("{{ color: {red}; }}")));
    assert!(scss.contains(".c0 { background-color: rgba(0, 0, 0, 0.5); }"));
    assert!(scss.contains(".c0 { border-radius: 30px; }"));
    assert!(scss.contains(&format!(".c0 {{ border: 2px solid {red}; }}")));
    assert!(scss.contains(".c1 { background-color: transparent; }"));
    // Шрифт не из списка plove и не установлен — предупреждение, а не ошибка.
    assert!(plan.warnings.iter().any(|w| w.contains("Fancy Sans")));

    spec.desktop[0].look.accent = Some("pink".into());
    let error = spec.validate().unwrap_err().to_string();
    assert!(error.contains("look.accent: «pink» — не цвет"), "{error}");
    spec.desktop[0].look.accent = None;
    spec.desktop[0].look.font = Some("Bad\"; }".into());
    assert!(spec.validate().is_err());
}

/// Панель Quickshell: свои QML как есть, Theme.qml и settings.json из
/// райса, Waybar не запускается, шрифты Google Fonts — в плане скачивания.
#[test]
fn quickshell_bar() {
    let home = tempfile::tempdir().unwrap();
    let env = Env::sandbox(home.path().to_path_buf());
    let registry = Registry::builtin().unwrap();
    let mut spec = example_spec();
    spec.components.insert("bar".into(), "quickshell".into());
    // У панели Quickshell свои выпадающие меню.
    spec.components.remove("popups");
    spec.overrides.remove("waybar");
    for style in ["segments", "rail", "island", "dock"] {
        spec.overrides.insert(
            "quickshell".into(),
            serde_yaml::from_str(&format!(
                "{{style: {style}, font_display: Doto, modules_right: [volume, battery]}}"
            ))
            .unwrap(),
        );
        let plan = plan::build(spec.clone(), &registry, &env).unwrap();
        let text = |end: &str| {
            let file = plan.files.iter().find(|f| f.path.ends_with(end)).unwrap();
            String::from_utf8(file.content.clone()).unwrap()
        };
        let settings: serde_json::Value =
            serde_json::from_str(&text("quickshell/plove/settings.json")).unwrap();
        assert_eq!(settings["style"], style);
        assert_eq!(settings["right"], serde_json::json!(["volume", "battery"]));
        let theme = text("quickshell/plove/Theme.qml");
        assert!(theme.contains("readonly property string fontDisplay: \"Doto\""));
        assert!(
            theme.contains("fonts/plove/doto/Doto%5BROND,wght%5D.ttf"),
            "{theme}"
        );
        // QML копируется как есть, с фигурными скобками.
        assert!(text("quickshell/plove/Bar.qml").contains("PanelWindow {"));
        // Свои выпадающие меню вместо eww-popups.
        for popup in ["VolumePopup", "NetworkPopup", "CalendarPopup", "CalcPopup"] {
            assert!(text(&format!("quickshell/plove/{popup}.qml")).contains("Popup {"));
        }
        assert!(plan.packages.iter().any(|p| p.name == "libqalculate"));
        let hypr = text("hypr/hyprland.conf");
        assert!(hypr.contains("exec-once = qs -c plove"));
        assert!(!hypr.contains("exec-once = waybar"));
        assert!(!plan
            .files
            .iter()
            .any(|f| f.path.ends_with("waybar/config.jsonc")));
        let doto = plan.fonts.iter().find(|f| f.name == "Doto").unwrap();
        assert!(!doto.installed && doto.urls[0].ends_with("ofl/doto/Doto[ROND,wght].ttf"));
        assert!(plan.packages.iter().any(|p| p.name == "quickshell"));
    }
    // Опечатка в имени шрифта не доходит до QML.
    spec.overrides.insert(
        "quickshell".into(),
        serde_yaml::from_str("{font_text: 'Bad\"; }'}").unwrap(),
    );
    assert!(plan::build(spec, &registry, &env).is_err());
}

/// Hyprland 0.57+: hyprland.lua вместо hyprland.conf, старый файл plove
/// убирает, hyprctl в модулях — в синтаксисе Lua. Если Hyprland стоит,
/// он сам проверяет получившийся конфиг.
#[test]
fn hyprland_lua_config() {
    let home = tempfile::tempdir().unwrap();
    let mut env = Env::sandbox(home.path().to_path_buf());
    env.hyprland_lua = false;
    let registry = Registry::builtin().unwrap();
    let options = ApplyOptions {
        skip_packages: true,
        no_confirm: true,
    };
    let mut spec = example_spec();
    spec.components.insert("power".into(), "power-menu".into());
    // Сначала старый Hyprland: только hyprland.conf.
    let old = plan::build(spec.clone(), &registry, &env).unwrap();
    assert!(old
        .files
        .iter()
        .any(|f| f.path.ends_with("hypr/hyprland.conf")));
    assert!(!old
        .files
        .iter()
        .any(|f| f.path.ends_with("hypr/hyprland.lua")));
    apply(&old, &env, &options).unwrap();

    env.hyprland_lua = true;
    let plan = plan::build(spec, &registry, &env).unwrap();
    let file = |end: &str| plan.files.iter().find(|f| f.path.ends_with(end)).unwrap();
    assert_eq!(file("hypr/hyprland.conf").status, FileStatus::Remove);
    let lua = String::from_utf8(file("hypr/hyprland.lua").content.clone()).unwrap();
    assert!(lua.contains("hl.on(\"hyprland.start\", function()"));
    assert!(
        lua.contains("hl.bind(\"SUPER + Return\", hl.dsp.exec_cmd(\"kitty\"))"),
        "{lua}"
    );
    assert!(lua.contains("hl.bind(\"SUPER + 1\"") || lua.contains("for i = 1, 9 do"));
    assert!(lua.contains("Выйти) hyprctl dispatch 'hl.dsp.exit()'"));
    assert!(lua.contains("pcall(dofile, user)"));
    let idle = String::from_utf8(file("hypr/hypridle.conf").content.clone()).unwrap();
    assert!(idle.contains("hyprctl dispatch 'hl.dsp.dpms({ action = \"off\" })'"));
    apply(&plan, &env, &options).unwrap();
    let path = home.path().join(".config/hypr/hyprland.lua");
    assert!(path.exists() && !home.path().join(".config/hypr/hyprland.conf").exists());

    if let Ok(out) = std::process::Command::new("Hyprland")
        .arg("--verify-config")
        .arg("-c")
        .arg(&path)
        .output()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(text.contains("config ok"), "{text}");
    }
}

//! Сквозной тест: план → применение → повтор → откат, всё в песочнице.
//! Запуск: `cargo test`.

use std::fs;

use plove_core::apply::{apply, ApplyOptions};
use plove_core::modules::Registry;
use plove_core::plan::{self, FileStatus};
use plove_core::presets;
use plove_core::spec::RiceSpec;
use plove_core::state;
use plove_core::system::Env;

fn example_spec() -> RiceSpec {
    presets::get("tokyo-night").unwrap().unwrap()
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
    assert_eq!(first.changed_files().len(), 35);
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
        .contains(&"pkill -f '[e]ww open desktop'; true".to_string()));
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

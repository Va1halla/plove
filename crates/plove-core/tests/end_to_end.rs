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
    assert_eq!(first.changed_files().len(), 26);
    let report = apply(&first, &env, &options).unwrap();
    assert_eq!(report.generation, Some(1));

    let hypr = fs::read_to_string(home.path().join(".config/hypr/hyprland.conf")).unwrap();
    assert!(hypr.contains("col.active_border = rgb(7aa2f7)"));
    assert!(hypr.contains("bind = $mod, Return, exec, kitty"));
    assert!(hypr.contains("kb_layout = us,ru"));
    assert!(hypr.contains("exec-once = swww-daemon"));
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

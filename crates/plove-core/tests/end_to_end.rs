//! Сквозной тест: план → применение → повтор → откат, всё в песочнице.
//! Запуск: `cargo test`.

use std::fs;
use std::path::Path;

use plove_core::apply::{apply, ApplyOptions};
use plove_core::modules::Registry;
use plove_core::plan::{self, FileStatus};
use plove_core::spec::RiceSpec;
use plove_core::state;
use plove_core::system::Env;

fn example_spec() -> RiceSpec {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/tokyo-night.yaml");
    RiceSpec::load(&path).unwrap()
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

    // Первое применение: hyprland и waybar новые, kitty меняется.
    let first = plan::build(example_spec(), &registry, &env).unwrap();
    assert_eq!(first.changed_files().len(), 4);
    let report = apply(&first, &env, &options).unwrap();
    assert_eq!(report.generation, Some(1));

    let hypr = fs::read_to_string(home.path().join(".config/hypr/hyprland.conf")).unwrap();
    assert!(hypr.contains("col.active_border = rgb(7aa2f7)"));
    assert!(hypr.contains("bind = $mod, Return, exec, kitty"));
    assert!(hypr.contains("kb_layout = us,ru"));
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
    spec.components
        .insert("terminal".into(), "alacritty".into());
    let error = plan::build(spec, &registry, &env).err().unwrap();
    assert!(error.to_string().contains("alacritty"));

    let mut spec = example_spec();
    spec.overrides
        .get_mut("waybar")
        .unwrap()
        .insert("positon".into(), "bottom".into());
    let error = plan::build(spec, &registry, &env).err().unwrap();
    assert!(error.to_string().contains("positon"));
}

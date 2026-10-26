# plove

Конструктор райсов для Arch Linux: описываешь рабочий стол одним YAML-файлом, plove ставит пакеты и пишет конфиги.

> Ранняя версия: есть ядро и CLI, модули Hyprland, Waybar и kitty. Готовые райсы, превью и TUI-мастер появятся следом.

## Быстрый старт

```sh
cargo build --release
# Посмотреть, что изменится, ничего не трогая:
./target/release/plove plan examples/tokyo-night.yaml
# Попробовать в песочнице: всё пишется в /tmp/plove-test, система не меняется:
./target/release/plove --root /tmp/plove-test apply examples/tokyo-night.yaml
# На настоящей системе:
./target/release/plove apply examples/tokyo-night.yaml
# Вернуть как было:
./target/release/plove rollback
```

Другие команды: `plove history` (история применений), `plove modules` (список модулей), `plove --help`.

## Как это работает

```text
rice.yaml ──► spec    разбор и проверка райса
          ──► plan    подбор модулей, рендер конфигов, сравнение с диском
          ──► apply   пакеты, копия старых файлов, запись, перезагрузка программ
          ──► state   история применений и откат
```

- **Райс — это данные.** Всё описано в RiceSpec (`examples/tokyo-night.yaml`): дизайн-токены (цвета, шрифты, отступы) отдельно от компонентов (какие программы).
- **Модуль = папка** в `modules/`: `module.yaml` плюс шаблоны конфигов на [Tera](https://keats.github.io/tera/) (синтаксис как у Jinja2). Новая программа добавляется новой папкой, ядро не меняется.
- **Сначала план, потом действие.** `plan` ничего не меняет. `apply` показывает план и спрашивает подтверждение.
- **Безопасный откат.** Перед записью plove сохраняет старые файлы в `~/.local/state/plove/generations/`. `plove rollback` возвращает их.
- **Ручные правки не теряются.** Сгенерированные файлы перезаписываются, а свои настройки пишите в `~/.config/hypr/user.conf` и `~/.config/kitty/user.conf`: plove создаёт их пустыми и никогда не трогает.

## Где что лежит

| Путь | Что там |
| --- | --- |
| `crates/plove-core/src/spec.rs` | формат райса и его проверка |
| `crates/plove-core/src/theme.rs` | токены темы → данные для шаблонов |
| `crates/plove-core/src/modules.rs` | загрузка модулей |
| `crates/plove-core/src/plan.rs` | построение плана |
| `crates/plove-core/src/apply.rs` | применение плана |
| `crates/plove-core/src/state.rs` | история и откат |
| `crates/plove-core/src/system.rs` | пути, pacman, запуск команд |
| `crates/plove-cli/src/main.rs` | командная строка |
| `modules/` | модули программ |
| `examples/` | примеры райсов |

Тесты: `cargo test`.

## Лицензия

На выбор: [MIT](LICENSE) или [Apache-2.0](LICENSE-APACHE).

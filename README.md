# plove

Конструктор райсов для Arch Linux: описываешь рабочий стол одним YAML-файлом, plove ставит пакеты и пишет конфиги.

> Ранняя версия: ядро, CLI, TUI-мастер, 4 готовых райса, 32 модуля и превью.

## Готовые райсы

| tokyo-night | catppuccin-mocha |
| --- | --- |
| ![tokyo-night](docs/previews/tokyo-night.png) | ![catppuccin-mocha](docs/previews/catppuccin-mocha.png) |
| **gruvbox** | **nord** |
| ![gruvbox](docs/previews/gruvbox.png) | ![nord](docs/previews/nord.png) |

Это превью, которые рисует сама plove (`plove preview <райс>`), а не скриншоты: настоящий рабочий стол выглядит так же по цветам, обоям и геометрии, но содержимое окон выдумано.

## Быстрый старт

```sh
cargo build --release
# Мастер: проверка системы, выбор райса, стиль и программы, тонкая настройка, план:
./target/release/plove
# Проверить систему отдельно (и починить то, что можно, с --fix):
./target/release/plove doctor
# Готовые райсы:
./target/release/plove presets
# Нарисовать превью в tokyo-night.png:
./target/release/plove preview tokyo-night
# Посмотреть, что изменится, ничего не трогая:
./target/release/plove plan tokyo-night
# Попробовать в песочнице: всё пишется в /tmp/plove-test, система не меняется:
./target/release/plove --root /tmp/plove-test apply tokyo-night
# На настоящей системе:
./target/release/plove apply tokyo-night
# Вернуть как было:
./target/release/plove rollback
```

Вместо имени готового райса можно передать путь к своему файлу: `plove apply ~/my-rice.yaml`.

### Свой райс из обоев

Готовые райсы не обязательны. В мастере (`plove`) на шаге «Райс» есть пункт **+ Своя картинка**: выберите картинку из `~/Pictures`, `~/Downloads`, `~/wallpapers` или наберите путь. plove возьмёт из неё цвета, а стиль, программы и тонкие настройки вы выберете на следующих шагах. Основа — нейтральный набор программ из официальных репозиториев, ни один готовый райс не используется. После применения райс сохраняется в `~/.config/plove/rices/<имя>.yaml`.

То же из командной строки:

```sh
# Цвета из картинки (PNG, JPEG, WebP), остальное — нейтральная основа:
./target/release/plove new ~/Pictures/my-wall.jpg
./target/release/plove edit my-wall.yaml      # стиль и программы в мастере
./target/release/plove apply my-wall.yaml
# Или взять шрифты, стиль и программы из готового райса:
./target/release/plove new ~/Pictures/my-wall.jpg --base nord
```

Тема всегда получается тёмной. Цвета подбираются автоматически, а файл райса можно поправить руками.

Подбор можно направить. Это помогает на обоях из оттенков одного цвета, где акцент иначе сливается с картинкой:

```sh
# Акцент: auto (самый яркий), contrast (другой оттенок, чем у большей части
# картинки, например цветы на голубых досках), dominant (основной цвет) или #rrggbb.
# Фон: tinted (с оттенком обоев), neutral (почти серый), deep (темнее).
# Цвета терминала: soft или vivid.
./target/release/plove new ~/Pictures/my-wall.jpg --accent contrast --background neutral --terminal vivid
# Пересчитать цвета готового своего райса, остальное не трогая:
./target/release/plove recolor my-wall.yaml --accent contrast
```

В мастере у райса с обоями-картинкой на шаге «Настройки» есть строки «Акцент», «Фон» и «Цвета терминала», превью меняется сразу. У готовых райсов там одна строка «Акцент»: как в райсе, контрастный (противоположный оттенок) или один из ярких цветов терминала — красный, зелёный, жёлтый, синий, пурпурный, голубой.

Поменять свой райс в мастере: `plove edit my-wall.yaml`. Райс из файла встанет первым в списке, а после подтверждения plove сохранит изменения обратно (старый файл останется рядом как `my-wall.yaml.bak`: комментарии при сохранении теряются) и применит. Если файла ещё нет, мастер начнёт с готовых райсов и сохранит выбранный в этот файл.

В мастере рядом со списком райсов и настройками стиля сразу видна картинка-превью, она меняется вместе с настройками. В терминалах с поддержкой картинок (kitty, ghostty, WezTerm, foot, Konsole) это настоящее изображение, в остальных — набросок цветными полублоками. В консоли Linux (TTY) картинки нет: там всего 16 цветов.

Конфиги программ, которых в новом райсе нет, plove удаляет (только те, что записала сама; `plove plan` показывает их с «-», а `plove rollback` возвращает).

Сменить райс за пару секунд, без вопросов и установки пакетов: `plove switch nord`. Если новому райсу не хватает программ, plove подскажет сделать `plove apply`.

Другие команды: `plove history` (история применений), `plove modules` (список модулей), `plove --help`.

## Стиль

Кроме цветов, у райса есть стиль: несколько крупных решений, которые сразу меняют панель, меню, уведомления, экран блокировки и рамки окон.

```yaml
theme:
  style:
    shape: pill        # sharp, rounded, pill — углы и «таблетки»
    bar: islands       # solid, floating, islands, minimal
    separators: none   # none, dots, line, arrows, slants, round
    borders: gradient  # none, thin, accent, gradient (вращается)
    density: normal    # compact, normal, spacious
    labels: glyphs     # glyphs, text, both — значки или слова на панели
    glass: light       # none, light, strong — прозрачность и размытие
```

Всё это можно выбрать и в мастере (`plove`) на экране настроек стрелками ←→. Состав панели задаётся в `overrides.waybar`: `modules_left`, `modules_center`, `modules_right` из launcher, workspaces, window, clock, media, cpu, memory, backlight, pulseaudio, network, battery, language, tray, power. Панель можно поставить и вертикально: `position: left`. Панелей может быть несколько, в том числе свои на разных мониторах:

```yaml
overrides:
  waybar:
    monitors: [eDP-1]                 # главная панель только на ноутбуке
    bars:
      - { position: left, monitors: [HDMI-A-1], center: [workspaces, clock] }
```

## Свои виджеты

Виджет — это команда, которая раз в несколько секунд выдаёт строку. Его можно поставить на панель или на рабочий стол рядом с готовыми:

```yaml
widgets:
  vpn:
    command: "nmcli -t -f NAME connection show --active | grep -q wg0 && echo on || echo off"
    every: 5             # секунды
    icon: "󰖂"
    text: "VPN {value}"
    color: green         # accent, fg, muted, red, green, yellow, blue, magenta, cyan
    click: "nmcli connection up wg0"
overrides:
  waybar: { modules_right: [vpn, pulseaudio, clock] }
  eww: { cards: [clock, system, media, vpn], anchor: top-right }
```

На рабочем столе (eww) есть готовые карточки: `clock` (часы и дата), `system` (кольца процессора, памяти и диска), `media` (музыка с кнопками). Проверить свой виджет до применения: `plove widget my.yaml vpn`.

## Рабочий стол: plove studio

Карточки на рабочем столе удобнее всего расставлять мышью:

```sh
./target/release/plove studio ~/.config/plove/rices/my.yaml   # или имя готового райса
```

Откроется страница в браузере: слева каталог, посередине ваш рабочий стол (обои и панель), справа настройки выбранной карточки. Карточки перетаскиваются мышью и прилипают к краям и центру экрана (с `Shift` — без привязки), размер меняется за уголок, `Del` удаляет, стрелки двигают точнее. «Сохранить» пишет раскладку в файл райса (старый остаётся рядом как `.bak`), «Применить» ещё и запускает `plove apply`: его вывод и вопрос пароля sudo — в терминале, где запущен studio. Готовый райс по имени сохраняется копией в `~/.config/plove/rices/`. Если в райсе уже были карточки из настроек eww, studio начнёт с них.

Что есть в каталоге:

| Вид | Что показывает | Параметры |
| --- | --- | --- |
| `clock` | крупное время и дата | `seconds`, `date` |
| `worldclock` | время в нескольких городах | `zones`: `Europe/London, Asia/Tokyo` |
| `calendar` | месяц с отмеченным сегодняшним днём | |
| `system` | кольца процессора, памяти и диска | |
| `player` | обложка, название, исполнитель и кнопки | |
| `weather` | погода сейчас с wttr.in, без ключей | `city` (пусто — по IP) |
| `countdown` | сколько дней до даты | `title`, `date`: `2027-01-01` |
| `note` | свой текст | `text` |
| `widget` | свой виджет из `widgets` | `id` |
| `dotclock` | крупные цифры из точек, как на табло (только Quickshell) | `seconds`, `date` |
| `metric` | одно число крупно и полоса под ним (только Quickshell) | `source`: `cpu`, `ram`, `disk`, `temp` |
| `sparkline` | загрузка за две минуты линией (только Quickshell) | `source`: `cpu`, `ram` |

В файле райса раскладка выглядит так, её можно править и руками:

```yaml
desktop:
  - { kind: clock, at: [5, 8], with: { seconds: true } }
  - { kind: weather, at: [78, 6], scale: 1.2, with: { city: Москва } }
  - { kind: player, at: [5, 72] }
```

`at` — левый верхний угол карточки в процентах ширины и высоты экрана, поэтому раскладка одинаково ложится на любой монитор. `scale` — во сколько раз больше обычного размера.

Рисует карточки панель Quickshell (`widgets: qs-desktop`, так в готовых райсах): на нижнем слое, над обоями и под окнами, с плавным появлением; раскладку plove пишет в `~/.config/quickshell/plove/desk.json`, и панель подхватывает её сразу. Прежний вариант — `widgets: eww`: каждая карточка — отдельное окно eww (новые виды `dotclock`, `metric`, `sparkline` он не рисует); пока в райсе есть `desktop`, настройки eww `cards` и `anchor` не действуют.

У каждой карточки своё оформление — в studio это раздел «Оформление», в файле `look`. Чего нет, то как у остальных карточек по стилю райса:

```yaml
  - kind: weather
    at: [3, 6]
    look: { font: Iosevka Nerd Font, accent: cyan, text: fg, fill: accent, opacity: 0.35, radius: 40, border: 2 }
```

| Поле | Что меняет |
| --- | --- |
| `font` | шрифт всей карточки; значки по-прежнему из Nerd Font райса |
| `accent` | крупные цифры, кольца, выделенный день, рамка |
| `text` | обычный текст; подписи — он же, бледнее |
| `fill` | фон карточки, `none` — без фона |
| `opacity` | непрозрачность фона, 0…1 |
| `radius` | скругление углов в px; больше половины высоты — «таблетка» |
| `border` | рамка цвета `accent`, px |

Цвета — имена из палитры райса (`accent`, `fg`, `bg`, `muted`, `surface`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`) или `#rrggbb`.

Когда на столе ничего не выбрано, справа в studio — шрифты системы: шрифт интерфейса (панель, уведомления, меню), моноширинный (терминал и значки, только Nerd Font) и размер. В списке шрифты, которые plove ставит сам из репозиториев Arch (Inter, Roboto, Ubuntu, Fira Sans, IBM Plex Sans, Montserrat, CaskaydiaCove, VictorMono, GeistMono и другие), и всё, что уже установлено. Новый шрифт поставится при «Применить».

## Панель на Quickshell

Панель рисует [Quickshell](https://quickshell.org) (пакет из extra, plove поставит его сам): плавные анимации, свои формы, шрифты по ролям. Это выбор по умолчанию в готовых райсах и в райсах из своей картинки; Waybar остаётся запасным вариантом. В мастере это строка «Программа панели», в файле райса:

```yaml
components:
  bar: quickshell        # или waybar — прежняя панель
overrides:
  quickshell:
    style: island        # segments | rail | island | dock
    font_display: Doto   # шрифт часов
    font_text: Space Grotesk
    font_mono: Space Mono
```

| Вид | Как выглядит |
| --- | --- |
| `segments` | три плавающие плашки: слева, по центру, справа |
| `rail` | тонкая вертикальная полоса у края (`rail_side: left` или `right`), часы в два ряда |
| `island` | один островок по центру сверху; под мышью у часов выезжает дата |
| `dock` | док снизу по центру; значки под мышью увеличиваются |

Модули: `workspaces` (точки; активный стол — черточка цвета акцента, при переключении она плавно переезжает), `clock`, `tray`, `volume` (колесо — громкость, средняя кнопка — без звука, правая — pavucontrol), `network`, `battery`, `calculator` — в `modules_left`, `modules_center`, `modules_right`.

У панели свои выпадающие меню (eww-popups ей не нужен): щелчок по звуку — громкость выхода и микрофона и выбор устройства (ползунок идёт за пальцем, не дожидаясь звукового сервера), по сети — подключение, адрес и сети Wi-Fi рядом (с NetworkManager), по часам — календарь (стрелки и колесо листают месяцы), по калькулятору — строка для qalc: `3200 * 15%`, `10 usd to eur`; Enter считает, второй Enter копирует ответ. Меню открываются и командой, например для своей клавиши в `user.lua`: `qs -c plove ipc call plove toggle calculator` (`volume`, `network`, `calendar`, `calculator`).

Уведомления тоже может показывать сама панель: `notifications: qs-notifications` (так в готовых райсах). Это свой сервер уведомлений вместо mako или swaync: новые всплывают карточками в углу (с картинкой и кнопками действий; важные не исчезают сами), все копятся в боковой панели. Боковая панель открывается колокольчиком (модуль `bell`, на нём число новых), клавишей `Super+N` или `qs -c plove ipc call plove panel`: часы, плитки «Сеть», «Bluetooth» (если есть адаптер), «Не беспокоить», «Микрофон», ползунки громкости и яркости (если у экрана есть подсветка), плеер (любой с MPRIS) и список уведомлений с кнопкой «Очистить». Закрывается щелчком мимо или `Esc`.

Когда меняется громкость (клавишей, колесом, чем угодно) или яркость, внизу экрана на полторы секунды появляется полоса с уровнем (OSD); мышь через неё проходит насквозь.

Компоненты панели — свои QML-файлы plove, они копируются в `~/.config/quickshell/plove/` как есть. Из райса генерируются только `Theme.qml` (цвета, шрифты, форма) и `settings.json` (вид и модули). Панель сама следит за этими файлами: после `plove apply` новый вид появляется сразу, без перезапуска.

Шрифты Google Fonts (Doto, Space Grotesk, Space Mono, Manrope, Outfit, Sora, Poppins, Rubik, Orbitron) plove скачивает в `~/.local/share/fonts/plove/`, остальные шрифты ставит пакетами. Их можно указать и в `fonts` райса, и в `look.font` карточек.

## Тонкая настройка программ

У программ есть описанные настройки: что делает каждая, какие значения можно, что по умолчанию. В мастере (`plove`) это экран «Тонкая настройка»: слева программы райса, справа их настройки по группам — переключатели, списки и ползунки меняются стрелками ←→, под выбранной строкой пояснение. То же списком в терминале:

```sh
plove modules hyprland   # размытие, тени, анимации, раскладка окон, тачпад, мониторы…
plove modules waybar     # положение, часы, рабочие столы, модули
plove modules            # все модули; у кого есть настройки, видно сразу
```

Кроме Hyprland и Waybar настройки есть у терминалов (курсор, отступ), лаунчеров, btop, Neovim (прозрачный фон), GTK (размер курсора, значки), скриншотов и записи экрана (папка, звук), истории буфера обмена, обоев swaybg (как вписать картинку), меню выхода и других.

В файле райса это `overrides`, plove проверяет значения и границы до применения:

```yaml
overrides:
  hyprland: { blur_size: 10, dim_inactive: true, window_animation: slide }
  waybar: { clock_seconds: true, workspace_count: 0 }
```

### Цвета терминала

У kitty, alacritty и ghostty цвета можно отвязать от райса: `colors` — `rice` (как у райса) или другая схема (любой готовый райс, `dracula`, `rose-pine`, `everforest`, `kanagawa`, `one-dark`, `solarized-dark`, `catppuccin-latte`), `background` — фон схемы, `darker`, `lighter` или `rice` (фон райса под цветами схемы).

```yaml
overrides:
  kitty: { colors: dracula, background: darker }
```

### Фон панели

У Waybar настройка `background`: `theme` (цвет темы), `tint` (с оттенком акцента), `gradient`, `wallpaper` (обои райса), `color` и `image`. Прозрачность — из стиля `glass`.

```yaml
overrides:
  waybar:
    background: gradient
    background_color: "#5e2a84"     # пусто — цвета из палитры
    background_color2: "#1d5c63"
    # background: image
    # background_image: ~/Pictures/bar.png
```

### Выпадающие меню

Компонент `popups: eww-popups` добавляет меню, которые открываются по щелчку на панели прямо под модулем: звук (громкость, микрофон, выбор выхода, микшер), сеть (адрес, Wi-Fi через NetworkManager), часы (календарь). Меню в цветах и форме райса. Модуль панели `calculator` открывает калькулятор в лаунчере: набрали выражение, Enter — ответ первой строкой, выбрали её — ответ скопирован (считает `qalc`, без него только арифметика). Средний щелчок по звуку — выключить звук, правый — микшер. Нужен eww из AUR, его поставит paru.

## Что входит в готовые райсы

Райсы отличаются не только цветами, но и набором программ:

| | tokyo-night | catppuccin-mocha | gruvbox | nord |
| --- | --- | --- | --- | --- |
| Терминал | kitty | Ghostty | Alacritty | kitty |
| Лаунчер | fuzzel | rofi | fuzzel | rofi |
| Уведомления | mako | SwayNC | mako | SwayNC |
| Обои | awww (бывший swww, с анимацией) | awww | swaybg | swaybg |
| Меню выхода | wlogout | wlogout | своё (в лаунчере) | своё |
| Часы на рабочем столе | eww | eww | | |

Во всех: Hyprland с тенями, размытием и анимациями, Waybar, zsh с подсказками и подсветкой, fastfetch, btop, yazi, Thunar, imv, Neovim с цветовой схемой райса, тёмная GTK-тема с иконками Papirus и окно пароля hyprpolkitagent. Любую программу можно поменять в мастере (`plove`) на экране настроек стрелками ←→.

| Клавиши | Что делает | Программа |
| --- | --- | --- |
| Super+Enter | терминал | kitty, Ghostty или Alacritty |
| Super+D | запуск приложений | fuzzel или rofi |
| Super+Tab | список окон | rofi |
| Super+E | файловый менеджер | Thunar |
| Super+Shift+F | файловый менеджер в терминале | yazi |
| Super+N | центр уведомлений | SwayNC |
| Ctrl+Shift+Esc | диспетчер задач | btop |
| Super+L | заблокировать экран | hyprlock |
| Super+Shift+E | меню: заблокировать, выйти, перезагрузить, выключить | wlogout или лаунчер |
| Print / Shift+Print | скриншот области / всего экрана (в буфер и в `~/Pictures/Screenshots`) | grim, slurp |
| Super+Shift+V | история буфера обмена | cliphist |
| Super+. | эмодзи и спецсимволы (в буфер) | rofimoji |
| Super+Shift+C | пипетка: цвет под курсором в буфер | hyprpicker |
| Super+Shift+R | запись области экрана, повторно — стоп (`~/Videos`) | wf-recorder |
| Super+Shift+стрелки | перенести окно, в том числе на соседний монитор | |
| Super+Ctrl+стрелки | перенести рабочий стол на соседний монитор | |
| Громкость, яркость, ⏯ | мультимедийные клавиши | wpctl, brightnessctl, playerctl |
| Super+Q / V / F | закрыть окно / плавающее / на весь экран | |
| Super+1…9 | рабочие столы | |

Через 5 минут простоя hypridle блокирует экран, через 10 гасит монитор. С 21:00 до 7:00 hyprsunset делает экран теплее. Qt-программы (Telegram и др.) получают цвета райса через qt6ct, курсор везде одинаковый. Картинки по умолчанию открываются в imv, папки в Thunar.

Несколько мониторов включаются сами. Если нужно точное расположение (в том числе один над другим) или поворот, опишите их в райсе:

```yaml
overrides:
  hyprland:
    scale: 1.25
    monitors:
      - { name: eDP-1, mode: 1920x1080@60 }
      - { name: DP-1, position: above }               # над ноутбуком
      - { name: HDMI-A-1, position: left, rotate: 90 } # вертикальный слева
```

Имена мониторов показывает `hyprctl monitors`. Можно писать и строки в формате Hyprland: `"DP-1, 2560x1440@144, 0x0, 1"`.

Если программу убрать из райса (например, виджеты), plove остановит её при следующем применении.

Свои файлы plove не перезаписывает: в `~/.zshrc` и `~/.config/nvim/init.lua` она только дописывает одну строку, подключающую свой конфиг. Если две программы мешают друг другу (два демона уведомлений, swaybg и swww, PulseAudio и PipeWire), plove останавливает лишнюю или предупреждает в плане. Пакеты из AUR (wlogout, eww) ставятся через paru; если его нет, plove поставит его сама.

`plove doctor --fix` ставит экран входа greetd, чтобы после перезагрузки сразу открывался вход в Hyprland, а не консоль.

## Как это работает

```text
rice.yaml ──► spec    разбор и проверка райса
          ──► plan    подбор модулей, рендер конфигов, сравнение с диском
          ──► apply   пакеты, копия старых файлов, запись, перезагрузка программ
          ──► state   история применений и откат
```

- **Райс — это данные.** Всё описано в RiceSpec (например, `presets/tokyo-night.yaml`): дизайн-токены (цвета, шрифты, отступы) отдельно от компонентов (какие программы).
- **Готовые райсы** лежат в `presets/` и вшиты в бинарник. Обои для них plove рисует сам из палитры (`wallpaper: { generate: hills | waves | rings }`), поэтому никаких проблем с лицензиями на картинки.
- **Превью** (`plove preview`) — макет из слоёв: обои, виджеты eww на рабочем столе, окна терминала с тенью и размытием и панель (Waybar или Quickshell в любом из четырёх видов) со значками и разделителями — в цветах, геометрии и стиле райса, с учётом тонкой настройки (тени, размытие, модули панели, карточки eww). Это не скриншот: шрифт в превью один, содержимое окон выдумано.
- **Модуль = папка** в `modules/`: `module.yaml` плюс шаблоны конфигов на [Tera](https://keats.github.io/tera/) (синтаксис как у Jinja2). Новая программа добавляется новой папкой, ядро не меняется.
- **Сначала план, потом действие.** `plan` ничего не меняет. `apply` показывает план и спрашивает подтверждение.
- **Безопасный откат.** Перед записью plove сохраняет старые файлы в `~/.local/state/plove/generations/`. `plove rollback` возвращает их.
- **Ручные правки не теряются.** Сгенерированные файлы перезаписываются, а свои настройки пишите в `~/.config/hypr/user.lua` (Hyprland 0.57+; на старых версиях — `user.conf`) и `~/.config/kitty/user.conf`: plove создаёт их пустыми и никогда не трогает.
- **Hyprland 0.57 и новее** читает только `~/.config/hypr/hyprland.lua`, старый `hyprland.conf` он не понимает. plove смотрит `Hyprland --version` и пишет нужный файл, а ненужный удаляет; команды `hyprctl dispatch` в меню выхода, hypridle и панели Quickshell тоже в синтаксисе версии (`hyprctl dispatch 'hl.dsp.exit()'`). Формат можно задать и вручную: `PLOVE_HYPRLAND_CONFIG=lua` или `conf`. Если Hyprland ещё не стоит, plove пишет Lua: pacman поставит версию из репозитория. В песочнице (`--root`) по умолчанию старый формат. Проверить конфиг без запуска: `Hyprland --verify-config -c ~/.config/hypr/hyprland.lua`.

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
| `crates/plove-core/src/presets.rs` | встроенные райсы |
| `crates/plove-core/src/wallpaper.rs` | обои, нарисованные из палитры |
| `crates/plove-core/src/palette.rs` | палитра из своих обоев (`plove new`) |
| `crates/plove-preview/src/lib.rs` | превью райса |
| `crates/plove-core/src/check.rs` | проверка системы (`plove doctor`) |
| `crates/plove-tui/src/lib.rs` | мастер в терминале |
| `crates/plove-cli/src/main.rs` | командная строка |
| `crates/plove-core/src/desk.rs` | карточки рабочего стола: каталог и раскладка |
| `crates/plove-cli/src/studio.rs`, `studio.html` | `plove studio`: редактор карточек в браузере |
| `modules/quickshell/` | панель на Quickshell: QML, Theme.qml и settings.json из райса |
| `modules/` | модули программ |
| `presets/` | готовые райсы |
| `docs/previews/` | картинки для README (`plove preview`) |

Тесты: `cargo test`.

## Лицензия

На выбор: [MIT](LICENSE) или [Apache-2.0](LICENSE-APACHE).

//! Проверка системы перед установкой райса (`plove doctor`).
//!
//! Первый запуск на живом Arch показал, что спотыкаются не о конфиги,
//! а о настройку самой системы: запуск от root, вход через `su`, нет
//! доступа к seat (экрану, клавиатуре и мыши). Здесь собраны все такие
//! проверки, и для каждой проблемы, которую можно починить
//! автоматически, есть список команд.
//!
//! Все вопросы к системе идут через трейт `Probe` (как интерфейс в C#).
//! В тестах вместо настоящей системы подставляется выдуманная.

use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{bail, Context, Result};

/// Насколько всё плохо.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Ok,
    /// Работать можно, но что-то может пойти не так.
    Warn,
    /// Дальше идти нельзя.
    Fail,
}

/// Результат одной проверки.
#[derive(Debug)]
pub struct Check {
    pub title: &'static str,
    pub level: Level,
    /// Что не так и что делать, человеческими словами.
    pub detail: String,
    /// Команды, которые чинят проблему. Пусто — автоматически не чинится.
    pub fix: Vec<Vec<String>>,
    /// После исправления нужно выйти из системы и войти заново.
    pub needs_relogin: bool,
}

impl Check {
    fn ok(title: &'static str, detail: impl Into<String>) -> Check {
        Check {
            title,
            level: Level::Ok,
            detail: detail.into(),
            fix: Vec::new(),
            needs_relogin: false,
        }
    }

    fn problem(title: &'static str, level: Level, detail: impl Into<String>) -> Check {
        Check {
            level,
            ..Check::ok(title, detail)
        }
    }
}

/// Вопросы к системе. Настоящие ответы даёт `LiveProbe`.
pub trait Probe {
    fn is_root(&self) -> bool;
    fn user(&self) -> String;
    fn groups(&self) -> Vec<String>;
    fn command_exists(&self, name: &str) -> bool;
    fn service_active(&self, name: &str) -> bool;
    /// Включена ли служба на автозапуск (`systemctl enable`).
    fn service_enabled(&self, name: &str) -> bool;
    fn path_exists(&self, path: &str) -> bool;
    fn online(&self) -> bool;
    /// paru не только есть, но и запускается.
    fn paru_works(&self) -> bool {
        self.command_exists("paru")
    }
    /// Папка сессии (`$XDG_RUNTIME_DIR`): насколько занята и сколько весят
    /// логи Hyprland в ней. `None` — сессии нет (например, вход по ssh).
    fn runtime_dir(&self) -> Option<RuntimeDir> {
        None
    }
}

/// Занятость папки сессии.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeDir {
    pub path: String,
    /// Сколько процентов занято.
    pub used_percent: u64,
    /// Логи Hyprland: путь и размер в байтах.
    pub hyprland_logs: Vec<(String, u64)>,
}

/// Выполнить все проверки по порядку.
pub fn run_all(probe: &dyn Probe) -> Vec<Check> {
    vec![
        user(probe),
        pacman(probe),
        sudo(probe),
        network(probe),
        gpu(probe),
        seat(probe),
        login(probe),
        paru(probe),
        other_desktop(probe),
        runtime_dir(probe),
    ]
}

/// Есть ли проблемы, с которыми дальше идти нельзя.
pub fn has_failures(checks: &[Check]) -> bool {
    checks.iter().any(|c| c.level == Level::Fail)
}

fn user(probe: &dyn Probe) -> Check {
    if probe.is_root() {
        return Check::problem(
            "Пользователь",
            Level::Fail,
            "plove запущен от root. Создайте обычного пользователя с sudo \
             (useradd -m -G wheel <имя>, passwd <имя>, раскомментируйте %wheel в visudo) \
             и войдите под ним прямо в TTY, а не через su",
        );
    }
    Check::ok("Пользователь", probe.user())
}

fn pacman(probe: &dyn Probe) -> Check {
    if probe.command_exists("pacman") {
        Check::ok("pacman", "есть")
    } else {
        Check::problem(
            "pacman",
            Level::Fail,
            "pacman не найден: plove работает только на Arch Linux",
        )
    }
}

fn sudo(probe: &dyn Probe) -> Check {
    if !probe.command_exists("sudo") {
        return Check::problem(
            "sudo",
            Level::Fail,
            "sudo не установлен. Под root выполните pacman -S sudo и разрешите группу wheel в visudo",
        );
    }
    if !probe.groups().iter().any(|g| g == "wheel") {
        return Check::problem(
            "sudo",
            Level::Warn,
            "пользователь не в группе wheel, sudo может не пустить. \
             Под root: usermod -aG wheel <имя> и раскомментируйте %wheel в visudo",
        );
    }
    Check::ok("sudo", "пользователь в группе wheel")
}

fn network(probe: &dyn Probe) -> Check {
    if probe.online() {
        Check::ok("Сеть", "archlinux.org доступен")
    } else {
        Check::problem(
            "Сеть",
            Level::Fail,
            "нет связи с archlinux.org: без сети пакеты не поставить",
        )
    }
}

fn gpu(probe: &dyn Probe) -> Check {
    if !probe.path_exists("/dev/dri/card0") && !probe.path_exists("/dev/dri/card1") {
        return Check::problem(
            "Видеокарта",
            Level::Fail,
            "нет /dev/dri/card*: Wayland не на чем запускать. \
             В виртуалке включите 3D-ускорение",
        );
    }
    if probe.path_exists("/sys/module/nvidia") {
        return Check::problem(
            "Видеокарта",
            Level::Warn,
            "драйвер NVIDIA: Hyprland с ним работает, но может понадобиться ручная настройка \
             (см. wiki.hypr.land, раздел Nvidia)",
        );
    }
    Check::ok("Видеокарта", "есть")
}

/// Seat — это доступ к экрану, клавиатуре и мыши. Без него ни Hyprland,
/// ни Sway не стартуют (ошибка «No backend was able to open a seat»).
/// plove всегда использует seatd: он работает одинаково везде.
fn seat(probe: &dyn Probe) -> Check {
    let active = probe.service_active("seatd");
    let in_group = probe.groups().iter().any(|g| g == "seat");
    if active && in_group {
        return Check::ok(
            "Доступ к экрану (seat)",
            "seatd работает, пользователь в группе seat",
        );
    }
    let mut fix = Vec::new();
    if !probe.command_exists("seatd") {
        fix.push(cmd(&[
            "sudo",
            "pacman",
            "-S",
            "--needed",
            "--noconfirm",
            "seatd",
        ]));
    }
    if !active {
        fix.push(cmd(&["sudo", "systemctl", "enable", "--now", "seatd"]));
    }
    if !in_group {
        fix.push(cmd(&["sudo", "usermod", "-aG", "seat", &probe.user()]));
    }
    let detail = match (active, in_group) {
        (false, false) => "seatd не запущен и пользователь не в группе seat",
        (false, true) => "seatd не запущен",
        _ => "пользователь не в группе seat",
    };
    Check {
        fix,
        needs_relogin: !in_group,
        ..Check::problem(
            "Доступ к экрану (seat)",
            Level::Warn,
            format!("{detail}: Hyprland не сможет запуститься"),
        )
    }
}

/// Экран входа. Без него после перезагрузки открывается консоль,
/// и Hyprland приходится запускать руками.
fn login(probe: &dyn Probe) -> Check {
    let managers = ["greetd", "sddm", "gdm", "lightdm", "ly"];
    if let Some(found) = managers.iter().find(|m| probe.service_enabled(m)) {
        return Check::ok("Экран входа", format!("{found} включён"));
    }
    // greetd с tuigreet: маленький экран входа прямо в консоли,
    // после ввода пароля сразу запускает Hyprland.
    let config = "[terminal]\\nvt = 1\\n\\n[default_session]\\n\
                  command = \"tuigreet --time --remember --cmd start-hyprland\"\\n\
                  user = \"greeter\"\\n";
    Check {
        fix: vec![
            cmd(&[
                "sudo",
                "pacman",
                "-S",
                "--needed",
                "--noconfirm",
                "greetd",
                "greetd-tuigreet",
            ]),
            cmd(&[
                "sudo",
                "sh",
                "-c",
                &format!("printf '{config}' > /etc/greetd/config.toml"),
            ]),
            cmd(&["sudo", "systemctl", "enable", "greetd"]),
        ],
        ..Check::problem(
            "Экран входа",
            Level::Warn,
            "не настроен: после перезагрузки откроется консоль. \
             Исправление ставит greetd, он заработает после перезагрузки",
        )
    }
}

/// paru нужен для пакетов из AUR (например, wlogout). Без него райсы
/// из официальных репозиториев работают, поэтому это лишь предупреждение.
fn paru(probe: &dyn Probe) -> Check {
    if probe.paru_works() {
        return Check::ok("paru (AUR)", "есть");
    }
    if probe.command_exists("paru") {
        return Check {
            fix: crate::system::paru_install_commands(),
            ..Check::problem(
                "paru (AUR)",
                Level::Warn,
                "установлен, но не запускается: скорее всего, собран под старую версию \
                 pacman (libalpm). Исправление пересоберёт его из исходников",
            )
        };
    }
    Check {
        fix: crate::system::paru_install_commands(),
        ..Check::problem(
            "paru (AUR)",
            Level::Warn,
            "не установлен: пакеты из AUR поставить не получится. \
             plove поставит его сам при первом таком пакете или по --fix",
        )
    }
}

fn other_desktop(probe: &dyn Probe) -> Check {
    let desktops = [
        ("gnome-shell", "GNOME"),
        ("plasmashell", "KDE Plasma"),
        ("xfce4-session", "Xfce"),
        ("cinnamon", "Cinnamon"),
    ];
    let found: Vec<&str> = desktops
        .iter()
        .filter(|(binary, _)| probe.command_exists(binary))
        .map(|(_, name)| *name)
        .collect();
    if found.is_empty() {
        Check::ok("Другое окружение", "не найдено")
    } else {
        Check::problem(
            "Другое окружение",
            Level::Warn,
            format!(
                "установлено {}. plove рассчитан на чистую систему: \
                 конфликтов файлов не будет, но вход по умолчанию может открывать старое окружение",
                found.join(", ")
            ),
        )
    }
}

/// Папка сессии в памяти (/run/user/…) маленькая. В виртуалке VMware
/// Hyprland пишет ошибку рендера на каждый кадр, и за полчаса его лог
/// занимает её целиком. После этого программы не могут создать общую
/// память для картинки окна и падают (rofi — с SIGBUS).
fn runtime_dir(probe: &dyn Probe) -> Check {
    const TITLE: &str = "Папка сессии";
    let Some(dir) = probe.runtime_dir() else {
        return Check::ok(TITLE, "сессии нет, не проверялось");
    };
    let logs: u64 = dir.hyprland_logs.iter().map(|(_, size)| size).sum();
    if dir.used_percent < 80 {
        return Check::ok(
            TITLE,
            format!("{} занята на {}%", dir.path, dir.used_percent),
        );
    }
    let mut detail = format!(
        "{} занята на {}%: программы, которым не хватит места, падают (rofi — с SIGBUS)",
        dir.path, dir.used_percent
    );
    let mut fix = Vec::new();
    if logs >= 50 * 1024 * 1024 {
        detail += &format!(
            ". Почти всё — лог Hyprland, {} МБ; его можно очистить, Hyprland продолжит писать в пустой файл",
            logs / 1024 / 1024
        );
        // Очистить, а не удалить: Hyprland держит файл открытым, и место
        // от удалённого файла не освободилось бы до выхода из сессии.
        fix = dir
            .hyprland_logs
            .iter()
            .map(|(path, _)| cmd(&["truncate", "-s", "0", path]))
            .collect();
    }
    Check {
        fix,
        ..Check::problem(TITLE, Level::Warn, detail)
    }
}

fn cmd(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

/// Выполнить команды исправления. Вывод и вопрос пароля sudo
/// идут прямо в терминал пользователя.
pub fn apply_fix(check: &Check) -> Result<()> {
    for command in &check.fix {
        let (program, args) = command.split_first().context("пустая команда")?;
        let status = Command::new(program)
            .args(args)
            .status()
            .with_context(|| format!("не удалось запустить {}", command.join(" ")))?;
        if !status.success() {
            bail!("команда «{}» завершилась с ошибкой", command.join(" "));
        }
    }
    Ok(())
}

/// Ответы настоящей системы.
pub struct LiveProbe;

impl Probe for LiveProbe {
    fn is_root(&self) -> bool {
        crate::system::is_root()
    }

    fn user(&self) -> String {
        std::env::var("USER").unwrap_or_else(|_| output("id", &["-un"]))
    }

    fn groups(&self) -> Vec<String> {
        // `id -nG` печатает группы через пробел. Это группы из базы
        // пользователей, а не текущего процесса: после usermod
        // проверка сразу видит новую группу, хоть войти заново всё равно нужно.
        let user = self.user();
        output("id", &["-nG", &user])
            .split_whitespace()
            .map(String::from)
            .collect()
    }

    fn command_exists(&self, name: &str) -> bool {
        crate::system::command_exists(name)
    }

    fn service_active(&self, name: &str) -> bool {
        Command::new("systemctl")
            .args(["is-active", "--quiet", name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }

    fn service_enabled(&self, name: &str) -> bool {
        Command::new("systemctl")
            .args(["is-enabled", "--quiet", name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }

    fn path_exists(&self, path: &str) -> bool {
        Path::new(path).exists()
    }

    fn paru_works(&self) -> bool {
        crate::system::paru_works()
    }

    fn runtime_dir(&self) -> Option<RuntimeDir> {
        let path = std::env::var("XDG_RUNTIME_DIR").ok()?;
        let used_percent = output("df", &["--output=pcent", &path])
            .lines()
            .last()?
            .trim()
            .trim_end_matches('%')
            .parse()
            .ok()?;
        let mut hyprland_logs: Vec<(String, u64)> =
            std::fs::read_dir(Path::new(&path).join("hypr"))
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| entry.path().join("hyprland.log"))
                .filter_map(|log| {
                    let size = log.metadata().ok()?.len();
                    Some((log.display().to_string(), size))
                })
                .collect();
        // Лог уже удалили, а Hyprland держит его открытым: место занято,
        // пока файл не очистить через /proc/<pid>/fd/<n>.
        for pid in output("pgrep", &["-x", "Hyprland"]).split_whitespace() {
            let fds = std::fs::read_dir(format!("/proc/{pid}/fd"));
            for fd in fds.into_iter().flatten().flatten() {
                let Ok(target) = std::fs::read_link(fd.path()) else {
                    continue;
                };
                let target = target.display().to_string();
                if target.ends_with("hyprland.log (deleted)") {
                    let size = fd.path().metadata().map_or(0, |m| m.len());
                    hyprland_logs.push((fd.path().display().to_string(), size));
                }
            }
        }
        Some(RuntimeDir {
            path,
            used_percent,
            hyprland_logs,
        })
    }

    fn online(&self) -> bool {
        // Достаточно открыть TCP-соединение: DNS и маршрут работают.
        let Ok(mut addresses) = ("archlinux.org", 443).to_socket_addrs() else {
            return false;
        };
        addresses.any(|a| TcpStream::connect_timeout(&a, Duration::from_secs(3)).is_ok())
    }
}

/// Вывод команды одной строкой, пустая строка при ошибке.
fn output(program: &str, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Выдуманная система для тестов: по умолчанию всё в порядке.
    struct Fake {
        root: bool,
        groups: Vec<&'static str>,
        seatd_active: bool,
        commands: Vec<&'static str>,
        greetd: bool,
    }

    impl Default for Fake {
        fn default() -> Self {
            Fake {
                root: false,
                groups: vec!["wheel", "seat"],
                seatd_active: true,
                commands: vec!["pacman", "sudo", "seatd", "paru"],
                greetd: true,
            }
        }
    }

    impl Probe for Fake {
        fn is_root(&self) -> bool {
            self.root
        }
        fn user(&self) -> String {
            "val".into()
        }
        fn groups(&self) -> Vec<String> {
            self.groups.iter().map(|g| g.to_string()).collect()
        }
        fn command_exists(&self, name: &str) -> bool {
            self.commands.contains(&name)
        }
        fn service_active(&self, name: &str) -> bool {
            name == "seatd" && self.seatd_active
        }
        fn service_enabled(&self, name: &str) -> bool {
            name == "greetd" && self.greetd
        }
        fn path_exists(&self, path: &str) -> bool {
            path == "/dev/dri/card0"
        }
        fn online(&self) -> bool {
            true
        }
    }

    #[test]
    fn healthy_system_passes() {
        let checks = run_all(&Fake::default());
        assert!(checks.iter().all(|c| c.level == Level::Ok), "{checks:#?}");
    }

    #[test]
    fn root_fails() {
        let fake = Fake {
            root: true,
            ..Fake::default()
        };
        assert!(has_failures(&run_all(&fake)));
    }

    /// И это тоже было в VM: после перезагрузки консоль вместо Hyprland.
    #[test]
    fn missing_login_screen_gets_greetd() {
        let fake = Fake {
            greetd: false,
            ..Fake::default()
        };
        let checks = run_all(&fake);
        let login = checks.iter().find(|c| c.title == "Экран входа").unwrap();
        assert_eq!(login.level, Level::Warn);
        assert_eq!(login.fix.len(), 3);
        assert!(login.fix[1][3].contains("start-hyprland"));
    }

    /// Тоже из VM: лог Hyprland занял всю папку сессии, rofi падал.
    #[test]
    fn full_runtime_dir_gets_log_truncated() {
        struct Full;
        impl Probe for Full {
            fn is_root(&self) -> bool {
                false
            }
            fn user(&self) -> String {
                "val".into()
            }
            fn groups(&self) -> Vec<String> {
                vec![]
            }
            fn command_exists(&self, _: &str) -> bool {
                true
            }
            fn service_active(&self, _: &str) -> bool {
                true
            }
            fn service_enabled(&self, _: &str) -> bool {
                true
            }
            fn path_exists(&self, _: &str) -> bool {
                true
            }
            fn online(&self) -> bool {
                true
            }
            fn runtime_dir(&self) -> Option<RuntimeDir> {
                Some(RuntimeDir {
                    path: "/run/user/1000".into(),
                    used_percent: 100,
                    hyprland_logs: vec![("/run/user/1000/hypr/x/hyprland.log".into(), 800 << 20)],
                })
            }
        }
        let check = runtime_dir(&Full);
        assert_eq!(check.level, Level::Warn);
        assert!(check.detail.contains("800 МБ"), "{}", check.detail);
        assert_eq!(
            check.fix,
            vec![cmd(&[
                "truncate",
                "-s",
                "0",
                "/run/user/1000/hypr/x/hyprland.log"
            ])]
        );
    }

    /// Ровно та ситуация, что была в VM: seatd установлен, но выключен,
    /// пользователь не в группе seat.
    #[test]
    fn missing_seat_gets_a_fix() {
        let fake = Fake {
            groups: vec!["wheel"],
            seatd_active: false,
            ..Fake::default()
        };
        let checks = run_all(&fake);
        let seat = checks.iter().find(|c| c.title.contains("seat")).unwrap();
        assert_eq!(seat.level, Level::Warn);
        assert!(seat.needs_relogin);
        assert_eq!(
            seat.fix,
            vec![
                cmd(&["sudo", "systemctl", "enable", "--now", "seatd"]),
                cmd(&["sudo", "usermod", "-aG", "seat", "val"]),
            ]
        );
    }
}

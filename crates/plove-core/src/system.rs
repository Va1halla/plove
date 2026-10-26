//! Всё, что касается живой системы: пути, pacman, запуск команд.
//!
//! Собрано в одном месте, чтобы остальной код можно было
//! тестировать без настоящего Arch.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};

/// Окружение, в котором работает plove.
pub struct Env {
    /// Домашняя папка: сюда пишутся конфиги и история.
    pub home: PathBuf,
    /// Режим песочницы: `home` — это тестовая папка,
    /// пакеты не ставятся, программы не перезагружаются.
    pub sandbox: bool,
}

impl Env {
    /// Настоящая система текущего пользователя.
    /// От root не работаем: Hyprland не рассчитан на запуск от root,
    /// а конфиги оказались бы в /root, а не у пользователя.
    pub fn real() -> Result<Env> {
        if is_root() {
            bail!(
                "plove нельзя запускать от root. Создайте обычного пользователя с sudo:\n\
                 \n  useradd -m -G wheel <имя>\n  passwd <имя>\n  EDITOR=nano visudo   \
                 # раскомментируйте строку %wheel ALL=(ALL:ALL) ALL\n\n\
                 затем войдите под ним и запустите plove снова"
            );
        }
        let home = std::env::var_os("HOME").context("переменная HOME не задана")?;
        Ok(Env {
            home: PathBuf::from(home),
            sandbox: false,
        })
    }

    /// Песочница: всё пишется в `root`, систему не трогаем.
    pub fn sandbox(root: PathBuf) -> Env {
        Env {
            home: root,
            sandbox: true,
        }
    }

    /// `~/.config/x` → `/home/user/.config/x`
    pub fn expand(&self, path: &str) -> PathBuf {
        match path.strip_prefix("~/") {
            Some(rest) => self.home.join(rest),
            None => PathBuf::from(path),
        }
    }

    /// Обратно: показать путь пользователю коротко, через `~`.
    pub fn display(&self, path: &Path) -> String {
        match path.strip_prefix(&self.home) {
            Ok(rest) => format!("~/{}", rest.display()),
            Err(_) => path.display().to_string(),
        }
    }

    /// Где plove хранит свою историю применений.
    pub fn state_dir(&self) -> PathBuf {
        self.home.join(".local/state/plove")
    }
}

/// Запущены ли мы от root. Владелец папки `/proc/self` — это пользователь
/// текущего процесса, так что обходимся без системных вызовов.
pub fn is_root() -> bool {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata("/proc/self").is_ok_and(|m| m.uid() == 0)
}

/// Запущенная графическая сессия Hyprland, к которой можно обращаться.
///
/// Если plove запущен в терминале внутри Hyprland, нужные переменные уже
/// есть в окружении. Если же из соседней TTY (Ctrl+Alt+F3), их нет, но
/// сессию можно найти по сокетам в `$XDG_RUNTIME_DIR`: тогда передаём
/// переменные командам перезагрузки сами.
pub struct Session {
    vars: Vec<(String, String)>,
}

/// Найти графическую сессию. `None` — Hyprland не запущен, перезагружать
/// программы бессмысленно: настройки подхватятся при его старте.
pub fn graphical_session() -> Option<Session> {
    if std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
    {
        return Some(Session { vars: Vec::new() });
    }
    let runtime = runtime_dir()?;
    let signature = newest_hyprland_instance(&runtime.join("hypr"))?;
    let display = wayland_socket(&runtime)?;
    Some(Session {
        vars: vec![
            ("HYPRLAND_INSTANCE_SIGNATURE".into(), signature),
            ("WAYLAND_DISPLAY".into(), display),
            ("XDG_RUNTIME_DIR".into(), runtime.display().to_string()),
        ],
    })
}

/// `$XDG_RUNTIME_DIR`, а если он не задан, `/run/user/<uid>`.
fn runtime_dir() -> Option<PathBuf> {
    use std::os::unix::fs::MetadataExt;
    let dir = match std::env::var_os("XDG_RUNTIME_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => {
            let uid = std::fs::metadata("/proc/self").ok()?.uid();
            PathBuf::from(format!("/run/user/{uid}"))
        }
    };
    dir.is_dir().then_some(dir)
}

/// Каждый запуск Hyprland создаёт папку `hypr/<подпись>/` с сокетом.
/// После падений там остаются старые папки, поэтому берём самую свежую.
fn newest_hyprland_instance(hypr: &Path) -> Option<String> {
    std::fs::read_dir(hypr)
        .ok()?
        .flatten()
        .filter(|entry| entry.path().join(".socket.sock").exists())
        .max_by_key(|entry| entry.metadata().and_then(|m| m.modified()).ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
}

/// Сокет Wayland: `wayland-1` и т. п. (рядом лежит `wayland-1.lock`, его пропускаем).
fn wayland_socket(runtime: &Path) -> Option<String> {
    let mut names: Vec<String> = std::fs::read_dir(runtime)
        .ok()?
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("wayland-") && !name.ends_with(".lock"))
        .collect();
    names.sort();
    names.into_iter().next()
}

/// Название виртуальной машины, в которой мы работаем (`vmware`, `oracle`,
/// `kvm`…), или пустая строка на настоящем железе.
pub fn virtual_machine() -> String {
    Command::new("systemd-detect-virt")
        .arg("--vm")
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// Установлен ли пакет. `None` — не удалось узнать (нет pacman, это не Arch).
pub fn package_installed(name: &str) -> Option<bool> {
    // Command — это Process.Start из C# или subprocess.run из Python.
    Command::new("pacman")
        .args(["-Q", name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()
        .map(|status| status.success())
}

/// Поставить пакеты через `sudo pacman`. Вывод pacman и вопрос пароля
/// идут прямо в терминал пользователя.
pub fn install_packages(packages: &[&str]) -> Result<()> {
    let status = Command::new("sudo")
        .args(["pacman", "-S", "--needed"])
        .args(packages)
        .status()
        .context("не удалось запустить sudo pacman")?;
    if !status.success() {
        bail!("pacman завершился с ошибкой, пакеты не установлены");
    }
    Ok(())
}

/// Поставить пакеты из AUR через paru. paru сам спросит пароль sudo.
pub fn install_aur(packages: &[&str]) -> Result<()> {
    let status = Command::new("paru")
        .args(["-S", "--needed"])
        .args(packages)
        .status()
        .context("не удалось запустить paru")?;
    if !status.success() {
        bail!("paru завершился с ошибкой, пакеты из AUR не установлены");
    }
    Ok(())
}

/// Команды установки paru: он сам живёт в AUR, поэтому собираем его
/// из готового бинарного пакета paru-bin через makepkg.
pub fn paru_install_commands() -> Vec<Vec<String>> {
    let build = "rm -rf /tmp/plove-paru && \
                 git clone https://aur.archlinux.org/paru-bin.git /tmp/plove-paru && \
                 cd /tmp/plove-paru && makepkg -si --noconfirm";
    [
        vec![
            "sudo",
            "pacman",
            "-S",
            "--needed",
            "--noconfirm",
            "base-devel",
            "git",
        ],
        vec!["sh", "-c", build],
    ]
    .iter()
    .map(|c| c.iter().map(|s| s.to_string()).collect())
    .collect()
}

/// Поставить paru.
pub fn install_paru() -> Result<()> {
    for command in paru_install_commands() {
        let status = Command::new(&command[0])
            .args(&command[1..])
            .status()
            .with_context(|| format!("не удалось запустить {}", command[0]))?;
        if !status.success() {
            bail!(
                "не удалось поставить paru (команда «{}»)",
                command.join(" ")
            );
        }
    }
    Ok(())
}

/// Выполнить команду через sh без вывода. `false`, если не вышло.
pub fn run_command(command: &str) -> bool {
    Command::new("sh")
        .args(["-c", command])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Есть ли программа в PATH.
pub fn command_exists(name: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {name}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Выполнить команду перезагрузки программы. `false`, если не вышло:
/// например, программа сейчас не запущена. Это не ошибка.
pub fn run_reload(command: &str, session: &Session) -> bool {
    Command::new("sh")
        .args(["-c", command])
        .envs(session.vars.iter().map(|(k, v)| (k, v)))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Так выглядит `$XDG_RUNTIME_DIR` с запущенным Hyprland и папкой
    /// от прошлого, упавшего запуска (без сокета).
    #[test]
    fn finds_hyprland_from_another_tty() {
        let runtime = tempfile::tempdir().unwrap();
        let dir = runtime.path();
        std::fs::create_dir_all(dir.join("hypr/old_crashed")).unwrap();
        std::fs::create_dir_all(dir.join("hypr/live")).unwrap();
        std::fs::write(dir.join("hypr/live/.socket.sock"), "").unwrap();
        std::fs::write(dir.join("wayland-1"), "").unwrap();
        std::fs::write(dir.join("wayland-1.lock"), "").unwrap();

        assert_eq!(
            newest_hyprland_instance(&dir.join("hypr")).as_deref(),
            Some("live")
        );
        assert_eq!(wayland_socket(dir).as_deref(), Some("wayland-1"));
    }
}

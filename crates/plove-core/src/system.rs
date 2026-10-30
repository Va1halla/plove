//! Всё, что касается живой системы: пути, pacman, запуск команд.
//!
//! Собрано в одном месте, чтобы остальной код можно было
//! тестировать без настоящего Arch.

use std::fs;
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
    /// Hyprland 0.57+: конфиг на Lua (hyprland.lua), старый hyprland.conf
    /// он не читает. См. [`hyprland_uses_lua`].
    pub hyprland_lua: bool,
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
            hyprland_lua: hyprland_uses_lua(),
        })
    }

    /// Песочница: всё пишется в `root`, систему не трогаем.
    pub fn sandbox(root: PathBuf) -> Env {
        // В песочнице результат не зависит от машины: старый формат,
        // если не попросили другой через PLOVE_HYPRLAND_CONFIG.
        Env {
            home: root,
            sandbox: true,
            hyprland_lua: config_override().unwrap_or(false),
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

/// Установлен ли пакет именно с этим именем. `pacman -Q` находит и пакет,
/// который его заменил (`pacman -Q p7zip` отвечает «7zip …»), а для
/// конфликтов важно точное имя.
pub fn package_installed_exact(name: &str) -> bool {
    Command::new("pacman")
        .args(["-Q", name])
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .is_some_and(|o| String::from_utf8_lossy(&o.stdout).split_whitespace().next() == Some(name))
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

/// Семейства шрифтов, которые видит fontconfig, по алфавиту.
/// Пусто, если fc-list нет.
pub fn font_families() -> Vec<String> {
    let Ok(out) = Command::new("fc-list").args([":", "family"]).output() else {
        return Vec::new();
    };
    let mut families: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        // «Inter,Inter Light»: первое имя — само семейство.
        .filter_map(|line| line.split(',').next())
        .map(|f| f.trim().to_string())
        .filter(|f| !f.is_empty())
        .collect();
    families.sort();
    families.dedup();
    families
}

/// `PLOVE_HYPRLAND_CONFIG=lua` или `conf`: формат конфига Hyprland
/// без определения версии.
fn config_override() -> Option<bool> {
    match std::env::var("PLOVE_HYPRLAND_CONFIG").ok()?.as_str() {
        "lua" => Some(true),
        "conf" => Some(false),
        _ => None,
    }
}

/// Hyprland 0.57 и новее читает только hyprland.lua. Версию берём из
/// `Hyprland --version`. Если Hyprland ещё не стоит, его поставит pacman,
/// а в репозитории уже версия с Lua.
pub fn hyprland_uses_lua() -> bool {
    if let Some(lua) = config_override() {
        return lua;
    }
    let Ok(out) = Command::new("Hyprland").arg("--version").output() else {
        return true;
    };
    hyprland_version(&String::from_utf8_lossy(&out.stdout)).is_none_or(|v| v >= (0, 57))
}

/// «Hyprland 0.57.0 built from branch…» → (0, 57).
fn hyprland_version(text: &str) -> Option<(u32, u32)> {
    let version = text
        .split_whitespace()
        .skip_while(|w| *w != "Hyprland")
        .nth(1)?;
    let mut parts = version.split('.').map(|p| p.parse::<u32>());
    Some((parts.next()?.ok()?, parts.next()?.ok()?))
}

/// Скачать файлы шрифта в `dir` и обновить кэш fontconfig.
pub fn download_font(urls: &[String], dir: &Path) -> Result<()> {
    let parent = dir.parent().context("у папки шрифта нет родителя")?;
    fs::create_dir_all(parent)?;
    let partial = dir.with_extension("part");
    let _ = fs::remove_dir_all(&partial);
    fs::create_dir_all(&partial)?;
    for url in urls {
        let name = url.rsplit('/').next().unwrap_or("font.ttf");
        let ok = Command::new("curl")
            // --globoff: в именах вроде Doto[ROND,wght].ttf скобки — не шаблон curl.
            .args(["-fsSL", "--globoff", "--max-time", "60", "-o"])
            .arg(partial.join(name))
            .arg(url)
            .status()
            .context("нужен curl")?
            .success();
        if !ok {
            let _ = fs::remove_dir_all(&partial);
            bail!("curl не смог скачать {url}");
        }
    }
    fs::rename(&partial, dir)?;
    let _ = Command::new("fc-cache")
        .arg("-f")
        .arg(parent)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    Ok(())
}

/// Поставить пакеты через `sudo pacman`. Вывод pacman и вопрос пароля
/// идут прямо в терминал пользователя. `no_confirm` — без вопроса
/// «Приступить к установке?»: иначе `plove apply -y` ждал бы ответа.
pub fn install_packages(packages: &[&str], no_confirm: bool) -> Result<()> {
    let status = Command::new("sudo")
        .args(["pacman", "-S", "--needed"])
        .args(no_confirm.then_some("--noconfirm"))
        .args(packages)
        .status()
        .context("не удалось запустить sudo pacman")?;
    if !status.success() {
        bail!("pacman завершился с ошибкой, пакеты не установлены");
    }
    Ok(())
}

/// Поставить пакеты из AUR через paru. paru сам спросит пароль sudo.
pub fn install_aur(packages: &[&str], no_confirm: bool) -> Result<()> {
    let status = Command::new("paru")
        .args(["-S", "--needed"])
        .args(no_confirm.then_some("--noconfirm"))
        .args(packages)
        .status()
        .context("не удалось запустить paru")?;
    if !status.success() {
        bail!("paru завершился с ошибкой, пакеты из AUR не установлены");
    }
    Ok(())
}

/// paru установлен и запускается. Готовая сборка paru-bin ломается после
/// обновления pacman (libalpm.so.15 → .16): команда есть, но не работает.
pub fn paru_works() -> bool {
    Command::new("paru")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Команды установки paru: он сам живёт в AUR, поэтому собираем его
/// через makepkg из исходников, под ту libalpm, что стоит в системе.
/// Готовый paru-bin не годится: он собран под старую libalpm и после
/// обновления pacman падает с «libalpm.so.15: cannot open shared object».
/// Сборка занимает несколько минут; cargo makepkg поставит сам (-s).
/// Если релиз paru ещё не поддерживает новую libalpm, пробуем paru-git.
/// Вместе с paru-bin убираем и paru-bin-debug: его файлы мешают
/// поставить отладочный пакет нового paru.
pub fn paru_install_commands() -> Vec<Vec<String>> {
    let build = "old=$(pacman -Qq paru-bin paru-bin-debug 2>/dev/null); \
                 if [ -n \"$old\" ]; then sudo pacman -Rdd --noconfirm $old; fi; \
                 for pkg in paru paru-git; do \
                     rm -rf /tmp/plove-paru && \
                     git clone https://aur.archlinux.org/$pkg.git /tmp/plove-paru && \
                     (cd /tmp/plove-paru && makepkg -si --noconfirm) && \
                     paru --version >/dev/null && exit 0; \
                 done; exit 1";
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
    fn reads_hyprland_version() {
        let text = "Hyprland 0.57.0 built from branch v0.57.0 at commit 0aa6745 clean";
        assert_eq!(hyprland_version(text), Some((0, 57)));
        assert_eq!(hyprland_version("Hyprland 0.56.2 built"), Some((0, 56)));
        assert_eq!(hyprland_version("command not found"), None);
    }

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

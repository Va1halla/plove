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
    pub fn real() -> Result<Env> {
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

/// Выполнить команду перезагрузки программы. `false`, если не вышло:
/// например, программа сейчас не запущена. Это не ошибка.
pub fn run_reload(command: &str) -> bool {
    Command::new("sh")
        .args(["-c", command])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

// Панель plove на Quickshell: по одной на каждый монитор.
// Сгенерированы только Theme.qml и settings.json, остальное — как есть.
import QtQuick
import Quickshell
import Quickshell.Io

ShellRoot {
    // `qs -c plove ipc call plove reload`: plove зовёт после записи всех
    // файлов (сама Quickshell перезагружается уже на первом из них).
    IpcHandler {
        target: "plove"
        function reload(): void { Quickshell.reload(true); }
        // Открыть или закрыть меню панели: volume, network, calendar, calculator.
        function toggle(name: string): void { Config.toggle(name); }
        // Боковая панель: уведомления, быстрые настройки, плеер.
        function panel(): void { Config.panel = !Config.panel; }
        // Клавиши яркости: перечитать яркость и показать OSD.
        function brightness(): void { Brightness.refresh(); }
        // Лаунчер: программы и калькулятор.
        function launcher(): void { launcher.open ? launcher.close("") : launcher.show(); }
        // dmenu: строки из файла, выбор — в <файл>.out (qs-dmenu.sh).
        function dmenu(path: string, prompt: string): void { launcher.dmenu(path, prompt); }
        // Заблокировать экран (модуль qs-lock) и посмотреть экран блокировки без блокировки.
        function lock(): void { lock.locked = true; }
        function lockPreview(): void { lock.preview = !lock.preview; }
        // Открыть или закрыть выезжающую панель конструктора по имени.
        function drawer(name: string): void { Config.toggleDrawer(name); }
    }

    Launcher { id: launcher }
    Lock { id: lock }
    Toasts {}
    Osd {}
    SidePanel {}

    // Обычная панель — если в райсе нет своей оболочки (раздел shell).
    Variants {
        model: Config.s.shell.length > 0 ? [] : Quickshell.screens
        Bar {}
    }

    // Своя оболочка: каждая поверхность на каждом мониторе.
    Variants {
        model: Quickshell.screens
        Scope {
            id: perScreen
            required property var modelData
            Variants {
                model: Config.s.shell
                Surface { screenData: perScreen.modelData }
            }
        }
    }

    // Карточки рабочего стола (модуль qs-desktop).
    Variants {
        model: Quickshell.screens
        DeskWindow {}
    }
}

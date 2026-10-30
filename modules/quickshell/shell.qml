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
    }

    Toasts {}
    Osd {}
    SidePanel {}

    Variants {
        model: Quickshell.screens
        Bar {}
    }
}

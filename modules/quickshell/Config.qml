// Настройки панели из райса. Файл settings.json пишет `plove apply`,
// а панель перечитывает его на лету: новый стиль или набор модулей
// появляется без перезапуска Quickshell.
pragma Singleton

import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland

Singleton {
    id: root

    readonly property string style: adapter.style
    readonly property bool vertical: adapter.style === "rail"
    readonly property alias s: adapter

    // Открытое выпадающее меню: volume, network, calendar, calculator или "".
    property string popup: ""
    // Боковая панель открыта.
    property bool panel: false
    function toggle(name) {
        if (popup === name) {
            popup = "";
        } else if (popup !== "") {
            // Wayland не даёт открыть меню, пока открыто другое с захватом
            // фокуса: сначала закрываем старое, новое — чуть позже.
            popup = "";
            next.name = name;
            next.restart();
        } else {
            popup = name;
        }
    }

    Timer {
        id: next
        property string name: ""
        interval: 60
        onTriggered: root.popup = name
    }

    // Команда в терминале райса (у терминала свой префикс: kitty -e …).
    function terminal(command) {
        const shell = "sh -c " + JSON.stringify(command);
        return adapter.terminal ? "setsid -f " + adapter.terminal + " " + shell : shell;
    }

    // Перейти на рабочий стол: номер или «e+1». Hyprland 0.57+ принимает
    // в dispatch только Lua, старые версии — прежний синтаксис.
    function workspace(target) {
        if (adapter.lua)
            Hyprland.dispatch("hl.dsp.focus({ workspace = " + JSON.stringify(target) + " })");
        else
            Hyprland.dispatch("workspace " + target);
    }

    function run(command) {
        Quickshell.execDetached(["sh", "-c", command]);
    }

    FileView {
        path: Quickshell.shellPath("settings.json")
        watchChanges: true
        blockLoading: true
        onFileChanged: reload()

        JsonAdapter {
            id: adapter
            property string style: "segments"
            property string side: "left"
            property var left: ["workspaces"]
            property var center: ["clock"]
            property var right: ["tray", "volume", "network", "battery"]
            property int workspaces: 5
            property string time: "HH:mm"
            property bool date: false
            property string terminal: ""
            property bool notifications: false
            property bool desktop: false
            property bool lua: false
        }
    }
}

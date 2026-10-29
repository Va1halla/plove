// Настройки панели из райса. Файл settings.json пишет `plove apply`,
// а панель перечитывает его на лету: новый стиль или набор модулей
// появляется без перезапуска Quickshell.
pragma Singleton

import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root

    readonly property string style: adapter.style
    readonly property bool vertical: adapter.style === "rail"
    readonly property alias s: adapter

    // Выпадающие меню panel → eww-popups (звук, сеть, календарь), если они в райсе.
    function popup(menu, group) {
        if (!adapter.popups)
            return;
        const anchor = adapter.anchors[group] || "top right";
        Quickshell.execDetached(["sh", Quickshell.env("HOME") + "/.local/share/plove/popups.sh", "toggle", menu, anchor, adapter.offset]);
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
            property bool popups: false
            property var anchors: ({})
            property string offset: "8x8"
        }
    }
}

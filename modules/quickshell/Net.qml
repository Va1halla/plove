// Сеть для панели: раз в 5 секунд спрашивает qs-net.sh.
pragma Singleton

import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root

    property string kind: "off"     // wifi, ethernet, off
    property string name: ""
    property int strength: 0

    Process {
        id: probe
        command: ["sh", Quickshell.env("HOME") + "/.local/share/plove/qs-net.sh"]
        running: true
        stdout: StdioCollector {
            onStreamFinished: {
                const parts = text.trim().split("\t");
                root.kind = parts[0] || "off";
                root.name = parts[1] || "";
                root.strength = parseInt(parts[2] || "0");
            }
        }
    }

    Timer {
        interval: 5000
        running: true
        repeat: true
        onTriggered: probe.running = true
    }
}

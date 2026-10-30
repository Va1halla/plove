// Яркость экрана через brightnessctl. На компьютере без подсветки
// (виртуалка, монитор по HDMI) available = false и ползунка нет.
pragma Singleton

import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root

    property bool available: false
    property real value: 0          // 0…1
    // Яркость поменялась (клавишей или ползунком): для OSD.
    signal changed()

    // Клавиши яркости зовут через IPC (refresh): sysfs о переменах не сообщает.
    function refresh() { probe.running = true; }

    function set(v) {
        value = Math.max(0.01, Math.min(1, v));
        changed();
        Quickshell.execDetached(["brightnessctl", "-c", "backlight", "set", Math.round(value * 100) + "%"]);
    }

    Process {
        id: probe
        command: ["sh", "-c", "ls /sys/class/backlight | grep -q . && brightnessctl -c backlight -m info"]
        running: true
        stdout: StdioCollector {
            onStreamFinished: {
                // backlight,acpi_video0,…,45%,…
                const f = text.trim().split(",");
                root.available = f.length > 3;
                if (!root.available)
                    return;
                const v = parseInt(f[3]) / 100;
                if (Math.abs(v - root.value) > 0.001) {
                    root.value = v;
                    root.changed();
                }
            }
        }
    }

    Timer {
        interval: 5000
        running: root.available
        repeat: true
        onTriggered: probe.running = true
    }
}

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

    function set(v) {
        value = Math.max(0.01, Math.min(1, v));
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
                if (root.available)
                    root.value = parseInt(f[3]) / 100;
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

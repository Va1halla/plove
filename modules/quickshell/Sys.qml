// Загрузка системы для карточек: процессор, память, диск, температура
// и история процессора и памяти за две минуты (для графиков).
pragma Singleton

import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root

    property real cpu: 0            // 0…1
    property real ram: 0
    property real disk: 0
    property real temp: -1          // °C, -1 — датчика нет
    property string ramText: ""
    property string diskText: ""
    property var cpuHistory: []
    property var ramHistory: []
    readonly property int samples: 60

    property var last: null         // [занято, всего] из /proc/stat

    function push(list, v) {
        const out = list.concat([v]);
        return out.length > samples ? out.slice(out.length - samples) : out;
    }
    function gib(kb) { return (kb / 1048576).toFixed(1); }

    FileView { id: stat; path: "/proc/stat" }
    FileView { id: mem; path: "/proc/meminfo" }
    // Датчика может не быть (виртуалка): тогда путь пустой и FileView молчит.
    property string thermalPath: ""
    FileView { id: thermal; path: root.thermalPath }
    Process {
        command: ["sh", "-c", "ls /sys/class/thermal/thermal_zone*/temp 2>/dev/null | head -1"]
        running: true
        stdout: StdioCollector { onStreamFinished: root.thermalPath = text.trim() }
    }

    Timer {
        interval: 2000
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: {
            stat.reload();
            mem.reload();
            if (root.thermalPath)
                thermal.reload();
            const f = stat.text().split("\n")[0].trim().split(/\s+/).slice(1).map(Number);
            const idle = f[3] + (f[4] || 0);
            const total = f.reduce((a, b) => a + b, 0);
            if (root.last) {
                const dt = total - root.last[1];
                root.cpu = dt > 0 ? 1 - (idle - root.last[0]) / dt : 0;
            }
            root.last = [idle, total];
            const info = {};
            for (const line of mem.text().split("\n")) {
                const m = line.match(/^(\w+):\s+(\d+)/);
                if (m) info[m[1]] = Number(m[2]);
            }
            if (info.MemTotal) {
                const used = info.MemTotal - info.MemAvailable;
                root.ram = used / info.MemTotal;
                root.ramText = gib(used) + " / " + gib(info.MemTotal) + " ГиБ";
            }
            const t = root.thermalPath ? parseInt(thermal.text()) : NaN;
            root.temp = isNaN(t) ? -1 : t / 1000;
            root.cpuHistory = push(root.cpuHistory, root.cpu);
            root.ramHistory = push(root.ramHistory, root.ram);
        }
    }

    Process {
        id: df
        command: ["df", "-B1", "--output=used,size", "/"]
        running: true
        stdout: StdioCollector {
            onStreamFinished: {
                const f = text.trim().split("\n")[1].trim().split(/\s+/).map(Number);
                root.disk = f[0] / f[1];
                root.diskText = (f[0] / 1073741824).toFixed(0) + " / " + (f[1] / 1073741824).toFixed(0) + " ГиБ";
            }
        }
    }
    Timer { interval: 60000; running: true; repeat: true; onTriggered: df.running = true }
}

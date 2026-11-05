// Блок «Вывод команды»: последняя строка вывода, раз в every секунд.
import QtQuick
import Quickshell.Io

Chip {
    id: chip
    property var opts: ({})
    property string value: ""
    shown: value !== ""

    Process {
        id: run
        command: ["sh", "-c", chip.opts.command || "true"]
        running: (chip.opts.command || "") !== ""
        stdout: StdioCollector {
            onStreamFinished: {
                const lines = text.trim().split("\n");
                chip.value = lines[lines.length - 1];
            }
        }
    }
    Timer {
        interval: Math.max(1, chip.opts.every || 30) * 1000
        running: (chip.opts.command || "") !== ""
        repeat: true
        onTriggered: run.running = true
    }
    Label {
        vertical: chip.vertical
        icon: chip.opts.icon || ""
        value: chip.value
        turned: true
    }
}

// Свой виджет райса (раздел widgets): вывод команды раз в every секунд,
// значок и подпись с {value}, цвет из палитры. Щелчок — команда click.
import QtQuick
import Quickshell.Io

Chip {
    id: chip

    property string widgetId: ""
    readonly property var widget: Config.s.widgets[widgetId] || null
    property string value: ""

    shown: widget !== null && value !== ""
    onClicked: if (widget && widget.click) Config.run(widget.click)

    Process {
        id: run
        command: ["sh", "-c", chip.widget ? chip.widget.command : "true"]
        running: chip.widget !== null
        stdout: StdioCollector { onStreamFinished: chip.value = text.trim().split("\n")[0] }
    }
    Timer {
        interval: chip.widget ? Math.max(1, chip.widget.every) * 1000 : 60000
        running: chip.widget !== null
        repeat: true
        onTriggered: run.running = true
    }

    Row {
        spacing: Math.round(Theme.pad / 2)
        Text {
            visible: chip.widget && chip.widget.icon
            anchors.verticalCenter: parent.verticalCenter
            text: chip.widget ? chip.widget.icon : ""
            color: chip.widget ? chip.widget.color : Theme.accent
            font.family: Theme.fontIcons
            font.pixelSize: Math.round(Theme.fontSize * 1.15)
        }
        Text {
            visible: !chip.vertical
            anchors.verticalCenter: parent.verticalCenter
            text: {
                if (!chip.widget) return "";
                const t = chip.widget.text || "{value}";
                return t.indexOf("{value}") >= 0 ? t.replace("{value}", chip.value) : t + " " + chip.value;
            }
            color: chip.widget ? chip.widget.color : Theme.fg
            font.family: Theme.fontMono
            font.pixelSize: Theme.fontSize
        }
    }
}

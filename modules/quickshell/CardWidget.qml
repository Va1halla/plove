// Карточка «Свой виджет»: вывод команды из widgets райса.
import QtQuick
import Quickshell.Io

Text {
    id: root
    property var card
    property string value: ""
    readonly property var widget: card ? Desk.widgets[card.opts.id] || null : null
    anchors.centerIn: parent
    text: widget ? (widget.icon ? widget.icon + " " : "") + (widget.text.indexOf("{value}") >= 0 ? widget.text.replace("{value}", value) : widget.text + " " + value) : ""
    color: widget ? widget.color : Theme.fg
    font.family: card ? card.font : Theme.fontText
    font.pixelSize: card ? card.px(18) : 18

    Process {
        id: run
        command: ["sh", "-c", root.widget ? root.widget.command : "true"]
        running: root.widget !== null
        stdout: StdioCollector { onStreamFinished: root.value = text.trim() }
    }
    Timer { interval: root.widget ? root.widget.every * 1000 : 60000; running: root.widget !== null; repeat: true; onTriggered: run.running = true }
}

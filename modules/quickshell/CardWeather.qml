// Карточка «Погода»: wttr.in раз в 15 минут (qs-weather.sh).
import QtQuick
import Quickshell
import Quickshell.Io

Row {
    id: root
    property var card
    property var now: ({ icon: "󰖐", temp: "…", desc: "" })
    anchors.verticalCenter: parent.verticalCenter
    spacing: card ? card.px(16) : 16

    Process {
        id: fetch
        command: ["sh", Quickshell.env("HOME") + "/.local/share/plove/qs-weather.sh", root.card ? (root.card.opts.city || "") : ""]
        running: root.card !== undefined
        stdout: StdioCollector {
            onStreamFinished: {
                try { root.now = JSON.parse(text); } catch (e) {}
            }
        }
    }
    Timer { interval: 900000; running: true; repeat: true; onTriggered: fetch.running = true }

    Text {
        anchors.verticalCenter: parent.verticalCenter
        text: root.now.icon
        color: root.card ? root.card.accent : Theme.accent
        font.family: Theme.fontIcons
        font.pixelSize: root.card ? root.card.px(48) : 48
    }
    Column {
        anchors.verticalCenter: parent.verticalCenter
        Text {
            text: root.now.temp
            color: root.card ? root.card.accent : Theme.accent
            font.family: root.card ? root.card.display : Theme.fontDisplay
            font.pixelSize: root.card ? root.card.px(30) : 30
            font.bold: true
        }
        Text {
            width: root.card ? root.card.width - root.card.px(48 + 16 + Theme.pad * 4) : 180
            text: root.now.desc
            elide: Text.ElideRight
            color: root.card ? root.card.muted : Theme.muted
            font.family: root.card ? root.card.font : Theme.fontText
            font.pixelSize: root.card ? root.card.px(14) : 14
        }
    }
}

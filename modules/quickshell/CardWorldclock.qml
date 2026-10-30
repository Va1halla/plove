// Карточка «Мировое время»: города и время в них.
import QtQuick
import Quickshell
import Quickshell.Io

Column {
    id: root
    property var card
    property var times: []
    readonly property var zones: card ? card.data_.zones || [] : []
    anchors.verticalCenter: parent.verticalCenter
    width: parent.width
    spacing: card ? card.px(6) : 6

    Process {
        id: date
        // Пояса проверены plove: только буквы, цифры и _/+-.
        command: ["sh", "-c", root.zones.map(z => "TZ='" + z + "' date +%H:%M").join("; ")]
        running: root.zones.length > 0
        stdout: StdioCollector { onStreamFinished: root.times = text.trim().split("\n") }
    }
    Timer { interval: 10000; running: true; repeat: true; onTriggered: date.running = true }

    Repeater {
        model: root.zones
        Item {
            required property string modelData
            required property int index
            width: root.width
            height: zone.height
            Text {
                id: zone
                text: modelData.split("/").pop().replace(/_/g, " ")
                color: root.card ? root.card.muted : Theme.muted
                font.family: root.card ? root.card.font : Theme.fontText
                font.pixelSize: root.card ? root.card.px(17) : 17
            }
            Text {
                anchors.right: parent.right
                anchors.baseline: zone.baseline
                text: root.times[index] || "—"
                color: root.card ? root.card.accent : Theme.accent
                font.family: root.card ? root.card.mono : Theme.fontMono
                font.pixelSize: root.card ? root.card.px(22) : 22
                font.bold: true
            }
        }
    }
}

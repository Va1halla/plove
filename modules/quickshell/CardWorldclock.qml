// Карточка «Мировое время»: город капителями, время крупно справа и
// разница с местным временем.
import QtQuick
import Quickshell
import Quickshell.Io

Column {
    id: root
    property var card
    property var times: []          // ["HH:MM", "+0300"]
    readonly property var zones: card ? card.data_.zones || [] : []
    anchors.verticalCenter: parent.verticalCenter
    width: parent.width
    spacing: card ? card.px(card.style === "classic" ? 6 : 8) : 6

    Process {
        id: date
        command: ["sh", "-c", "date +%z; " + root.zones.map(z => "TZ='" + z + "' date '+%H:%M %z'").join("; ")]
        running: root.zones.length > 0
        stdout: StdioCollector { onStreamFinished: root.times = text.trim().split("\n") }
    }
    Timer { interval: 10000; running: true; repeat: true; onTriggered: date.running = true }

    function offset(z) {
        const minutes = s => (s[0] === "-" ? -1 : 1) * (parseInt(s.slice(1, 3)) * 60 + parseInt(s.slice(3, 5)));
        if (!root.times[0] || !z) return "";
        const d = (minutes(z) - minutes(root.times[0])) / 60;
        return d === 0 ? "здесь" : (d > 0 ? "+" : "") + d + " ч";
    }

    Repeater {
        model: root.zones
        Item {
            required property string modelData
            required property int index
            readonly property var parts: (root.times[index + 1] || "— ").split(" ")
            width: root.width
            height: time.height
            Column {
                anchors.verticalCenter: parent.verticalCenter
                Text {
                    text: modelData.split("/").pop().replace(/_/g, " ")
                    color: root.card ? (root.card.style === "classic" ? root.card.muted : root.card.ink) : Theme.muted
                    font.family: root.card ? root.card.font : Theme.fontText
                    font.pixelSize: root.card ? root.card.px(16) : 16
                    font.weight: Font.DemiBold
                    font.capitalization: root.card && root.card.style === "dots" ? Font.AllUppercase : Font.MixedCase
                }
                CardLabel {
                    visible: root.card && root.card.style !== "classic"
                    card: root.card
                    text: root.offset(parent.parent.parts[1])
                }
            }
            Text {
                id: time
                anchors { right: parent.right; verticalCenter: parent.verticalCenter }
                text: parent.parts[0]
                color: root.card ? root.card.accent : Theme.accent
                font.family: root.card ? (root.card.style === "soft" ? root.card.display : root.card.mono) : Theme.fontMono
                font.pixelSize: root.card ? root.card.px(root.card.style === "classic" ? 22 : 26) : 22
                font.weight: root.card && root.card.style === "soft" ? Font.Black : Font.Bold
            }
        }
    }
}

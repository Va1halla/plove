// Карточка «Система»: кольца процессора, памяти и диска в стиле карточки.
import QtQuick

Row {
    id: root
    property var card
    anchors.centerIn: parent
    spacing: card ? card.px(card.style === "classic" ? 26 : 22) : 26
    Repeater {
        model: [["cpu", Sys.cpu], ["ram", Sys.ram], ["disk", Sys.disk]]
        Ring {
            required property var modelData
            card: root.card
            label: modelData[0]
            value: modelData[1]
            size: root.card ? root.card.px(root.card.style === "classic" ? 64 : 70) : 64
        }
    }
}

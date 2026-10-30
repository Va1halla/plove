// Карточка «Система»: кольца процессора, памяти и диска.
import QtQuick

Row {
    property var card
    anchors.centerIn: parent
    spacing: card ? card.px(26) : 26
    Repeater {
        model: [["cpu", Sys.cpu], ["ram", Sys.ram], ["disk", Sys.disk]]
        Ring {
            required property var modelData
            label: modelData[0]
            value: modelData[1]
            size: parent.card ? parent.card.px(64) : 64
            accent: parent.card ? parent.card.accent : Theme.accent
            ink: parent.card ? parent.card.ink : Theme.fg
            muted: parent.card ? parent.card.muted : Theme.muted
            font: parent.card ? parent.card.mono : Theme.fontMono
        }
    }
}

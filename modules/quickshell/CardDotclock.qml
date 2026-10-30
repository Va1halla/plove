// Карточка «Точечные часы»: время точками 5×7 и дата мелко.
import QtQuick
import Quickshell

Column {
    property var card
    anchors.centerIn: parent
    spacing: card ? card.px(12) : 12

    SystemClock { id: clock; precision: card && card.opts.seconds ? SystemClock.Seconds : SystemClock.Minutes }

    DotText {
        anchors.horizontalCenter: parent.horizontalCenter
        text: Qt.formatDateTime(clock.date, card && card.opts.seconds ? "HH:mm:ss" : "HH:mm")
        dot: card ? card.px(card.opts.seconds ? 8 : 12) : 12
        on: card ? card.accent : Theme.accent
    }
    Text {
        anchors.horizontalCenter: parent.horizontalCenter
        visible: card && card.opts.date !== false
        text: clock.date.toLocaleDateString(Qt.locale(), "ddd · d MMMM · yyyy").toUpperCase()
        color: card ? card.muted : Theme.muted
        font.family: card ? card.mono : Theme.fontMono
        font.pixelSize: card ? card.px(14) : 14
        font.letterSpacing: card ? card.px(2) : 2
    }
}

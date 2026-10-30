// Карточка «Точечные часы»: время точками 5×7 и дата капителями.
import QtQuick
import Quickshell

Column {
    id: root
    property var card
    anchors.centerIn: parent
    spacing: card ? card.px(14) : 14
    SystemClock { id: clock; precision: root.card && root.card.opts.seconds ? SystemClock.Seconds : SystemClock.Minutes }

    DotText {
        anchors.horizontalCenter: parent.horizontalCenter
        text: Qt.formatDateTime(clock.date, root.card && root.card.opts.seconds ? "HH:mm:ss" : "HH:mm")
        dot: root.card ? root.card.px(root.card.opts.seconds ? 8 : 12) : 12
        on: root.card ? (root.card.style === "soft" ? root.card.ink : root.card.accent) : Theme.accent
        off: Theme.fade(root.card ? root.card.ink : Theme.fg, 0.07)
    }
    CardLabel {
        anchors.horizontalCenter: parent.horizontalCenter
        visible: !root.card || root.card.opts.date !== false
        card: root.card
        text: clock.date.toLocaleDateString(Qt.locale(), "ddd · d MMMM · yyyy")
        font.pixelSize: root.card ? root.card.px(14) : 14
        font.letterSpacing: 2
        font.capitalization: Font.AllUppercase
    }
}

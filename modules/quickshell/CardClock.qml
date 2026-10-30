// Карточка «Часы»: крупное время и дата под ним.
import QtQuick
import Quickshell

Column {
    property var card
    anchors.centerIn: parent

    SystemClock { id: clock; precision: card && card.opts.seconds ? SystemClock.Seconds : SystemClock.Minutes }

    Text {
        anchors.horizontalCenter: parent.horizontalCenter
        text: Qt.formatDateTime(clock.date, card && card.opts.seconds ? "HH:mm:ss" : "HH:mm")
        color: card ? card.accent : Theme.accent
        font.family: card ? card.display : Theme.fontDisplay
        font.pixelSize: card ? card.px(96) : 96
        font.bold: true
        style: Text.Raised
        styleColor: Theme.fade(Theme.bg, 0.5)
    }
    Text {
        anchors.horizontalCenter: parent.horizontalCenter
        visible: card && card.opts.date !== false
        text: {
            const t = clock.date.toLocaleDateString(Qt.locale(), "dddd, d MMMM");
            return t.charAt(0).toUpperCase() + t.slice(1);
        }
        color: card ? card.ink : Theme.fg
        font.family: card ? card.font : Theme.fontText
        font.pixelSize: card ? card.px(20) : 20
        style: Text.Raised
        styleColor: Theme.fade(Theme.bg, 0.7)
    }
}

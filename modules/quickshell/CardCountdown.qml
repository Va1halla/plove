// Карточка «Обратный отсчёт»: сколько дней до даты.
import QtQuick
import Quickshell

Column {
    property var card
    anchors.centerIn: parent
    SystemClock { id: clock; precision: SystemClock.Hours }
    readonly property int days: {
        if (!card) return 0;
        const target = new Date(card.opts.date + "T00:00:00");
        return isNaN(target) ? 0 : Math.max(0, Math.ceil((target - clock.date) / 86400000));
    }
    Text {
        anchors.horizontalCenter: parent.horizontalCenter
        text: parent.days
        color: parent.card ? parent.card.accent : Theme.accent
        font.family: parent.card ? parent.card.display : Theme.fontDisplay
        font.pixelSize: parent.card ? parent.card.px(46) : 46
        font.bold: true
    }
    Text {
        anchors.horizontalCenter: parent.horizontalCenter
        text: "дн. до: " + (parent.card ? parent.card.opts.title : "")
        color: parent.card ? parent.card.muted : Theme.muted
        font.family: parent.card ? parent.card.font : Theme.fontText
        font.pixelSize: parent.card ? parent.card.px(14) : 14
    }
}

// Карточка «Обратный отсчёт»: дни крупно и подпись.
import QtQuick
import Quickshell

Column {
    id: root
    property var card
    anchors.centerIn: parent
    spacing: card ? card.px(6) : 6
    SystemClock { id: clock; precision: SystemClock.Hours }
    readonly property int days: {
        if (!card) return 0;
        const target = new Date(card.opts.date + "T00:00:00");
        return isNaN(target) ? 0 : Math.max(0, Math.ceil((target - clock.date) / 86400000));
    }
    BigValue {
        anchors.horizontalCenter: parent.horizontalCenter
        card: root.card
        value: root.days
        size: 52
        color: root.card ? root.card.accent : Theme.accent
    }
    CardLabel {
        anchors.horizontalCenter: parent.horizontalCenter
        card: root.card
        text: "дн. до: " + (root.card ? root.card.opts.title : "")
    }
}

// Карточка «Часы»: крупное время и вторая строка с днём и датой.
// classic и glass — цифры прямо на обоях с тенью; dots — цифрами из
// точек; soft — крупно и жирно на светлой карточке.
import QtQuick
import QtQuick.Effects
import Quickshell

Column {
    id: root
    property var card
    anchors.centerIn: parent
    spacing: card ? card.px(card.style === "dots" ? 14 : 4) : 4

    SystemClock { id: clock; precision: root.card && root.card.opts.seconds ? SystemClock.Seconds : SystemClock.Minutes }
    readonly property string time: Qt.formatDateTime(clock.date, card && card.opts.seconds ? "HH:mm:ss" : "HH:mm")

    Item {
        anchors.horizontalCenter: parent.horizontalCenter
        width: root.card && root.card.style === "dots" ? dots.width : big.implicitWidth
        height: root.card && root.card.style === "dots" ? dots.height : big.implicitHeight

        Text {
            id: big
            visible: !root.card || root.card.style !== "dots"
            text: root.time
            color: root.card ? (root.card.style === "soft" ? root.card.ink : root.card.accent) : Theme.accent
            font.family: root.card ? root.card.display : Theme.fontDisplay
            font.pixelSize: root.card ? root.card.px(root.card.style === "classic" ? 96 : 104) : 96
            font.weight: root.card && root.card.style === "soft" ? Font.Black : root.card && root.card.style === "glass" ? Font.DemiBold : Font.Bold
            font.letterSpacing: root.card && root.card.style !== "classic" ? -root.card.px(4) : 0
            layer.enabled: root.card !== undefined && root.card.frameless
            layer.effect: MultiEffect {
                shadowEnabled: true
                shadowColor: root.card && root.card.style === "glass" ? Theme.fade(Theme.bg, 0.8) : Theme.fade(Theme.bg, 0.6)
                shadowBlur: 0.8
                shadowVerticalOffset: 4
                shadowHorizontalOffset: 0
            }
        }
        DotText {
            id: dots
            visible: root.card !== undefined && root.card.style === "dots"
            text: root.time
            dot: root.card ? root.card.px(root.card.opts.seconds ? 8 : 11) : 11
            on: root.card ? root.card.accent : Theme.accent
            off: Theme.fade(root.card ? root.card.ink : Theme.fg, 0.07)
        }
    }

    Text {
        anchors.horizontalCenter: parent.horizontalCenter
        visible: !root.card || root.card.opts.date !== false
        text: {
            const t = clock.date.toLocaleDateString(Qt.locale(), root.card && root.card.style === "dots" ? "ddd · d MMMM" : "dddd, d MMMM");
            return t.charAt(0).toUpperCase() + t.slice(1);
        }
        color: root.card ? (root.card.style === "dots" ? root.card.muted : root.card.ink) : Theme.fg
        opacity: root.card && root.card.style === "soft" ? 0.7 : 1
        font.family: root.card ? root.card.font : Theme.fontText
        font.pixelSize: root.card ? root.card.px(root.card.style === "dots" ? 14 : 20) : 20
        font.capitalization: root.card && root.card.style === "dots" ? Font.AllUppercase : Font.MixedCase
        font.letterSpacing: root.card && root.card.style === "dots" ? 2 : 0
        style: root.card && root.card.frameless ? Text.Raised : Text.Normal
        styleColor: Theme.fade(Theme.bg, 0.6)
    }
}

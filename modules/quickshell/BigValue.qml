// Крупное число карточки: плавно перетекает к новому значению.
// В виде dots — цифрами из точек.
import QtQuick

Item {
    id: big

    property var card
    property real value: 0
    property int decimals: 0
    property string prefix: ""
    property string suffix: ""
    property string text: ""            // готовый текст вместо числа
    property real size: 34              // высота цифр при scale 1
    property color color: card ? card.ink : Theme.fg

    property real shown: value
    Behavior on shown { NumberAnimation { duration: 700; easing.type: Easing.OutCubic } }
    readonly property string label: text !== "" ? text : prefix + shown.toFixed(decimals) + suffix
    readonly property bool dotted: card !== undefined && card !== null && card.style === "dots"

    implicitWidth: dotted ? dots.width : plain.implicitWidth
    implicitHeight: dotted ? dots.height : plain.implicitHeight

    Text {
        id: plain
        visible: !big.dotted
        text: big.label
        color: big.color
        font.family: big.card ? big.card.display : Theme.fontDisplay
        font.pixelSize: big.card ? big.card.px(big.size) : big.size
        font.weight: big.card && big.card.style === "soft" ? Font.Black : Font.Bold
        font.letterSpacing: big.card && big.card.style === "glass" ? -1 : 0
    }
    DotText {
        id: dots
        visible: big.dotted
        text: big.label
        dot: big.card ? big.card.px(big.size / 7.5) : 4
        on: big.color
        off: Theme.fade(big.color, 0.07)
    }
}

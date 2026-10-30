// Карточка «Заметка»: свой текст; у dots — моноширинным.
import QtQuick

Text {
    property var card
    anchors.verticalCenter: parent.verticalCenter
    width: parent.width
    text: card ? card.opts.text : ""
    wrapMode: Text.Wrap
    color: card ? card.ink : Theme.fg
    font.family: card ? (card.style === "dots" ? card.mono : card.font) : Theme.fontText
    font.pixelSize: card ? card.px(17) : 17
    font.weight: card && card.style === "soft" ? Font.DemiBold : Font.Normal
    lineHeight: 1.15
}

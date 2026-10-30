// Карточка «Заметка»: свой текст.
import QtQuick

Text {
    property var card
    anchors.verticalCenter: parent.verticalCenter
    width: parent.width
    text: card ? card.opts.text : ""
    wrapMode: Text.Wrap
    color: card ? card.ink : Theme.fg
    font.family: card ? card.font : Theme.fontText
    font.pixelSize: card ? card.px(17) : 17
}

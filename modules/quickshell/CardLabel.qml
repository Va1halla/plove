// Подпись на карточке: мелко, капителями и с разрядкой (кроме classic).
import QtQuick

Text {
    property var card
    color: card ? card.muted : Theme.muted
    font.family: card ? (card.style === "dots" ? card.mono : card.font) : Theme.fontText
    font.pixelSize: card ? card.px(card.style === "dots" ? 11 : 12) : 12
    font.capitalization: card && card.caps ? Font.AllUppercase : Font.MixedCase
    font.letterSpacing: card && card.caps ? (card.style === "dots" ? 2 : 1.2) : 0
    font.weight: card && card.style === "soft" ? Font.DemiBold : Font.Medium
}

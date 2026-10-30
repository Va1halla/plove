// Карточка «Календарь»: сегодня — таблетка цвета акцента.
import QtQuick

CalendarGrid {
    property var card
    anchors.centerIn: parent
    cell: card ? parent.width / 7 : 36
    text: card ? card.px(14) : 14
    accent: card ? card.accent : Theme.accent
    ink: card ? card.ink : Theme.fg
    muted: card ? card.muted : Theme.muted
    font: card ? card.font : Theme.fontText
    mono: card ? card.mono : Theme.fontMono
    caps: card ? card.caps : false
}

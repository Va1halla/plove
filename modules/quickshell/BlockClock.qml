// Блок «Крупные часы» выезжающей панели: время и дата.
import QtQuick
import Quickshell

Column {
    property var opts: ({})
    spacing: 2
    SystemClock { id: clock; precision: opts.seconds ? SystemClock.Seconds : SystemClock.Minutes }
    Text {
        text: Qt.formatTime(clock.date, opts.seconds ? "HH:mm:ss" : "HH:mm")
        color: Theme.fg
        font.family: Theme.fontDisplay || Theme.fontText
        font.pixelSize: Math.round(Theme.fontSize * 3)
        font.weight: Font.Bold
    }
    Text {
        text: clock.date.toLocaleDateString(Qt.locale("ru_RU"), "dddd, d MMMM")
        color: Theme.muted
        font.family: Theme.fontText
        font.pixelSize: Theme.fontSize
    }
}

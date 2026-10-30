// Карточки рабочего стола из desk.json (пишет модуль qs-desktop по
// раскладке из plove studio). Файл перечитывается на лету.
pragma Singleton

import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    readonly property bool enabled: Config.s.desktop
    readonly property var cards: data.cards
    // Вид карточек: glass, dots, soft или classic.
    readonly property string style: data.style
    readonly property var widgets: data.widgets

    FileView {
        path: Quickshell.shellPath("desk.json")
        watchChanges: true
        blockLoading: true
        onFileChanged: reload()
        JsonAdapter {
            id: data
            property string style: "glass"
            property var cards: []
            property var widgets: ({})
        }
    }
}

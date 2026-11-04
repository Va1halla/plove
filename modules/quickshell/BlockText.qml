// Блок «Надпись»: просто текст.
import QtQuick

Chip {
    property var opts: ({})
    Text {
        text: opts.text || ""
        color: Theme.fg
        font.family: Theme.fontText
        font.pixelSize: Theme.fontSize
    }
}

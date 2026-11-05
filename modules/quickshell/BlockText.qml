// Блок «Надпись»: просто текст; на боковой полосе — повёрнут вдоль неё.
import QtQuick

Chip {
    id: chip
    property var opts: ({})
    Item {
        width: chip.vertical ? label.height : label.width
        height: chip.vertical ? label.width : label.height
        Text {
            id: label
            anchors.centerIn: parent
            rotation: chip.vertical ? 90 : 0
            text: chip.opts.text || ""
            color: Theme.fg
            font.family: Theme.fontText
            font.pixelSize: Theme.fontSize
        }
    }
}

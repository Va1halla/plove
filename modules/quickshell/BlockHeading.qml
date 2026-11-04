// Блок «Заголовок» выезжающей панели.
import QtQuick

Text {
    property var opts: ({})
    text: opts.text || ""
    color: Theme.fg
    font.family: Theme.fontText
    font.pixelSize: Math.round(Theme.fontSize * 1.3)
    font.weight: Font.DemiBold
}

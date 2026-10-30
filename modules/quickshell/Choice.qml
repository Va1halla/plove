// Строка списка, которую можно выбрать: устройство, сеть.
import QtQuick

Rectangle {
    id: row

    property string text: ""
    property string detail: ""
    property bool chosen: false
    signal clicked()

    height: Math.round(Theme.fontSize * 2.2)
    radius: Theme.radiusChip > height / 2 ? height / 2 : Theme.radiusChip
    color: chosen ? Theme.fade(Theme.accent, 0.18) : area.containsMouse ? Theme.fade(Theme.fg, 0.08) : "transparent"
    Behavior on color { ColorAnimation { duration: Theme.fast } }

    Text {
        anchors { left: parent.left; leftMargin: Theme.pad; right: info.left; rightMargin: Theme.pad; verticalCenter: parent.verticalCenter }
        text: row.text
        elide: Text.ElideRight
        color: row.chosen ? Theme.accent : Theme.fg
        font.family: Theme.fontText
        font.pixelSize: Theme.fontSize
    }
    Text {
        id: info
        anchors { right: parent.right; rightMargin: Theme.pad; verticalCenter: parent.verticalCenter }
        text: row.detail
        color: Theme.muted
        font.family: Theme.fontMono
        font.pixelSize: Math.round(Theme.fontSize * 0.9)
    }
    MouseArea {
        id: area
        anchors.fill: parent
        hoverEnabled: true
        onClicked: row.clicked()
    }
}

// Плитка быстрых настроек: значок, название, состояние. Включено —
// цвета акцента.
import QtQuick

Rectangle {
    id: tile

    property string icon: ""
    property string title: ""
    property string detail: ""
    property bool active: false
    signal clicked()

    width: (352 - Theme.pad) / 2
    height: Math.round(Theme.fontSize * 3.6)
    radius: Math.min(Theme.radius, height / 2)
    color: active ? Theme.fade(Theme.accent, area.containsMouse ? 0.32 : 0.22) : Theme.fade(Theme.fg, area.containsMouse ? 0.11 : 0.06)
    Behavior on color { ColorAnimation { duration: Theme.fast } }

    Row {
        anchors { left: parent.left; leftMargin: Theme.pad * 1.5; verticalCenter: parent.verticalCenter }
        spacing: Theme.pad
        Text {
            anchors.verticalCenter: parent.verticalCenter
            text: tile.icon
            color: tile.active ? Theme.accent : Theme.fg
            font.family: Theme.fontIcons
            font.pixelSize: Math.round(Theme.fontSize * 1.4)
        }
        Column {
            anchors.verticalCenter: parent.verticalCenter
            Text {
                text: tile.title
                color: Theme.fg
                font.family: Theme.fontText
                font.pixelSize: Theme.fontSize
                font.bold: true
            }
            Text {
                text: tile.detail
                visible: text !== ""
                width: tile.width - Theme.fontSize * 4
                elide: Text.ElideRight
                color: Theme.muted
                font.family: Theme.fontText
                font.pixelSize: Math.round(Theme.fontSize * 0.82)
            }
        }
    }
    MouseArea {
        id: area
        anchors.fill: parent
        hoverEnabled: true
        onClicked: tile.clicked()
    }
}

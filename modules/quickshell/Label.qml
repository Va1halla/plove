// Значок и значение рядом; у rail — значок над небольшой цифрой.
import QtQuick

Item {
    id: label

    property bool vertical: false
    property string icon: ""
    property string value: ""
    property bool dim: false
    property bool alert: false

    readonly property color tint: alert ? Theme.red : dim ? Theme.muted : Theme.fg
    width: vertical ? column.width : row.width
    height: vertical ? column.height : row.height

    Row {
        id: row
        visible: !label.vertical
        spacing: Math.round(Theme.pad / 2)
        Text {
            anchors.verticalCenter: parent.verticalCenter
            text: label.icon
            color: label.alert ? Theme.red : Theme.accent
            font.family: Theme.fontIcons
            font.pixelSize: Math.round(Theme.fontSize * 1.15)
        }
        Text {
            anchors.verticalCenter: parent.verticalCenter
            text: label.value
            color: label.tint
            font.family: Theme.fontMono
            font.pixelSize: Theme.fontSize
            elide: Text.ElideRight
            width: Math.min(implicitWidth, Theme.fontSize * 9)
        }
    }

    Column {
        id: column
        visible: label.vertical
        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: label.icon
            color: label.alert ? Theme.red : Theme.accent
            font.family: Theme.fontIcons
            font.pixelSize: Math.round(Theme.fontSize * 1.15)
        }
    }
}

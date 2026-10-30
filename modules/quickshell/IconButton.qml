// Круглая кнопка со значком: включено — цвета акцента.
import QtQuick

Rectangle {
    id: btn

    property string icon: ""
    property bool active: true
    signal clicked()

    width: Math.round(Theme.fontSize * 2.4)
    height: width
    radius: width / 2
    color: active ? Theme.fade(Theme.accent, area.containsMouse ? 0.35 : 0.22) : Theme.fade(Theme.fg, area.containsMouse ? 0.16 : 0.08)
    Behavior on color { ColorAnimation { duration: Theme.fast } }

    Text {
        anchors.centerIn: parent
        text: btn.icon
        color: btn.active ? Theme.accent : Theme.muted
        font.family: Theme.fontIcons
        font.pixelSize: Math.round(Theme.fontSize * 1.2)
    }
    MouseArea {
        id: area
        anchors.fill: parent
        hoverEnabled: true
        onClicked: btn.clicked()
    }
}

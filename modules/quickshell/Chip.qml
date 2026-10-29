// Основа модуля панели: подсветка и лёгкое увеличение под мышью,
// щелчки и колесо. Содержимое — один дочерний элемент.
import QtQuick

Item {
    id: chip

    property string group: "right"
    property bool vertical: false
    property bool big: false
    // Показывать ли модуль: батарея без батареи и пустой трей прячутся.
    property bool shown: true
    readonly property bool hovered: hover.hovered
    default property alias content: box.data

    signal clicked(var mouse)
    signal scrolled(int delta)

    readonly property int padX: vertical ? Math.round(Theme.pad / 2) : Theme.pad
    readonly property int padY: vertical ? Math.round(Theme.pad / 2) : Math.round(Theme.pad / 3)
    implicitWidth: box.childrenRect.width + padX * 2
    implicitHeight: box.childrenRect.height + padY * 2

    // В доке значки под мышью заметно крупнее, как в macOS.
    scale: hovered ? (big ? 1.18 : 1.06) : 1
    Behavior on scale { NumberAnimation { duration: Theme.fast; easing.type: Theme.easing } }

    Rectangle {
        anchors.fill: parent
        radius: Math.min(Theme.radiusChip, height / 2)
        color: chip.hovered ? Theme.fade(Theme.fg, 0.12) : Theme.fade(Theme.fg, 0)
        Behavior on color { ColorAnimation { duration: Theme.fast } }
    }

    HoverHandler { id: hover }

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
        onClicked: mouse => chip.clicked(mouse)
        onWheel: wheel => chip.scrolled(wheel.angleDelta.y > 0 ? 1 : -1)
    }

    Item {
        id: box
        x: chip.padX
        y: chip.padY
        width: childrenRect.width
        height: childrenRect.height
    }
}

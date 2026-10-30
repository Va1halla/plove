// Ползунок, который идёт за пальцем: пока его тянут, показывает своё
// значение и не ждёт, пока программа (звук, яркость) подтвердит новое.
import QtQuick

Item {
    id: slider

    property real value: 0          // настоящее значение снаружи, 0…max
    property real max: 1
    property bool dim: false
    signal moved(real value)        // новое значение, пока тянут

    property real dragged: 0
    readonly property bool dragging: mouse.pressed
    readonly property real shown: Math.max(0, Math.min(max, dragging ? dragged : value))

    implicitWidth: 240
    implicitHeight: 22

    Rectangle {
        id: track
        anchors.verticalCenter: parent.verticalCenter
        width: parent.width
        height: 8
        radius: 4
        color: Theme.fade(Theme.fg, 0.12)
        Rectangle {
            width: Math.max(height, track.width * slider.shown / slider.max)
            height: parent.height
            radius: parent.radius
            color: slider.dim ? Theme.muted : Theme.accent
            Behavior on width { enabled: !slider.dragging; NumberAnimation { duration: Theme.fast } }
        }
    }
    Rectangle {
        x: track.width * slider.shown / slider.max - width / 2
        anchors.verticalCenter: parent.verticalCenter
        width: slider.dragging ? 20 : 16
        height: width
        radius: width / 2
        color: Theme.fg
        Behavior on width { NumberAnimation { duration: Theme.fast } }
    }

    MouseArea {
        id: mouse
        anchors.fill: parent
        anchors.margins: -6
        function set(x) {
            slider.dragged = Math.max(0, Math.min(slider.max, (x - 6) / track.width * slider.max));
            slider.moved(slider.dragged);
        }
        onPressed: e => set(e.x)
        onPositionChanged: e => { if (pressed) set(e.x); }
        onWheel: e => slider.moved(Math.max(0, Math.min(slider.max, slider.value + (e.angleDelta.y > 0 ? 0.05 : -0.05) * slider.max)))
    }
}

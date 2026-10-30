// Кольцо загрузки с числом внутри и подписью под ним.
import QtQuick
import QtQuick.Shapes

Column {
    id: ring

    property real value: 0          // 0…1
    property string label: ""
    property real size: 64
    property color accent: Theme.accent
    property color ink: Theme.fg
    property color muted: Theme.muted
    property string font: Theme.fontMono
    spacing: size / 12

    Item {
        width: ring.size
        height: ring.size
        Shape {
            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer
            ShapePath {
                fillColor: "transparent"
                strokeColor: Theme.fade(ring.ink, 0.12)
                strokeWidth: ring.size / 11
                PathAngleArc { centerX: ring.size / 2; centerY: ring.size / 2; radiusX: ring.size * 0.45; radiusY: ring.size * 0.45; startAngle: 0; sweepAngle: 360 }
            }
            ShapePath {
                fillColor: "transparent"
                strokeColor: ring.accent
                strokeWidth: ring.size / 11
                capStyle: ShapePath.RoundCap
                PathAngleArc {
                    centerX: ring.size / 2; centerY: ring.size / 2; radiusX: ring.size * 0.45; radiusY: ring.size * 0.45
                    startAngle: -90
                    sweepAngle: 360 * Math.max(0.01, Math.min(1, ring.value))
                    Behavior on sweepAngle { NumberAnimation { duration: 600; easing.type: Easing.OutCubic } }
                }
            }
        }
        Text {
            anchors.centerIn: parent
            text: Math.round(ring.value * 100)
            color: ring.ink
            font.family: ring.font
            font.pixelSize: Math.round(ring.size * 0.24)
        }
    }
    Text {
        anchors.horizontalCenter: parent.horizontalCenter
        text: ring.label
        color: ring.muted
        font.family: ring.font
        font.pixelSize: Math.round(ring.size * 0.2)
    }
}

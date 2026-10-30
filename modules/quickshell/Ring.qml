// Кольцо уровня в стиле карточки: стекло — со свечением, точки — из
// точек по кругу, soft — толстое, classic — как в eww. Число внутри
// плавно перетекает, подпись — под кольцом.
import QtQuick
import QtQuick.Shapes
import QtQuick.Effects

Column {
    id: ring

    property var card
    property real value: 0
    property string label: ""
    property real size: 64
    readonly property string style: card ? card.style : "classic"
    readonly property color accent: card ? card.accent : Theme.accent
    readonly property color ink: card ? card.ink : Theme.fg
    property real shown: Math.max(0, Math.min(1, value))
    Behavior on shown { NumberAnimation { duration: 800; easing.type: Easing.OutCubic } }
    spacing: size / 10

    Item {
        width: ring.size
        height: ring.size

        // Точки по кругу.
        Repeater {
            model: ring.style === "dots" ? 28 : 0
            Rectangle {
                required property int index
                readonly property real angle: -Math.PI / 2 + index * 2 * Math.PI / 28
                width: ring.size / 13
                height: width
                radius: width / 2
                x: ring.size / 2 + Math.cos(angle) * ring.size * 0.44 - width / 2
                y: ring.size / 2 + Math.sin(angle) * ring.size * 0.44 - height / 2
                color: index < Math.round(ring.shown * 28) ? ring.accent : Theme.fade(ring.ink, 0.12)
            }
        }

        Shape {
            id: shape
            visible: ring.style !== "dots"
            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer
            readonly property real line: ring.size / (ring.style === "soft" ? 7 : 11)
            layer.enabled: ring.style === "glass"
            layer.samples: 4
            layer.effect: MultiEffect {
                shadowEnabled: true
                shadowColor: ring.accent
                shadowBlur: 0.7
                shadowOpacity: 0.9
                shadowHorizontalOffset: 0
                shadowVerticalOffset: 0
            }
            ShapePath {
                fillColor: "transparent"
                strokeColor: Theme.fade(ring.ink, ring.style === "soft" ? 0.10 : 0.13)
                strokeWidth: shape.line
                PathAngleArc { centerX: ring.size / 2; centerY: ring.size / 2; radiusX: ring.size * 0.42; radiusY: ring.size * 0.42; startAngle: 0; sweepAngle: 360 }
            }
            ShapePath {
                fillColor: "transparent"
                strokeColor: ring.accent
                strokeWidth: shape.line
                capStyle: ShapePath.RoundCap
                PathAngleArc {
                    centerX: ring.size / 2; centerY: ring.size / 2; radiusX: ring.size * 0.42; radiusY: ring.size * 0.42
                    startAngle: -90
                    sweepAngle: 360 * Math.max(0.01, ring.shown)
                }
            }
        }
        Text {
            anchors.centerIn: parent
            text: Math.round(ring.shown * 100) + (ring.style === "classic" ? "" : "%")
            color: ring.ink
            font.family: ring.card ? (ring.style === "dots" ? ring.card.mono : ring.card.display) : Theme.fontMono
            font.pixelSize: Math.round(ring.size * (ring.style === "classic" ? 0.24 : 0.22))
            font.weight: ring.style === "soft" ? Font.Black : Font.DemiBold
        }
    }
    CardLabel {
        anchors.horizontalCenter: parent.horizontalCenter
        card: ring.card
        text: ring.label
    }
}

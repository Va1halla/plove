// Полоса уровня в стиле карточки: стекло — градиент со свечением,
// точки — ряд точек, soft — толстая скруглённая.
import QtQuick
import QtQuick.Effects

Item {
    id: meter

    property var card
    property real value: 0              // 0…1
    property color color: card ? card.accent : Theme.accent
    readonly property string style: card ? card.style : "classic"
    readonly property real thick: card ? card.px(style === "soft" ? 10 : style === "dots" ? 6 : 6) : 6
    property real shown: Math.max(0, Math.min(1, value))
    Behavior on shown { NumberAnimation { duration: 700; easing.type: Easing.OutCubic } }

    implicitHeight: thick

    // Точки: сколько горит — по уровню.
    Row {
        visible: meter.style === "dots"
        spacing: meter.thick * 0.6
        readonly property int count: Math.max(1, Math.floor((meter.width + spacing) / (meter.thick + spacing)))
        Repeater {
            model: parent.count
            Rectangle {
                required property int index
                width: meter.thick
                height: meter.thick
                radius: meter.thick / 2
                color: index < Math.round(meter.shown * parent.count) ? meter.color : Theme.fade(meter.card ? meter.card.ink : Theme.fg, 0.10)
            }
        }
    }

    Rectangle {
        visible: meter.style !== "dots"
        width: parent.width
        height: meter.thick
        radius: meter.thick / 2
        color: meter.card ? meter.card.track : Theme.fade(Theme.fg, 0.12)
        Rectangle {
            id: bar
            width: Math.max(height, parent.width * meter.shown)
            height: parent.height
            radius: parent.radius
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0; color: meter.style === "glass" ? Qt.lighter(meter.color, 1.35) : meter.color }
                GradientStop { position: 1; color: meter.color }
            }
            layer.enabled: meter.style === "glass"
            layer.effect: MultiEffect {
                shadowEnabled: true
                shadowColor: meter.color
                shadowBlur: 0.6
                shadowOpacity: 0.8
                shadowHorizontalOffset: 0
                shadowVerticalOffset: 0
            }
        }
    }
}

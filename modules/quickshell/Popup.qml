// Выпадающее меню модуля панели: под панелью, над доком или сбоку от
// rail. Забирает фокус и закрывается щелчком мимо. Открытым может быть
// только одно меню (Config.popup).
import QtQuick
import Quickshell

PopupWindow {
    id: pop

    required property Item owner
    required property string name
    default property alias content: body.data
    readonly property bool open: Config.popup === name
    // С какой стороны от модуля: у дока — сверху, у rail — сбоку.
    readonly property int side: Config.style === "dock" ? Edges.Top
        : Config.style === "rail" ? (Config.s.side === "right" ? Edges.Left : Edges.Right)
        : Edges.Bottom
    readonly property bool across: side === Edges.Left || side === Edges.Right

    anchor.item: owner
    anchor.edges: side
    anchor.gravity: side
    anchor.adjustment: PopupAdjustment.All
    // Отступ от панели: прямоугольник модуля, расширенный на gap.
    anchor.rect.x: across ? -Theme.gap : 0
    anchor.rect.y: across ? 0 : -Theme.gap
    anchor.rect.width: owner.width + (across ? Theme.gap * 2 : 0)
    anchor.rect.height: owner.height + (across ? 0 : Theme.gap * 2)
    grabFocus: true
    visible: open
    onClosed: if (Config.popup === name) Config.popup = ""
    color: "transparent"

    implicitWidth: body.width + Theme.pad * 4
    implicitHeight: body.height + Theme.pad * 4

    Rectangle {
        id: frame
        anchors.fill: parent
        color: Theme.fade(Theme.bg, Math.max(Theme.alpha, 0.97))
        radius: Theme.radius
        border.width: Math.max(1, Theme.border)
        border.color: Theme.fade(Theme.surface, 0.9)
        // Плавное появление: чуть снизу и прозрачное.
        opacity: pop.open ? 1 : 0
        scale: pop.open ? 1 : 0.96
        Behavior on opacity { NumberAnimation { duration: Theme.fast } }
        Behavior on scale { NumberAnimation { duration: Theme.slow; easing.type: Theme.easing } }

        Item {
            id: body
            x: Theme.pad * 2
            y: Theme.pad * 2
            width: childrenRect.width
            height: childrenRect.height
        }
    }
}

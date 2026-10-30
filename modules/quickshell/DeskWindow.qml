// Рабочий стол одного монитора: карточки на нижнем слое — над обоями,
// под окнами. Место — в процентах всего экрана, как в plove studio;
// панель их не сдвигает. Щелчки мимо карточек проходят насквозь.
import QtQuick
import Quickshell
import Quickshell.Wayland

PanelWindow {
    id: win

    required property var modelData
    screen: modelData
    visible: Desk.enabled
    color: "transparent"
    WlrLayershell.namespace: "plove-desktop"
    WlrLayershell.layer: WlrLayer.Bottom
    exclusionMode: ExclusionMode.Ignore
    anchors {
        top: true
        bottom: true
        left: true
        right: true
    }
    mask: Region { item: cards }

    Item {
        id: cards
        anchors.fill: parent
        Repeater {
            model: Desk.cards
            Card {
                required property var modelData
                data_: modelData
                x: win.width * modelData.x / 100
                y: win.height * modelData.y / 100
            }
        }
    }
}

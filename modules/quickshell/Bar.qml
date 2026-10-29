// Окно панели на одном мониторе. Стиль (segments, rail, island, dock)
// решает, у какого края окно и как разложены группы модулей.
import QtQuick
import Quickshell
import Quickshell.Wayland

PanelWindow {
    id: bar

    required property var modelData
    screen: modelData

    readonly property string style: Config.style
    readonly property bool rail: style === "rail"
    readonly property bool dock: style === "dock"
    readonly property bool onRight: Config.s.side === "right"
    // Толщина плашки; у дока крупнее, у rail — по ширине значков.
    readonly property int thick: dock ? Math.round(Theme.barHeight * 1.35) : Theme.barHeight

    color: "transparent"
    WlrLayershell.namespace: "plove-bar"
    WlrLayershell.layer: WlrLayer.Top

    anchors {
        top: !dock
        bottom: dock || rail
        left: !(rail && onRight)
        right: !(rail && !onRight)
    }
    implicitHeight: rail ? 0 : thick + Theme.gap
    implicitWidth: rail ? thick + Theme.gap : 0
    exclusiveZone: rail ? implicitWidth : implicitHeight

    Loader {
        anchors.fill: parent
        sourceComponent: bar.rail ? railLayout : bar.style === "segments" ? segmentsLayout : islandLayout
    }

    // Три плашки у краёв: левая, по центру, правая.
    Component {
        id: segmentsLayout
        Item {
            Segment {
                anchors { left: parent.left; top: parent.top; leftMargin: Theme.gap; topMargin: Theme.gap }
                height: bar.thick
                groups: [Config.s.left]
                names: ["left"]
            }
            Segment {
                anchors { horizontalCenter: parent.horizontalCenter; top: parent.top; topMargin: Theme.gap }
                height: bar.thick
                groups: [Config.s.center]
                names: ["center"]
            }
            Segment {
                anchors { right: parent.right; top: parent.top; rightMargin: Theme.gap; topMargin: Theme.gap }
                height: bar.thick
                groups: [Config.s.right]
                names: ["right"]
            }
        }
    }

    // Один островок по центру сверху, у дока — снизу.
    Component {
        id: islandLayout
        Item {
            Segment {
                anchors.horizontalCenter: parent.horizontalCenter
                y: bar.dock ? 0 : Theme.gap
                height: bar.thick
                groups: [Config.s.left, Config.s.center, Config.s.right]
                names: ["left", "center", "right"]
                big: bar.dock
            }
        }
    }

    // Тонкая полоса во всю высоту: группы сверху, посередине и снизу.
    Component {
        id: railLayout
        Item {
            Segment {
                x: bar.onRight ? 0 : Theme.gap
                anchors { top: parent.top; bottom: parent.bottom; margins: Theme.gap }
                width: bar.thick
                vertical: true
                groups: [Config.s.left, Config.s.center, Config.s.right]
                names: ["left", "center", "right"]
            }
        }
    }
}

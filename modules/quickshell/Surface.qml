// Поверхность из конструктора оболочки (раздел shell райса): панель у
// края экрана или выезжающая панель. Где она, какой длины и какие в ней
// блоки — всё из settings.json, окно строится само.
import QtQuick
import Quickshell
import Quickshell.Wayland
import Quickshell.Hyprland

PanelWindow {
    id: surf

    required property var modelData
    property var screenData: null
    screen: screenData

    readonly property var s: modelData
    readonly property bool vertical: s.edge === "left" || s.edge === "right"
    readonly property bool drawer: s.kind === "drawer"
    // Во всю длину края; иначе — по размеру содержимого.
    readonly property bool full: drawer || s.align === "fill"
    // Выезжающая — только на мониторе с фокусом: два окна с HyprlandFocusGrab
    // сразу снимают захват друг у друга, и панель тут же закрывается.
    readonly property bool here: !Hyprland.focusedMonitor || !screenData || Hyprland.focusedMonitor.name === screenData.name
    readonly property bool open: !drawer || (Config.drawer === s.id && here)
    readonly property int thick: s.size
    // В начале или в конце края. Окно всё равно во всю длину: у окна в углу
    // Hyprland не учитывает зону, а лишнее место не ловит мышь (mask).
    readonly property bool pinned: !drawer && (s.align === "start" || s.align === "end")
    readonly property bool spans: full || pinned

    visible: open || slide.running || slideY.running
    color: "transparent"
    WlrLayershell.namespace: drawer ? "plove-panel" : "plove-bar"
    WlrLayershell.layer: drawer ? WlrLayer.Overlay : WlrLayer.Top
    WlrLayershell.keyboardFocus: drawer && open ? WlrKeyboardFocus.OnDemand : WlrKeyboardFocus.None
    // Отступ от окон добавит Hyprland (gaps_out), как у обычной панели.
    exclusiveZone: drawer ? 0 : (vertical ? implicitWidth : implicitHeight) - Theme.gap

    anchors {
        top: s.edge === "top" || (surf.vertical && surf.spans)
        bottom: s.edge === "bottom" || (surf.vertical && surf.spans)
        left: s.edge === "left" || (!surf.vertical && surf.spans)
        right: s.edge === "right" || (!surf.vertical && surf.spans)
    }
    implicitWidth: vertical ? thick + Theme.gap * 2 : (spans ? 0 : plate.implicitWidth + Theme.gap * 2)
    implicitHeight: vertical ? (spans ? 0 : plate.implicitHeight + Theme.gap * 2) : thick + Theme.gap * 2
    Region { id: hit; item: plate }
    mask: pinned ? hit : null

    HyprlandFocusGrab {
        windows: [surf]
        active: surf.drawer && surf.open
        onCleared: Config.drawer = ""
    }

    Rectangle {
        id: plate
        // Выезжающая панель въезжает со своего края.
        readonly property real away: surf.open ? 0 : surf.thick + Theme.gap * 2
        readonly property real sign: s.edge === "right" || s.edge === "bottom" ? 1 : -1
        readonly property bool atEnd: surf.pinned && s.align === "end"
        x: (atEnd && !surf.vertical ? surf.width - width - Theme.gap : Theme.gap) + (surf.vertical ? sign * away : 0)
        y: (atEnd && surf.vertical ? surf.height - height - Theme.gap : Theme.gap) + (surf.vertical ? 0 : sign * away)
        Behavior on x { NumberAnimation { id: slide; duration: Theme.slow; easing.type: Theme.easing } }
        Behavior on y { NumberAnimation { id: slideY; duration: Theme.slow; easing.type: Theme.easing } }
        width: surf.pinned && !surf.vertical ? implicitWidth : surf.width - Theme.gap * 2
        height: surf.pinned && surf.vertical ? implicitHeight : surf.height - Theme.gap * 2
        implicitWidth: surf.vertical ? surf.thick : layout.implicitWidth + Theme.pad * 2
        implicitHeight: surf.vertical ? layout.implicitHeight + Theme.pad * 2 : surf.thick

        // Как плашки обычной панели: стекло, точки или цвет фона.
        readonly property string look: Desk.enabled ? Desk.style : ""
        color: look === "glass" ? Theme.fade(Theme.bg, 0.42)
            : look === "dots" ? Qt.rgba(0.055, 0.055, 0.055, 0.94)
            : Theme.fade(Theme.bg, surf.drawer ? Math.max(Theme.alpha, 0.95) : Theme.alpha)
        radius: surf.drawer ? Theme.radius : Math.min(Theme.radius, (surf.vertical ? width : height) / 2)
        border.width: Math.max(1, Theme.border)
        border.color: look === "glass" ? Qt.rgba(1, 1, 1, 0.18) : Theme.fade(Theme.surface, 0.9)

        focus: surf.drawer && surf.open
        Keys.onEscapePressed: Config.drawer = ""

        // Выезжающая: блоки столбиком сверху вниз.
        // Не помещаются — прокручиваются.
        Flickable {
            visible: surf.drawer
            anchors { fill: parent; margins: Theme.pad * 2 }
            contentHeight: column.height
            boundsBehavior: Flickable.StopAtBounds
            clip: true
            Column {
                id: column
                width: parent.width
                spacing: Theme.pad * 1.5
                Repeater {
                    model: surf.drawer ? (s.sections[0] || []) : []
                    Block {
                        required property var modelData
                        block: modelData
                        inDrawer: true
                        edge: s.edge
                        width: column.width
                    }
                }
            }
        }

        // Панель: три группы — в начале, посередине и в конце.
        Item {
            id: layout
            visible: !surf.drawer
            anchors.fill: parent
            anchors.margins: Theme.pad
            implicitWidth: surf.full ? 0 : packed.implicitWidth
            implicitHeight: surf.full ? 0 : packed.implicitHeight

            // Во всю длину: группы прижаты к краям и к центру.
            Section {
                visible: surf.full
                blocks: s.sections[0] || []
                vertical: surf.vertical
                edge: s.edge
                group: "left"
                x: 0
                y: 0
                anchors.verticalCenter: surf.vertical ? undefined : parent.verticalCenter
                anchors.horizontalCenter: surf.vertical ? parent.horizontalCenter : undefined
            }
            Section {
                visible: surf.full
                blocks: s.sections[1] || []
                vertical: surf.vertical
                edge: s.edge
                group: "center"
                anchors.centerIn: parent
            }
            Section {
                visible: surf.full
                blocks: s.sections[2] || []
                vertical: surf.vertical
                edge: s.edge
                group: "right"
                anchors.right: surf.vertical ? undefined : parent.right
                anchors.bottom: surf.vertical ? parent.bottom : undefined
                anchors.verticalCenter: surf.vertical ? undefined : parent.verticalCenter
                anchors.horizontalCenter: surf.vertical ? parent.horizontalCenter : undefined
            }

            // Островок: все три группы подряд.
            Grid {
                id: packed
                visible: !surf.full
                anchors.centerIn: parent
                flow: surf.vertical ? Grid.TopToBottom : Grid.LeftToRight
                columns: surf.vertical ? 1 : 3
                rows: surf.vertical ? 3 : 1
                spacing: Theme.pad
                horizontalItemAlignment: Grid.AlignHCenter
                verticalItemAlignment: Grid.AlignVCenter
                Repeater {
                    model: surf.full ? 0 : 3
                    Section {
                        required property int index
                        blocks: s.sections[index] || []
                        vertical: surf.vertical
                        edge: s.edge
                        group: ["left", "center", "right"][index]
                        // Пустая группа не даёт лишнего промежутка.
                        visible: blocks.length > 0
                    }
                }
            }
        }
    }
}

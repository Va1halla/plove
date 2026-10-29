// Модули одной группы подряд: в строку или, у rail, в столбик.
import QtQuick

Grid {
    id: grp

    property var names: []
    property string group: "right"
    property bool vertical: false
    property bool big: false

    readonly property var files: ({
        workspaces: "Workspaces.qml",
        clock: "Clock.qml",
        tray: "Tray.qml",
        volume: "Volume.qml",
        network: "Network.qml",
        battery: "Battery.qml"
    })

    flow: vertical ? Grid.TopToBottom : Grid.LeftToRight
    columns: vertical ? 1 : Math.max(1, names.length)
    rows: vertical ? Math.max(1, names.length) : 1
    spacing: Math.round(Theme.pad / 2)
    horizontalItemAlignment: Grid.AlignHCenter
    verticalItemAlignment: Grid.AlignVCenter

    Repeater {
        model: grp.names
        Loader {
            required property string modelData
            source: grp.files[modelData] || ""
            // Модуль сам решает, есть ли что показать (батарея, трей).
            visible: item !== null && item.shown
            onLoaded: {
                item.group = grp.group;
                item.vertical = Qt.binding(() => grp.vertical);
                item.big = Qt.binding(() => grp.big);
            }
        }
    }
}

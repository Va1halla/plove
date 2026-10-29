// Значки программ в трее: щелчок — открыть программу, правый — её меню.
import QtQuick
import Quickshell
import Quickshell.Services.SystemTray
import Quickshell.Widgets

Chip {
    id: tray

    shown: SystemTray.items.values.length > 0
    readonly property int size: Math.round(Theme.fontSize * 1.15)

    Grid {
        flow: tray.vertical ? Grid.TopToBottom : Grid.LeftToRight
        columns: tray.vertical ? 1 : Math.max(1, SystemTray.items.values.length)
        spacing: Math.round(Theme.pad * 0.75)

        Repeater {
            model: SystemTray.items
            IconImage {
                id: icon
                required property var modelData
                implicitSize: tray.size
                source: modelData.icon

                MouseArea {
                    anchors.fill: parent
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    onClicked: mouse => {
                        if (mouse.button === Qt.RightButton && icon.modelData.hasMenu) {
                            const p = icon.mapToItem(null, 0, icon.height);
                            icon.modelData.display(QsWindow.window, p.x, p.y);
                        } else {
                            icon.modelData.activate();
                        }
                    }
                }
            }
        }
    }
}

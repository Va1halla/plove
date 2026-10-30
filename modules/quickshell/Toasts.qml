// Всплывающие уведомления в углу экрана: сверху справа, у дока — снизу.
// Пропадают сами через несколько секунд, важные — только по щелчку.
import QtQuick
import Quickshell
import Quickshell.Wayland
import Quickshell.Services.Notifications

PanelWindow {
    id: win

    readonly property bool bottom: Config.style === "dock"
    screen: Quickshell.screens[0]
    visible: Notifs.toasts.length > 0 && !Config.panel
    color: "transparent"
    WlrLayershell.namespace: "notifications"
    WlrLayershell.layer: WlrLayer.Overlay
    exclusiveZone: 0
    anchors {
        top: !bottom
        bottom: bottom
        right: true
    }
    margins {
        top: Theme.gap
        bottom: Theme.gap
        right: Theme.gap
    }
    implicitWidth: 360
    implicitHeight: column.height

    Column {
        id: column
        spacing: Theme.pad
        Repeater {
            model: Notifs.toasts
            NotifCard {
                id: card
                required property var modelData
                notification: modelData
                toast: true
                onClosed: { Notifs.hideToast(modelData); modelData.dismiss(); }
                // Появляется справа.
                x: 0
                NumberAnimation on x { from: 380; to: 0; duration: Theme.slow; easing.type: Theme.easing }
                Timer {
                    running: !card.critical
                    interval: card.notification.expireTimeout > 0 ? card.notification.expireTimeout : 6000
                    onTriggered: Notifs.hideToast(card.notification)
                }
            }
        }
    }
}

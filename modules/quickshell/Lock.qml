// Блокировка экрана: настоящая (ext-session-lock, Hyprland не пустит к
// окнам, пока PAM не примет пароль) и предпросмотр — тот же экран
// поверх окон, без блокировки, чтобы посмотреть оформление.
import QtQuick
import Quickshell
import Quickshell.Wayland

Scope {
    id: root

    property bool locked: false
    property bool preview: false

    WlSessionLock {
        locked: root.locked
        WlSessionLockSurface {
            color: Theme.bg
            LockView {
                anchors.fill: parent
                onUnlocked: root.locked = false
            }
        }
    }

    LazyLoader {
        active: root.preview
        PanelWindow {
            screen: Quickshell.screens[0]
            color: "transparent"
            WlrLayershell.namespace: "plove-lock-preview"
            WlrLayershell.layer: WlrLayer.Overlay
            WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
            exclusionMode: ExclusionMode.Ignore
            anchors { top: true; bottom: true; left: true; right: true }
            LockView {
                anchors.fill: parent
                preview: true
                onUnlocked: root.preview = false
            }
        }
    }
}

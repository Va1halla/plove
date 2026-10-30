// Уведомления: свой сервер D-Bus вместо mako/swaync, если в райсе
// notifications: qs-notifications. Новые всплывают карточками (toasts),
// все копятся в боковой панели. «Не беспокоить» переживает перезапуск.
pragma Singleton

import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Services.Notifications

Singleton {
    id: root

    readonly property bool enabled: Config.s.notifications
    readonly property bool dnd: state.dnd
    // Все уведомления, новые сверху.
    readonly property var list: loader.item ? loader.item.trackedNotifications.values.slice().reverse() : []
    // Всплывающие сейчас (не больше четырёх).
    property var toasts: []
    property int unread: 0
    property bool started: false

    Timer { interval: 1500; running: true; onTriggered: root.started = true }

    function setDnd(on) { state.dnd = on; }
    function hideToast(n) { toasts = toasts.filter(t => t !== n); }
    function clear() {
        for (const n of list)
            n.dismiss();
        toasts = [];
        unread = 0;
    }

    LazyLoader {
        id: loader
        active: root.enabled
        NotificationServer {
            keepOnReload: true
            actionsSupported: true
            bodyMarkupSupported: true
            imageSupported: true
            persistenceSupported: true
            onNotification: n => {
                n.tracked = true;
                // После перезагрузки панели сервер заново присылает старые
                // уведомления: их — только в список, не всплывающими.
                if (!root.started)
                    return;
                root.unread++;
                if (!root.dnd || n.urgency === NotificationUrgency.Critical)
                    root.toasts = [n].concat(root.toasts).slice(0, 4);
            }
        }
    }

    FileView {
        path: Quickshell.statePath("shell.json")
        watchChanges: true
        onFileChanged: reload()
        onAdapterUpdated: writeAdapter()
        onLoadFailed: err => { if (err === FileViewError.FileNotFound) writeAdapter(); }
        JsonAdapter {
            id: state
            property bool dnd: false
        }
    }
}

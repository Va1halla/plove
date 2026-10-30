// Колокольчик на панели: число новых уведомлений, щелчок — боковая панель.
import QtQuick

Chip {
    id: bell

    onClicked: Config.panel = !Config.panel

    Label {
        vertical: bell.vertical
        icon: Notifs.dnd ? "󰂛" : Notifs.unread > 0 ? "󰂞" : "󰂚"
        value: Notifs.unread > 0 ? String(Notifs.unread) : ""
        dim: Notifs.dnd
    }
}

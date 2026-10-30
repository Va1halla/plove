// Боковая панель справа: часы, быстрые настройки (сеть, Bluetooth,
// «не беспокоить», микрофон), громкость и яркость, плеер и уведомления.
// Super+N или щелчок по колокольчику; закрывается щелчком мимо и Esc.
import QtQuick
import Quickshell
import Quickshell.Wayland
import Quickshell.Hyprland
import Quickshell.Bluetooth
import Quickshell.Services.Pipewire

PanelWindow {
    id: panel

    readonly property bool open: Config.panel
    readonly property var sink: Pipewire.defaultAudioSink
    readonly property var source: Pipewire.defaultAudioSource
    readonly property var adapter: Bluetooth.defaultAdapter

    screen: Quickshell.screens[0]
    visible: open || slide.running
    color: "transparent"
    WlrLayershell.namespace: "plove-panel"
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: open ? WlrKeyboardFocus.OnDemand : WlrKeyboardFocus.None
    exclusiveZone: 0
    anchors {
        top: true
        bottom: true
        right: true
    }
    // Содержимое 352 px и поля по краям.
    implicitWidth: sheet.width + Theme.gap * 2

    onOpenChanged: if (open) Notifs.unread = 0

    PwObjectTracker { objects: [panel.sink, panel.source].filter(n => n) }

    HyprlandFocusGrab {
        windows: [panel]
        active: panel.open
        onCleared: Config.panel = false
    }

    SystemClock { id: clock; precision: SystemClock.Minutes }

    Rectangle {
        id: sheet
        width: 352 + Theme.pad * 4
        height: parent.height - Theme.gap * 2
        y: Theme.gap
        // Выезжает справа.
        x: panel.open ? Theme.gap : width + Theme.gap * 2
        Behavior on x { NumberAnimation { id: slide; duration: Theme.slow; easing.type: Theme.easing } }
        radius: Theme.radius
        color: Theme.fade(Theme.bg, Math.max(Theme.alpha, 0.95))
        border.width: Math.max(1, Theme.border)
        border.color: Theme.fade(Theme.surface, 0.9)
        focus: panel.open
        Keys.onEscapePressed: Config.panel = false

        Flickable {
            anchors { fill: parent; margins: Theme.pad * 2 }
            contentHeight: column.height
            clip: true

            Column {
                id: column
                width: 352
                spacing: Theme.pad * 1.5

                // Часы и дата.
                Row {
                    width: parent.width
                    Column {
                        width: parent.width - dnd.width
                        Text {
                            text: Qt.formatDateTime(clock.date, "HH:mm")
                            color: Theme.fg
                            font.family: Theme.fontDisplay
                            font.pixelSize: Math.round(Theme.fontSize * 3)
                            font.bold: true
                        }
                        Text {
                            text: {
                                const t = clock.date.toLocaleDateString(Qt.locale(), "dddd, d MMMM");
                                return t.charAt(0).toUpperCase() + t.slice(1);
                            }
                            color: Theme.muted
                            font.family: Theme.fontText
                            font.pixelSize: Theme.fontSize
                        }
                    }
                    IconButton {
                        id: dnd
                        anchors.verticalCenter: parent.verticalCenter
                        icon: "󰒓"
                        active: false
                        onClicked: { Config.panel = false; Config.run("command -v nwg-look >/dev/null && nwg-look || " + Config.terminal("plove")); }
                    }
                }

                // Быстрые настройки.
                Grid {
                    id: tiles
                    columns: 2
                    spacing: Theme.pad
                    Tile {
                        icon: Net.kind === "wifi" ? "󰤨" : Net.kind === "ethernet" ? "󰈀" : "󰤭"
                        title: Net.kind === "wifi" ? "Wi-Fi" : "Сеть"
                        detail: Net.kind === "off" ? "нет подключения" : Net.name
                        active: Net.kind !== "off"
                        onClicked: { Config.panel = false; Config.toggle("network"); }
                    }
                    Tile {
                        visible: panel.adapter !== null
                        icon: panel.adapter && panel.adapter.enabled ? "󰂯" : "󰂲"
                        title: "Bluetooth"
                        detail: panel.adapter && panel.adapter.enabled ? "включён" : "выключен"
                        active: panel.adapter !== null && panel.adapter.enabled
                        onClicked: panel.adapter.enabled = !panel.adapter.enabled
                    }
                    Tile {
                        visible: Notifs.enabled
                        icon: Notifs.dnd ? "󰂛" : "󰂚"
                        title: "Не беспокоить"
                        detail: Notifs.dnd ? "включено" : "выключено"
                        active: Notifs.dnd
                        onClicked: Notifs.setDnd(!Notifs.dnd)
                    }
                    Tile {
                        visible: panel.source !== null
                        icon: panel.source && panel.source.audio.muted ? "󰍭" : "󰍬"
                        title: "Микрофон"
                        detail: panel.source && panel.source.audio.muted ? "выключен" : "включён"
                        active: panel.source !== null && !panel.source.audio.muted
                        onClicked: panel.source.audio.muted = !panel.source.audio.muted
                    }
                }

                // Громкость и яркость.
                Row {
                    spacing: Theme.pad
                    visible: panel.sink !== null
                    IconButton {
                        icon: panel.sink && panel.sink.audio.muted ? "󰖁" : "󰕾"
                        active: !(panel.sink && panel.sink.audio.muted)
                        onClicked: panel.sink.audio.muted = !panel.sink.audio.muted
                    }
                    Slider {
                        anchors.verticalCenter: parent.verticalCenter
                        width: 352 - Theme.fontSize * 2.4 - Theme.pad
                        max: 1.5
                        value: panel.sink && panel.sink.audio ? panel.sink.audio.volume : 0
                        dim: panel.sink !== null && panel.sink.audio.muted
                        onMoved: v => panel.sink.audio.volume = v
                    }
                }
                Row {
                    spacing: Theme.pad
                    visible: Brightness.available
                    IconButton { icon: "󰃟"; active: true }
                    Slider {
                        anchors.verticalCenter: parent.verticalCenter
                        width: 352 - Theme.fontSize * 2.4 - Theme.pad
                        value: Brightness.value
                        onMoved: v => Brightness.set(v)
                    }
                }

                Media {}

                // Уведомления.
                Item {
                    visible: Notifs.enabled
                    width: parent.width
                    height: Math.max(head.height, clear.height)
                    Heading {
                        id: head
                        anchors.verticalCenter: parent.verticalCenter
                        text: "Уведомления" + (Notifs.list.length ? " · " + Notifs.list.length : "")
                    }
                    Text {
                        id: clear
                        anchors { right: parent.right; verticalCenter: parent.verticalCenter }
                        visible: Notifs.list.length > 0
                        text: "Очистить"
                        color: clearArea.containsMouse ? Theme.accent : Theme.muted
                        font.family: Theme.fontText
                        font.pixelSize: Math.round(Theme.fontSize * 0.9)
                        MouseArea { id: clearArea; anchors.fill: parent; hoverEnabled: true; onClicked: Notifs.clear() }
                    }
                }
                Text {
                    visible: Notifs.enabled && Notifs.list.length === 0
                    width: parent.width
                    horizontalAlignment: Text.AlignHCenter
                    topPadding: Theme.pad * 2
                    text: "󰂚\nНовых уведомлений нет"
                    color: Theme.muted
                    font.family: Theme.fontText
                    font.pixelSize: Theme.fontSize
                }
                Repeater {
                    model: Notifs.list
                    NotifCard {
                        required property var modelData
                        width: 352
                        notification: modelData
                        onClosed: modelData.dismiss()
                    }
                }
            }
        }
    }
}

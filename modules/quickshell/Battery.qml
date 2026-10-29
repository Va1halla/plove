// Батарея ноутбука; на компьютере без батареи модуля нет.
import QtQuick
import Quickshell.Services.UPower

Chip {
    id: bat

    readonly property var dev: UPower.displayDevice
    readonly property real level: dev ? dev.percentage : 0
    readonly property bool charging: dev !== null && dev.state === UPowerDeviceState.Charging

    shown: dev !== null && dev.isLaptopBattery

    Label {
        vertical: bat.vertical
        icon: bat.charging ? "󰂄" : ["󰂎", "󰁺", "󰁻", "󰁼", "󰁽", "󰁾", "󰁿", "󰂀", "󰂁", "󰂂", "󰁹"][Math.min(10, Math.floor(bat.level * 10))]
        value: Math.round(bat.level * 100) + "%"
        alert: bat.level < 0.15 && !bat.charging
    }
}

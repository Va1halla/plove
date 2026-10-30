// Сеть: Wi-Fi с именем и силой сигнала или проводная. Щелчок — меню сети.
import QtQuick

Chip {
    id: net

    onClicked: Config.toggle("network")

    NetworkPopup { owner: net }

    Label {
        vertical: net.vertical
        icon: Net.kind === "wifi" ? (Net.strength > 66 ? "󰤨" : Net.strength > 33 ? "󰤥" : "󰤟") : Net.kind === "ethernet" ? "󰈀" : "󰤭"
        value: Net.kind === "wifi" ? Net.name : Net.kind === "ethernet" ? "eth" : "нет сети"
        dim: Net.kind === "off"
    }
}

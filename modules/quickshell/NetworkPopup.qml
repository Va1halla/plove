// Меню сети: подключение, адрес и сети Wi-Fi рядом (если есть
// NetworkManager). Настройки — в терминале или nm-connection-editor.
import QtQuick
import Quickshell

Popup {
    id: pop
    name: "network"

    onOpenChanged: if (open) { Net.refresh(); Net.scan(); }

    Column {
        spacing: Theme.pad
        width: 280

        Heading { text: "Сеть" }
        Row {
            spacing: Theme.pad
            IconButton {
                icon: Net.kind === "wifi" ? "󰤨" : Net.kind === "ethernet" ? "󰈀" : "󰤭"
                active: Net.kind !== "off"
            }
            Column {
                anchors.verticalCenter: parent.verticalCenter
                Caption { text: Net.kind === "off" ? "Нет подключения" : Net.name }
                Text {
                    text: Net.kind === "off" ? "" : [Net.device, Net.address].filter(x => x).join(" · ")
                    color: Theme.muted
                    font.family: Theme.fontMono
                    font.pixelSize: Math.round(Theme.fontSize * 0.9)
                }
            }
        }

        Heading { text: "Wi-Fi"; visible: Net.networks.length > 0 }
        Repeater {
            model: Net.networks
            Choice {
                required property var modelData
                width: 280
                text: (modelData.secure ? "󰌾  " : "") + modelData.ssid
                detail: modelData.signal + "%"
                chosen: modelData.current
                onClicked: Config.run("nmcli dev wifi connect " + JSON.stringify(modelData.ssid) + " || " + Config.terminal("nmtui-connect"))
            }
        }

        Choice {
            width: 280
            text: "󰒓  Настройки сети"
            onClicked: {
                Config.popup = "";
                Config.run("command -v nm-connection-editor >/dev/null && nm-connection-editor || " + Config.terminal("nmtui || networkctl status; read _"));
            }
        }
    }
}

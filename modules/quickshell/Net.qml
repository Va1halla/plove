// Сеть для панели: раз в 5 секунд спрашивает qs-net.sh.
pragma Singleton

import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root

    property string kind: "off"     // wifi, ethernet, off
    property string name: ""
    property int strength: 0
    property string address: ""
    property string device: ""
    // Сети Wi-Fi рядом: [{ ssid, signal, secure, current }], пусто без NetworkManager.
    property var networks: []

    function scan() { wifi.running = true; }
    function refresh() { probe.running = true; }

    Process {
        id: probe
        command: ["sh", Quickshell.env("HOME") + "/.local/share/plove/qs-net.sh"]
        running: true
        stdout: StdioCollector {
            onStreamFinished: {
                const parts = text.trim().split("\t");
                root.kind = parts[0] || "off";
                root.name = parts[1] || "";
                root.strength = parseInt(parts[2] || "0");
                root.address = parts[3] || "";
                root.device = parts[4] || "";
            }
        }
    }

    Process {
        id: wifi
        command: ["sh", Quickshell.env("HOME") + "/.local/share/plove/qs-net.sh", "wifi"]
        stdout: StdioCollector {
            onStreamFinished: {
                root.networks = text.trim().split("\n").filter(l => l).map(l => {
                    // nmcli экранирует «:» в имени как «\:».
                    const f = l.replace(/\\:/g, "\u0001").split(":").map(x => x.replace(/\u0001/g, ":"));
                    return { current: f[0] === "*", ssid: f[1], signal: parseInt(f[2] || "0"), secure: (f[3] || "") !== "" };
                });
            }
        }
    }

    Timer {
        interval: 5000
        running: true
        repeat: true
        onTriggered: probe.running = true
    }
}

// Карточка «Показатель»: подпись, значок, одно число крупно и полоса.
import QtQuick

Column {
    id: root
    property var card
    readonly property string source: card ? card.opts.source || "cpu" : "cpu"
    readonly property var info: ({
        cpu: { title: "ПРОЦЕССОР", icon: "󰻠", value: Sys.cpu, text: Math.round(Sys.cpu * 100) + "%" },
        ram: { title: "ПАМЯТЬ", icon: "󰍛", value: Sys.ram, text: Math.round(Sys.ram * 100) + "%" },
        disk: { title: "ДИСК", icon: "󰋊", value: Sys.disk, text: Math.round(Sys.disk * 100) + "%" },
        temp: { title: "ТЕМПЕРАТУРА", icon: "󰔏", value: Sys.temp < 0 ? 0 : Sys.temp / 100, text: Sys.temp < 0 ? "—" : Math.round(Sys.temp) + "°" }
    })[source]
    anchors.verticalCenter: parent.verticalCenter
    width: parent.width
    spacing: card ? card.px(8) : 8

    Text {
        text: root.info.title
        color: root.card ? root.card.muted : Theme.muted
        font.family: root.card ? root.card.font : Theme.fontText
        font.pixelSize: root.card ? root.card.px(12) : 12
        font.letterSpacing: 1
        font.bold: true
    }
    Item {
        width: parent.width
        height: big.height
        Text {
            anchors { left: parent.left; verticalCenter: parent.verticalCenter }
            text: root.info.icon
            color: root.card ? root.card.ink : Theme.fg
            font.family: Theme.fontIcons
            font.pixelSize: root.card ? root.card.px(22) : 22
        }
        Text {
            id: big
            anchors.right: parent.right
            text: root.info.text
            color: root.card ? root.card.ink : Theme.fg
            font.family: root.card ? root.card.display : Theme.fontDisplay
            font.pixelSize: root.card ? root.card.px(34) : 34
            font.bold: true
        }
    }
    Rectangle {
        width: parent.width
        height: root.card ? root.card.px(6) : 6
        radius: height / 2
        color: Theme.fade(root.card ? root.card.ink : Theme.fg, 0.12)
        Rectangle {
            width: parent.width * Math.max(0.02, Math.min(1, root.info.value))
            height: parent.height
            radius: parent.radius
            color: root.info.value > 0.85 ? Theme.red : (root.card ? root.card.accent : Theme.accent)
            Behavior on width { NumberAnimation { duration: 600; easing.type: Easing.OutCubic } }
        }
    }
}

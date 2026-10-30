// Карточка «Показатель»: подпись, значок, одно число крупно (плавно
// перетекает) и полоса уровня в стиле карточки.
import QtQuick

Item {
    id: root
    property var card
    anchors.fill: parent
    readonly property string source: card ? card.opts.source || "cpu" : "cpu"
    readonly property var info: ({
        cpu: { title: "Процессор", icon: "󰻠", value: Sys.cpu, number: Sys.cpu * 100, suffix: "%", detail: "" },
        ram: { title: "Память", icon: "󰍛", value: Sys.ram, number: Sys.ram * 100, suffix: "%", detail: Sys.ramText },
        disk: { title: "Диск", icon: "󰋊", value: Sys.disk, number: Sys.disk * 100, suffix: "%", detail: Sys.diskText },
        temp: { title: "Температура", icon: "󰔏", value: Sys.temp < 0 ? 0 : Sys.temp / 100, number: Math.max(0, Sys.temp), suffix: "°", detail: Sys.temp < 0 ? "нет датчика" : "" }
    })[source]
    readonly property color level: root.info.value > 0.85 ? Theme.red : (card ? card.accent : Theme.accent)

    CardLabel { id: title; card: root.card; text: root.info.title }
    Text {
        anchors { right: parent.right; verticalCenter: title.verticalCenter }
        text: root.info.icon
        color: root.card ? root.card.muted : Theme.muted
        font.family: Theme.fontIcons
        font.pixelSize: root.card ? root.card.px(18) : 18
    }
    BigValue {
        id: number
        anchors { left: parent.left; verticalCenter: parent.verticalCenter; verticalCenterOffset: root.card ? -root.card.px(2) : 0 }
        card: root.card
        value: root.info.number
        suffix: root.info.suffix
        text: root.source === "temp" && Sys.temp < 0 ? "—" : ""
        size: 36
    }
    CardLabel {
        anchors { right: parent.right; baseline: number.bottom }
        card: root.card
        text: root.info.detail
        font.capitalization: Font.MixedCase
        font.letterSpacing: 0
    }
    Meter {
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
        card: root.card
        value: root.info.value
        color: root.level
    }
}

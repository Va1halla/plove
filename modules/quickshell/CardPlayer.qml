// Карточка «Плеер»: обложка, название, исполнитель, кнопки. Пока
// ничего не играет и плееров нет, карточка спрятана.
import QtQuick
import Quickshell.Widgets
import Quickshell.Services.Mpris

Row {
    id: root
    property var card
    readonly property var player: Mpris.players.values.find(p => p.isPlaying) || Mpris.players.values[0] || null
    anchors.verticalCenter: parent.verticalCenter
    spacing: card ? card.px(14) : 14

    Binding { target: root.card; property: "visible"; value: root.player !== null && root.player.trackTitle !== ""; when: root.card !== undefined }

    ClippingRectangle {
        readonly property real side: root.card ? root.card.px(110) : 110
        width: side
        height: side
        radius: Math.min(Theme.radiusChip, 16)
        color: Theme.fade(root.card ? root.card.accent : Theme.accent, 0.2)
        Image {
            anchors.fill: parent
            source: root.player ? root.player.trackArtUrl : ""
            fillMode: Image.PreserveAspectCrop
            asynchronous: true
        }
    }
    Column {
        anchors.verticalCenter: parent.verticalCenter
        width: root.card ? root.card.width - root.card.px(110 + 14 + Theme.pad * 4) : 200
        spacing: 4
        Text {
            width: parent.width
            text: root.player ? root.player.trackTitle : ""
            elide: Text.ElideRight
            color: root.card ? root.card.ink : Theme.fg
            font.family: root.card ? root.card.font : Theme.fontText
            font.pixelSize: root.card ? root.card.px(18) : 18
            font.bold: true
        }
        Text {
            width: parent.width
            text: root.player ? root.player.trackArtist : ""
            elide: Text.ElideRight
            color: root.card ? root.card.muted : Theme.muted
            font.family: root.card ? root.card.font : Theme.fontText
            font.pixelSize: root.card ? root.card.px(14) : 14
        }
        Row {
            spacing: root.card ? root.card.px(18) : 18
            topPadding: 6
            Repeater {
                model: [["󰒮", () => root.player.previous()], [root.player && root.player.isPlaying ? "󰏤" : "󰐊", () => root.player.togglePlaying()], ["󰒭", () => root.player.next()]]
                Text {
                    required property var modelData
                    text: modelData[0]
                    color: hover.containsMouse ? (root.card ? root.card.accent : Theme.accent) : (root.card ? root.card.ink : Theme.fg)
                    font.family: Theme.fontIcons
                    font.pixelSize: root.card ? root.card.px(22) : 22
                    MouseArea { id: hover; anchors.fill: parent; hoverEnabled: true; onClicked: parent.modelData[1]() }
                }
            }
        }
    }
}

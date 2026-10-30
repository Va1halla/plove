// Карточка «Плеер»: размытая обложка на всю карточку фоном, обложка,
// название, исполнитель, полоса времени и кнопки. Пока ничего не
// играет и плееров нет, карточка спрятана.
import QtQuick
import QtQuick.Effects
import Quickshell.Widgets
import Quickshell.Services.Mpris

Item {
    id: root
    property var card
    anchors.fill: parent
    readonly property var player: Mpris.players.values.find(p => p.isPlaying) || Mpris.players.values[0] || null
    readonly property string art: player ? player.trackArtUrl : ""

    Binding { target: root.card; property: "visible"; value: root.player !== null && root.player.trackTitle !== ""; when: root.card !== undefined }

    Timer {
        running: root.player !== null && root.player.isPlaying
        interval: 1000
        repeat: true
        onTriggered: root.player.positionChanged()
    }

    // Фон: та же обложка, размытая, под затемнением (не у classic).
    Image {
        id: backdrop
        visible: false
        source: root.art
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        width: root.width + (root.card ? root.card.px(44) : 0)
        height: root.height + (root.card ? root.card.px(36) : 0)
    }
    MultiEffect {
        visible: root.art !== "" && root.card && root.card.style !== "classic" && root.card.style !== "dots"
        x: -(root.card ? root.card.px(22) : 0)
        y: -(root.card ? root.card.px(18) : 0)
        width: backdrop.width
        height: backdrop.height
        source: backdrop
        blurEnabled: true
        blur: 1
        blurMax: 64
        opacity: 0.45
    }

    Row {
        anchors.verticalCenter: parent.verticalCenter
        spacing: root.card ? root.card.px(16) : 16

        ClippingRectangle {
            readonly property real side: root.height
            width: side
            height: side
            radius: root.card ? Math.min(root.card.radius * 0.7, root.card.px(18)) : 12
            color: Theme.fade(root.card ? root.card.accent : Theme.accent, 0.2)
            Image {
                anchors.fill: parent
                source: root.art
                fillMode: Image.PreserveAspectCrop
                asynchronous: true
            }
        }
        Column {
            anchors.verticalCenter: parent.verticalCenter
            width: root.width - root.height - (root.card ? root.card.px(16) : 16)
            spacing: root.card ? root.card.px(6) : 6
            CardLabel { card: root.card; text: root.player ? root.player.identity : "" }
            Text {
                width: parent.width
                text: root.player ? root.player.trackTitle : ""
                elide: Text.ElideRight
                color: root.card ? root.card.ink : Theme.fg
                font.family: root.card ? root.card.display : Theme.fontText
                font.pixelSize: root.card ? root.card.px(19) : 19
                font.weight: Font.Bold
            }
            Text {
                width: parent.width
                text: root.player ? root.player.trackArtist : ""
                elide: Text.ElideRight
                color: root.card ? root.card.muted : Theme.muted
                font.family: root.card ? root.card.font : Theme.fontText
                font.pixelSize: root.card ? root.card.px(14) : 14
            }
            Meter {
                width: parent.width
                card: root.card
                visible: root.player !== null && root.player.lengthSupported && root.player.length > 0
                value: root.player && root.player.length > 0 ? root.player.position / root.player.length : 0
            }
            Row {
                spacing: root.card ? root.card.px(20) : 20
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
}

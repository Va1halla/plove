// Плеер: обложка, название, исполнитель, кнопки и полоса времени.
// Берёт первый плеер, который сейчас играет, иначе первый вообще.
import QtQuick
import Quickshell
import Quickshell.Widgets
import Quickshell.Services.Mpris

Rectangle {
    id: media

    readonly property var players: Mpris.players.values
    readonly property var player: players.find(p => p.isPlaying) || players[0] || null

    visible: player !== null
    width: 352
    height: visible ? row.height + Theme.pad * 3 : 0
    radius: Math.min(Theme.radius, 22)
    color: Theme.fade(Theme.fg, 0.06)

    // Позиция обновляется не сама: раз в секунду, пока играет.
    Timer {
        running: media.player !== null && media.player.isPlaying
        interval: 1000
        repeat: true
        onTriggered: media.player.positionChanged()
    }

    Row {
        id: row
        x: Theme.pad * 1.5
        y: Theme.pad * 1.5
        spacing: Theme.pad * 1.5

        ClippingRectangle {
            width: 76
            height: 76
            radius: Math.min(Theme.radiusChip, 16)
            color: Theme.fade(Theme.accent, 0.2)
            Image {
                anchors.fill: parent
                source: media.player ? media.player.trackArtUrl : ""
                fillMode: Image.PreserveAspectCrop
                asynchronous: true
            }
            Text {
                anchors.centerIn: parent
                visible: !media.player || !media.player.trackArtUrl
                text: "󰎆"
                color: Theme.accent
                font.family: Theme.fontIcons
                font.pixelSize: 30
            }
        }

        Column {
            width: 352 - 76 - Theme.pad * 4.5
            spacing: 4
            Text {
                width: parent.width
                text: media.player ? media.player.trackTitle || media.player.identity : ""
                elide: Text.ElideRight
                color: Theme.fg
                font.family: Theme.fontText
                font.pixelSize: Theme.fontSize
                font.bold: true
            }
            Text {
                width: parent.width
                text: media.player ? media.player.trackArtist : ""
                elide: Text.ElideRight
                color: Theme.muted
                font.family: Theme.fontText
                font.pixelSize: Math.round(Theme.fontSize * 0.9)
            }
            Row {
                spacing: Theme.pad
                IconButton { icon: "󰒮"; active: false; onClicked: media.player.previous() }
                IconButton {
                    icon: media.player && media.player.isPlaying ? "󰏤" : "󰐊"
                    onClicked: media.player.togglePlaying()
                }
                IconButton { icon: "󰒭"; active: false; onClicked: media.player.next() }
            }
            Slider {
                width: parent.width
                visible: media.player !== null && media.player.lengthSupported && media.player.length > 0
                max: media.player && media.player.length > 0 ? media.player.length : 1
                value: media.player ? media.player.position : 0
                onMoved: v => { if (media.player.canSeek) media.player.position = v; }
            }
        }
    }
}

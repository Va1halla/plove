// Всплывающая полоса громкости и яркости внизу экрана (у дока — над
// ним): появляется, когда громкость или яркость меняются, и гаснет через
// полторы секунды. Пока открыто меню звука или боковая панель, молчит:
// там свои ползунки.
import QtQuick
import Quickshell
import Quickshell.Wayland
import Quickshell.Services.Pipewire

PanelWindow {
    id: osd

    readonly property var sink: Pipewire.defaultAudioSink
    property string kind: "volume"     // volume или brightness
    property bool shown: false
    // Первые секунды после запуска — без OSD: громкость «меняется» при подключении.
    property bool ready: false

    function show(what) {
        if (!ready || Config.panel || Config.popup === "volume")
            return;
        kind = what;
        shown = true;
        hide.restart();
    }

    screen: Quickshell.screens[0]
    visible: shown || fade.running
    color: "transparent"
    WlrLayershell.namespace: "plove-osd"
    WlrLayershell.layer: WlrLayer.Overlay
    exclusiveZone: 0
    anchors.bottom: true
    margins.bottom: Config.style === "dock" ? Theme.barHeight * 2 + Theme.gap * 2 : Theme.gap * 6
    implicitWidth: 300
    implicitHeight: 56
    // Мышь проходит насквозь: OSD не мешает щёлкать под ним.
    mask: Region {}

    PwObjectTracker { objects: osd.sink ? [osd.sink] : [] }

    Connections {
        target: osd.sink ? osd.sink.audio : null
        function onVolumeChanged() { osd.show("volume"); }
        function onMutedChanged() { osd.show("volume"); }
    }
    Connections {
        target: Brightness
        function onChanged() { osd.show("brightness"); }
    }

    Timer { interval: 2000; running: true; onTriggered: osd.ready = true }
    Timer { id: hide; interval: 1500; onTriggered: osd.shown = false }

    readonly property bool muted: kind === "volume" && sink !== null && sink.audio.muted
    readonly property real level: kind === "volume" ? (sink ? sink.audio.volume : 0) : Brightness.value

    Rectangle {
        anchors.fill: parent
        radius: Math.min(Theme.radius, height / 2)
        color: Theme.fade(Theme.bg, Math.max(Theme.alpha, 0.95))
        border.width: Math.max(1, Theme.border)
        border.color: Theme.fade(Theme.surface, 0.9)
        opacity: osd.shown ? 1 : 0
        Behavior on opacity { NumberAnimation { id: fade; duration: Theme.slow } }

        Row {
            anchors.centerIn: parent
            spacing: Theme.pad * 1.5
            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: osd.kind === "brightness" ? "󰃟" : osd.muted ? "󰖁" : osd.level > 0.6 ? "󰕾" : osd.level > 0.25 ? "󰖀" : "󰕿"
                color: osd.muted ? Theme.muted : Theme.accent
                font.family: Theme.fontIcons
                font.pixelSize: Math.round(Theme.fontSize * 1.6)
            }
            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                width: 170
                height: 8
                radius: 4
                color: Theme.fade(Theme.fg, 0.12)
                Rectangle {
                    width: Math.min(1, osd.level) * parent.width
                    height: parent.height
                    radius: parent.radius
                    color: osd.muted ? Theme.muted : Theme.accent
                    Behavior on width { NumberAnimation { duration: Theme.fast } }
                }
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                width: Theme.fontSize * 2.6
                text: Math.round(osd.level * 100)
                color: Theme.fg
                font.family: Theme.fontMono
                font.pixelSize: Theme.fontSize
            }
        }
    }
}

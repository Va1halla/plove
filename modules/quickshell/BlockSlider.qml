// Блок «Ползунок»: громкость или яркость, со значком слева.
import QtQuick
import Quickshell.Services.Pipewire

Row {
    id: row
    property var opts: ({})
    readonly property bool volume: (opts.what || "volume") === "volume"
    readonly property var sink: Pipewire.defaultAudioSink
    property bool shown: volume ? sink !== null : Brightness.available
    spacing: Theme.pad

    PwObjectTracker { objects: row.sink ? [row.sink] : [] }

    Text {
        anchors.verticalCenter: parent.verticalCenter
        text: row.volume ? "󰕾" : "󰃠"
        color: Theme.accent
        font.family: Theme.fontIcons
        font.pixelSize: Math.round(Theme.fontSize * 1.3)
    }
    Slider {
        anchors.verticalCenter: parent.verticalCenter
        width: row.width - Theme.pad * 3 - Theme.fontSize
        value: row.volume ? (row.sink && row.sink.audio ? row.sink.audio.volume : 0) : Brightness.value
        dim: row.volume && row.sink !== null && row.sink.audio.muted
        onMoved: v => {
            if (row.volume)
                row.sink.audio.volume = v;
            else
                Brightness.set(v);
        }
    }
}

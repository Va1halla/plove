// Блок «Ползунок»: громкость или яркость, со значком слева.
import QtQuick
import Quickshell.Services.Pipewire

Item {
    id: row
    property var opts: ({})
    readonly property bool volume: (opts.what || "volume") === "volume"
    readonly property var sink: Pipewire.defaultAudioSink
    property bool shown: volume ? sink !== null : Brightness.available
    implicitWidth: 240
    implicitHeight: bar.implicitHeight

    PwObjectTracker { objects: row.sink ? [row.sink] : [] }

    Text {
        id: icon
        anchors.verticalCenter: parent.verticalCenter
        text: row.volume ? "󰕾" : "󰃠"
        color: Theme.accent
        font.family: Theme.fontIcons
        font.pixelSize: Math.round(Theme.fontSize * 1.3)
    }
    Slider {
        id: bar
        // Справа место под половину ручки, иначе её срежет край столбика.
        anchors { left: icon.right; right: parent.right; leftMargin: Theme.pad; rightMargin: 10; verticalCenter: parent.verticalCenter }
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

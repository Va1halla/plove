// Меню звука: громкость выхода и микрофона, выбор устройства вывода.
import QtQuick
import Quickshell
import Quickshell.Services.Pipewire

Popup {
    id: pop
    name: "volume"

    readonly property var sink: Pipewire.defaultAudioSink
    readonly property var source: Pipewire.defaultAudioSource
    readonly property var outputs: Pipewire.nodes.values.filter(n => n.isSink && !n.isStream && n.audio)

    PwObjectTracker { objects: [pop.sink, pop.source].filter(n => n) }

    Column {
        spacing: Theme.pad
        width: 280

        Heading { text: "Звук" }
        Row {
            spacing: Theme.pad
            IconButton {
                icon: pop.sink && pop.sink.audio.muted ? "󰖁" : "󰕾"
                active: !(pop.sink && pop.sink.audio.muted)
                onClicked: if (pop.sink) pop.sink.audio.muted = !pop.sink.audio.muted
            }
            Slider {
                anchors.verticalCenter: parent.verticalCenter
                width: 190
                max: 1.5
                value: pop.sink && pop.sink.audio ? pop.sink.audio.volume : 0
                dim: pop.sink && pop.sink.audio.muted
                onMoved: v => { if (pop.sink) pop.sink.audio.volume = v; }
            }
            Caption {
                anchors.verticalCenter: parent.verticalCenter
                text: Math.round((pop.sink && pop.sink.audio ? pop.sink.audio.volume : 0) * 100) + "%"
            }
        }
        Row {
            spacing: Theme.pad
            visible: pop.source !== null
            IconButton {
                icon: pop.source && pop.source.audio.muted ? "󰍭" : "󰍬"
                active: !(pop.source && pop.source.audio.muted)
                onClicked: if (pop.source) pop.source.audio.muted = !pop.source.audio.muted
            }
            Slider {
                anchors.verticalCenter: parent.verticalCenter
                width: 190
                value: pop.source && pop.source.audio ? pop.source.audio.volume : 0
                dim: pop.source && pop.source.audio.muted
                onMoved: v => { if (pop.source) pop.source.audio.volume = v; }
            }
        }

        Heading { text: "Вывод"; visible: pop.outputs.length > 1 }
        Repeater {
            model: pop.outputs.length > 1 ? pop.outputs : []
            Choice {
                required property var modelData
                width: 280
                text: modelData.description || modelData.nickname || modelData.name
                chosen: modelData === pop.sink
                onClicked: Pipewire.preferredDefaultAudioSink = modelData
            }
        }
    }
}

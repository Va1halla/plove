// Громкость: колесо меняет, щелчок — меню звука, правый — микшер.
import QtQuick
import Quickshell.Services.Pipewire

Chip {
    id: vol

    readonly property var sink: Pipewire.defaultAudioSink
    readonly property bool ready: sink !== null && sink.audio !== null
    readonly property real level: ready ? sink.audio.volume : 0
    readonly property bool muted: ready ? sink.audio.muted : false

    shown: ready
    onClicked: mouse => {
        if (mouse.button === Qt.MiddleButton)
            sink.audio.muted = !sink.audio.muted;
        else if (mouse.button === Qt.RightButton)
            Config.run("command -v pavucontrol >/dev/null && pavucontrol || " + Config.terminal("wpctl status; read _"));
        else
            Config.toggle("volume");
    }
    onScrolled: delta => {
        if (ready)
            sink.audio.volume = Math.max(0, Math.min(1.5, level + delta * 0.05));
    }

    VolumePopup { owner: vol }

    PwObjectTracker {
        objects: vol.sink ? [vol.sink] : []
    }

    Label {
        vertical: vol.vertical
        icon: vol.muted ? "󰖁" : vol.level > 0.6 ? "󰕾" : vol.level > 0.25 ? "󰖀" : "󰕿"
        value: Math.round(vol.level * 100) + "%"
        dim: vol.muted
    }
}

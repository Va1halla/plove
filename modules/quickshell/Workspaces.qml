// Рабочие столы точками: занятые ярче, активный вытягивается в
// черточку цвета акцента, и при переключении она плавно переезжает.
import QtQuick
import Quickshell.Hyprland

Chip {
    id: ws

    readonly property int active: Hyprland.focusedWorkspace ? Hyprland.focusedWorkspace.id : 1
    // Номера столов: 1…N всегда, плюс занятые дальше N. Спецстолы (id < 1) не показываем.
    readonly property var ids: {
        const all = Hyprland.workspaces.values.map(w => w.id).filter(id => id > 0);
        const n = Math.max(Config.s.workspaces, 0);
        const out = [];
        for (let i = 1; i <= n; i++)
            out.push(i);
        for (const id of all)
            if (out.indexOf(id) < 0)
                out.push(id);
        return out.sort((a, b) => a - b);
    }
    readonly property int dot: Math.round(Theme.fontSize * 0.55)

    function occupied(id) {
        const w = Hyprland.workspaces.values.find(w => w.id === id);
        return w !== undefined && w.toplevels.values.length > 0;
    }

    onScrolled: delta => Config.workspace(delta > 0 ? "e-1" : "e+1")

    Grid {
        flow: ws.vertical ? Grid.TopToBottom : Grid.LeftToRight
        columns: ws.vertical ? 1 : Math.max(1, ws.ids.length)
        spacing: Math.round(ws.dot * 0.8)
        horizontalItemAlignment: Grid.AlignHCenter
        verticalItemAlignment: Grid.AlignVCenter

        Repeater {
            model: ws.ids
            Rectangle {
                id: mark
                required property int modelData
                readonly property bool current: modelData === ws.active
                readonly property int long: current ? Math.round(ws.dot * 2.6) : ws.dot
                width: ws.vertical ? ws.dot : long
                height: ws.vertical ? long : ws.dot
                radius: ws.dot / 2
                color: current ? Theme.accent : ws.occupied(modelData) ? Theme.fg : Theme.fade(Theme.muted, 0.6)
                Behavior on width { NumberAnimation { duration: Theme.slow; easing.type: Theme.easing } }
                Behavior on height { NumberAnimation { duration: Theme.slow; easing.type: Theme.easing } }
                Behavior on color { ColorAnimation { duration: Theme.slow } }

                MouseArea {
                    anchors.fill: parent
                    anchors.margins: -Math.round(ws.dot / 2)
                    onClicked: Config.workspace(mark.modelData)
                }
            }
        }
    }
}

// Лаунчер: программы с поиском, калькулятор прямо в строке поиска
// (= 2+2 или просто выражение) и режим dmenu — список строк от другой
// программы (меню выхода, история буфера обмена). Super+D, Esc — закрыть.
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Hyprland
import Quickshell.Widgets

PanelWindow {
    id: win

    property bool open: false
    // Режим dmenu: файл со строками; выбор пишется в <файл>.out.
    property string dmenuFile: ""
    property string prompt: ""
    property var lines: []
    property int current: 0
    property string answer: ""

    function show() {
        dmenuFile = "";
        prompt = "";
        start();
    }
    function dmenu(path, hint) {
        dmenuFile = path;
        prompt = hint;
        linesFile.path = path;
        linesFile.reload();
        lines = linesFile.text().split("\n").filter(l => l !== "");
        start();
    }
    function start() {
        Config.popup = "";
        Config.panel = false;
        input.text = "";
        answer = "";
        current = 0;
        open = true;
        input.forceActiveFocus();
    }
    function close(choice) {
        if (!open) return;
        open = false;
        if (dmenuFile) {
            writer.command = ["sh", "-c", "printf '%s' \"$1\" > \"$2.out\"", "dmenu", choice || "", dmenuFile];
            writer.running = true;
        }
    }

    // Похоже на выражение: цифры и знаки, или начинается с «=».
    readonly property bool calc: !dmenuFile && (/^=/.test(input.text) || /^[\d(.][\d\s+\-*/%^().,]*[\d)%]$/.test(input.text.trim()) && /[+\-*/%^]/.test(input.text))
    readonly property string query: input.text.trim().toLowerCase()
    readonly property var results: {
        if (dmenuFile)
            return lines.filter(l => l.toLowerCase().indexOf(query) >= 0).slice(0, 50).map(l => ({ text: l }));
        const apps = DesktopEntries.applications.values.filter(a => !a.noDisplay);
        if (!query)
            return apps.slice().sort((a, b) => a.name.localeCompare(b.name)).slice(0, 50).map(a => ({ app: a, text: a.name }));
        // Сначала начинается с запроса, потом содержит его, потом по описанию.
        const score = a => {
            const name = a.name.toLowerCase();
            if (name.startsWith(query)) return 0;
            if (name.indexOf(query) >= 0) return 1;
            const extra = [a.genericName, a.comment, (a.keywords || []).join(" "), a.id].join(" ").toLowerCase();
            return extra.indexOf(query) >= 0 ? 2 : 9;
        };
        return apps.map(a => ({ app: a, text: a.name, score: score(a) }))
            .filter(r => r.score < 9)
            .sort((x, y) => x.score - y.score || x.text.localeCompare(y.text))
            .slice(0, 50);
    }

    function accept() {
        if (calc && answer) {
            Quickshell.execDetached(["wl-copy", answer]);
            close("");
            return;
        }
        const r = results[current];
        if (dmenuFile) {
            close(r ? r.text : input.text);
        } else if (r) {
            r.app.execute();
            close("");
        }
    }

    screen: Quickshell.screens[0]
    visible: open
    color: "transparent"
    WlrLayershell.namespace: "launcher"
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: open ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
    exclusiveZone: 0
    anchors.top: true
    margins.top: Math.round(screen.height * 0.18)
    implicitWidth: 560
    implicitHeight: box.height

    HyprlandFocusGrab {
        windows: [win]
        active: win.open
        onCleared: win.close("")
    }

    FileView { id: linesFile; blockLoading: true }
    Process { id: writer }

    // Калькулятор: qalc, как в меню панели.
    Process {
        id: qalc
        stdout: StdioCollector { onStreamFinished: win.answer = text.trim() }
    }
    Timer {
        id: solve
        interval: 150
        onTriggered: {
            qalc.command = ["sh", "-c", "command -v qalc >/dev/null && LC_ALL=C qalc -t -- \"$1\"", "calc", input.text.replace(/^=/, "")];
            qalc.running = true;
        }
    }

    Rectangle {
        id: box
        width: parent.width
        height: column.height + Theme.pad * 3
        radius: Theme.radius
        color: Theme.fade(Theme.bg, Math.max(Theme.alpha, 0.96))
        border.width: Math.max(1, Theme.border)
        border.color: Theme.fade(Theme.surface, 0.9)
        scale: win.open ? 1 : 0.97
        Behavior on scale { NumberAnimation { duration: Theme.fast; easing.type: Theme.easing } }

        Column {
            id: column
            x: Theme.pad * 1.5
            y: Theme.pad * 1.5
            width: parent.width - Theme.pad * 3
            spacing: Theme.pad

            Rectangle {
                width: parent.width
                height: Math.round(Theme.fontSize * 2.8)
                radius: Math.min(Theme.radiusChip, height / 2)
                color: Theme.fade(Theme.fg, 0.07)
                Text {
                    id: lens
                    anchors { left: parent.left; leftMargin: Theme.pad * 1.2; verticalCenter: parent.verticalCenter }
                    text: win.dmenuFile ? "󰍜" : win.calc ? "󰃬" : "󰍉"
                    color: Theme.accent
                    font.family: Theme.fontIcons
                    font.pixelSize: Math.round(Theme.fontSize * 1.3)
                }
                TextInput {
                    id: input
                    anchors { left: lens.right; leftMargin: Theme.pad; right: parent.right; rightMargin: Theme.pad; verticalCenter: parent.verticalCenter }
                    color: Theme.fg
                    selectionColor: Theme.accent
                    font.family: Theme.fontText
                    font.pixelSize: Math.round(Theme.fontSize * 1.15)
                    clip: true
                    onTextChanged: {
                        win.current = 0;
                        win.answer = "";
                        if (win.calc) solve.restart();
                    }
                    Keys.onEscapePressed: win.close("")
                    Keys.onReturnPressed: win.accept()
                    Keys.onEnterPressed: win.accept()
                    Keys.onDownPressed: win.current = Math.min(win.results.length - 1, win.current + 1)
                    Keys.onUpPressed: win.current = Math.max(0, win.current - 1)
                    Keys.onTabPressed: win.current = (win.current + 1) % Math.max(1, win.results.length)
                    Text {
                        visible: input.text === ""
                        text: win.prompt || (win.dmenuFile ? "Выберите…" : "Программа или выражение: 2+2, = 10 usd to eur")
                        color: Theme.muted
                        font: input.font
                    }
                }
            }

            // Ответ калькулятора: Enter копирует.
            Rectangle {
                visible: win.calc
                width: parent.width
                height: Math.round(Theme.fontSize * 3.4)
                radius: Math.min(Theme.radiusChip, 14)
                color: Theme.fade(Theme.accent, 0.16)
                Text {
                    anchors { left: parent.left; leftMargin: Theme.pad * 1.5; verticalCenter: parent.verticalCenter }
                    text: "= " + (win.answer || "…")
                    color: Theme.accent
                    font.family: Theme.fontDisplay
                    font.pixelSize: Math.round(Theme.fontSize * 1.6)
                    font.bold: true
                }
                Text {
                    anchors { right: parent.right; rightMargin: Theme.pad * 1.5; verticalCenter: parent.verticalCenter }
                    text: "Enter — скопировать"
                    color: Theme.muted
                    font.family: Theme.fontText
                    font.pixelSize: Math.round(Theme.fontSize * 0.85)
                }
            }

            ListView {
                id: list
                visible: !win.calc
                width: parent.width
                height: Math.min(contentHeight, Theme.fontSize * 2.6 * 8)
                clip: true
                model: win.results
                currentIndex: win.current
                highlightMoveDuration: Theme.fast
                highlight: Rectangle {
                    radius: Math.min(Theme.radiusChip, 12)
                    color: Theme.fade(Theme.accent, 0.18)
                }
                delegate: Item {
                    id: row
                    required property var modelData
                    required property int index
                    width: list.width
                    height: Math.round(Theme.fontSize * 2.6)
                    Row {
                        anchors { left: parent.left; leftMargin: Theme.pad; verticalCenter: parent.verticalCenter }
                        spacing: Theme.pad
                        IconImage {
                            visible: row.modelData.app !== undefined
                            implicitSize: Math.round(Theme.fontSize * 1.7)
                            source: row.modelData.app ? Quickshell.iconPath(row.modelData.app.icon, "application-x-executable") : ""
                            anchors.verticalCenter: parent.verticalCenter
                        }
                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            width: list.width - Theme.fontSize * 4
                            text: row.modelData.text
                            elide: Text.ElideRight
                            color: row.index === win.current ? Theme.accent : Theme.fg
                            font.family: Theme.fontText
                            font.pixelSize: Theme.fontSize
                        }
                    }
                    MouseArea {
                        anchors.fill: parent
                        hoverEnabled: true
                        onEntered: win.current = row.index
                        onClicked: { win.current = row.index; win.accept(); }
                    }
                }
            }
            Text {
                visible: !win.calc && win.results.length === 0
                text: "Ничего не нашлось"
                color: Theme.muted
                font.family: Theme.fontText
                font.pixelSize: Theme.fontSize
            }
        }
    }
}

// Калькулятор: qalc (libqalculate) понимает проценты, единицы и валюты;
// без него — обычная арифметика. Enter — посчитать, второй Enter —
// скопировать ответ.
import QtQuick
import Quickshell
import Quickshell.Io

Popup {
    id: pop
    name: "calculator"

    property string answer: ""
    onOpenChanged: if (open) { input.text = ""; answer = ""; input.forceActiveFocus(); }

    function solve() {
        const expr = input.text.trim();
        if (!expr) return;
        qalc.command = ["sh", "-c", "command -v qalc >/dev/null && LC_ALL=C qalc -t -- \"$1\"", "calc", expr];
        qalc.fallback = expr;
        qalc.running = true;
    }

    Process {
        id: qalc
        property string fallback: ""
        stdout: StdioCollector {
            onStreamFinished: {
                const out = text.trim();
                if (out) {
                    pop.answer = out;
                    return;
                }
                // Нет qalc: только цифры, скобки и + - * / % ^.
                if (/^[0-9+\-*/%^().,\s]+$/.test(qalc.fallback)) {
                    try {
                        pop.answer = String(Function("return (" + qalc.fallback.replace(/\^/g, "**").replace(/,/g, ".") + ")")());
                    } catch (e) {
                        pop.answer = "?";
                    }
                } else {
                    pop.answer = "?";
                }
            }
        }
    }

    Column {
        spacing: Theme.pad
        width: 300

        Heading { text: "Калькулятор" }
        Rectangle {
            width: 300
            height: Math.round(Theme.fontSize * 2.4)
            radius: Math.min(Theme.radiusChip, height / 2)
            color: Theme.fade(Theme.fg, 0.08)
            border.width: input.activeFocus ? 1 : 0
            border.color: Theme.accent
            TextInput {
                id: input
                anchors { fill: parent; leftMargin: Theme.pad; rightMargin: Theme.pad }
                verticalAlignment: TextInput.AlignVCenter
                color: Theme.fg
                selectionColor: Theme.accent
                font.family: Theme.fontMono
                font.pixelSize: Theme.fontSize
                clip: true
                onAccepted: {
                    if (pop.answer && pop.answer !== "?" && text === lastSolved) {
                        Quickshell.execDetached(["wl-copy", pop.answer]);
                        Config.popup = "";
                    } else {
                        lastSolved = text;
                        pop.solve();
                    }
                }
                property string lastSolved: ""
                Keys.onEscapePressed: Config.popup = ""
            }
            Text {
                anchors { left: parent.left; leftMargin: Theme.pad; verticalCenter: parent.verticalCenter }
                visible: input.text === ""
                text: "2+2*2, 3200 * 15%, 10 usd to eur"
                color: Theme.muted
                font: input.font
            }
        }
        Text {
            width: 300
            text: pop.answer ? "= " + pop.answer : ""
            visible: text !== ""
            wrapMode: Text.Wrap
            color: Theme.accent
            font.family: Theme.fontDisplay
            font.pixelSize: Math.round(Theme.fontSize * 1.5)
            font.bold: true
        }
        Text {
            text: pop.answer && pop.answer !== "?" ? "Enter ещё раз — скопировать" : ""
            visible: text !== ""
            color: Theme.muted
            font.family: Theme.fontText
            font.pixelSize: Math.round(Theme.fontSize * 0.85)
        }
    }
}

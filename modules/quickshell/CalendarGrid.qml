// Месяц сеткой: заголовок со стрелками, дни недели с понедельника,
// сегодняшний день цветом акцента. Колесо листает месяцы.
import QtQuick
import Quickshell

Column {
    id: cal

    property int shift: 0
    property real cell: Math.round(Theme.fontSize * 2.3)
    property real text: Theme.fontSize
    property color accent: Theme.accent
    property color ink: Theme.fg
    property color muted: Theme.muted
    property string font: Theme.fontText
    property string mono: Theme.fontMono
    // Заголовок и дни недели капителями (виды карточек кроме classic).
    property bool caps: false
    spacing: Math.round(cell / 4)

    SystemClock { id: clock; precision: SystemClock.Minutes }

    readonly property date month: new Date(clock.date.getFullYear(), clock.date.getMonth() + shift, 1)
    readonly property var cells: {
        const first = (month.getDay() + 6) % 7;
        const days = new Date(month.getFullYear(), month.getMonth() + 1, 0).getDate();
        const out = [];
        for (let i = 0; i < first; i++) out.push(0);
        for (let d = 1; d <= days; d++) out.push(d);
        return out;
    }

    WheelHandler { onWheel: e => cal.shift += e.angleDelta.y > 0 ? -1 : 1 }

    Item {
        width: grid.width
        height: title.height
        Text {
            anchors { left: parent.left; verticalCenter: parent.verticalCenter }
            text: "󰅁"
            color: cal.muted
            font.family: Theme.fontIcons
            font.pixelSize: cal.text
            MouseArea { anchors.fill: parent; anchors.margins: -6; onClicked: cal.shift-- }
        }
        Text {
            id: title
            anchors.horizontalCenter: parent.horizontalCenter
            text: {
                const t = Qt.locale().standaloneMonthName(cal.month.getMonth(), Locale.LongFormat);
                return t.charAt(0).toUpperCase() + t.slice(1) + " " + cal.month.getFullYear();
            }
            color: cal.accent
            font.family: cal.font
            font.pixelSize: Math.round(cal.text * 1.1)
            font.bold: true
            font.capitalization: cal.caps ? Font.AllUppercase : Font.MixedCase
            font.letterSpacing: cal.caps ? 1.5 : 0
        }
        Text {
            anchors { right: parent.right; verticalCenter: parent.verticalCenter }
            text: "󰅂"
            color: cal.muted
            font.family: Theme.fontIcons
            font.pixelSize: cal.text
            MouseArea { anchors.fill: parent; anchors.margins: -6; onClicked: cal.shift++ }
        }
    }

    Grid {
        id: grid
        columns: 7
        Repeater {
            model: [1, 2, 3, 4, 5, 6, 0]
            Text {
                required property int modelData
                width: cal.cell
                horizontalAlignment: Text.AlignHCenter
                text: Qt.locale().dayName(modelData, Locale.ShortFormat)
                color: cal.muted
                font.family: cal.font
                font.pixelSize: Math.round(cal.text * 0.85)
                font.capitalization: cal.caps ? Font.AllUppercase : Font.MixedCase
            }
        }
        Repeater {
            model: cal.cells
            Rectangle {
                required property int modelData
                readonly property bool today: cal.shift === 0 && modelData === clock.date.getDate()
                width: cal.cell
                height: cal.cell * 0.86
                // Сегодня — таблетка во всю клетку.
                radius: cal.caps ? height / 2 : Math.min(Theme.radiusChip, height / 2)
                color: today ? cal.accent : "transparent"
                Text {
                    anchors.centerIn: parent
                    text: parent.modelData > 0 ? parent.modelData : ""
                    color: parent.today ? Theme.bg : cal.ink
                    font.family: cal.mono
                    font.pixelSize: cal.text
                    font.bold: parent.today
                }
            }
        }
    }
}

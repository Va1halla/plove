// Календарь по щелчку на часах: месяц с сегодняшним днём, стрелки и
// колесо листают месяцы.
import QtQuick
import Quickshell

Popup {
    id: pop
    name: "calendar"

    property int shift: 0          // месяцев от текущего
    onOpenChanged: if (open) shift = 0

    SystemClock { id: clock; precision: SystemClock.Minutes }

    readonly property date month: new Date(clock.date.getFullYear(), clock.date.getMonth() + shift, 1)
    // Ячейки: пустые до первого дня (неделя с понедельника), затем дни.
    readonly property var cells: {
        const first = (month.getDay() + 6) % 7;
        const days = new Date(month.getFullYear(), month.getMonth() + 1, 0).getDate();
        const out = [];
        for (let i = 0; i < first; i++) out.push(0);
        for (let d = 1; d <= days; d++) out.push(d);
        return out;
    }
    readonly property int cell: Math.round(Theme.fontSize * 2.3)

    Column {
        spacing: Theme.pad
        WheelHandler { onWheel: e => pop.shift += e.angleDelta.y > 0 ? -1 : 1 }

        Item {
            width: grid.width
            height: title.height
            IconButton { icon: "󰅁"; active: false; width: title.height; anchors.left: parent.left; onClicked: pop.shift-- }
            Text {
                id: title
                anchors.horizontalCenter: parent.horizontalCenter
                text: {
                    const t = Qt.locale().standaloneMonthName(pop.month.getMonth(), Locale.LongFormat);
                    return t.charAt(0).toUpperCase() + t.slice(1) + " " + pop.month.getFullYear();
                }
                color: Theme.accent
                font.family: Theme.fontText
                font.pixelSize: Math.round(Theme.fontSize * 1.1)
                font.bold: true
            }
            IconButton { icon: "󰅂"; active: false; width: title.height; anchors.right: parent.right; onClicked: pop.shift++ }
        }

        Grid {
            id: grid
            columns: 7
            Repeater {
                model: [1, 2, 3, 4, 5, 6, 0]
                Text {
                    required property int modelData
                    width: pop.cell
                    horizontalAlignment: Text.AlignHCenter
                    text: Qt.locale().dayName(modelData, Locale.ShortFormat)
                    color: Theme.muted
                    font.family: Theme.fontText
                    font.pixelSize: Math.round(Theme.fontSize * 0.85)
                }
            }
            Repeater {
                model: pop.cells
                Rectangle {
                    required property int modelData
                    readonly property bool today: pop.shift === 0 && modelData === clock.date.getDate()
                    width: pop.cell
                    height: pop.cell
                    radius: Math.min(Theme.radiusChip, width / 2)
                    color: today ? Theme.accent : "transparent"
                    Text {
                        anchors.centerIn: parent
                        text: parent.modelData > 0 ? parent.modelData : ""
                        color: parent.today ? Theme.bg : Theme.fg
                        font.family: Theme.fontMono
                        font.pixelSize: Theme.fontSize
                        font.bold: parent.today
                    }
                }
            }
        }
    }
}

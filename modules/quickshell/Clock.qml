// Часы; щелчок — календарь. У rail время в два ряда, у island дата
// выезжает, когда наводишь мышь.
import QtQuick
import Quickshell

Chip {
    id: c

    readonly property bool showDate: !vertical && (Config.s.date || (Config.style === "island" && hovered))

    onClicked: Config.toggle("calendar")

    CalendarPopup { owner: c }

    SystemClock {
        id: clock
        precision: Config.s.time.indexOf("ss") >= 0 ? SystemClock.Seconds : SystemClock.Minutes
    }

    Item {
        width: c.vertical ? column.width : row.width
        height: c.vertical ? column.height : row.height

        Row {
            id: row
            visible: !c.vertical
            spacing: date.width > 0 ? Theme.pad : 0
            // Двоеточие рисуем сами: у шрифтов-табло (Doto) свой знак
            // «:» похож на «⁝».
            Row {
                id: time
                anchors.verticalCenter: parent.verticalCenter
                spacing: Math.round(Theme.fontSize * 0.12)
                readonly property real size: Math.round(Theme.fontSize * (c.big ? 1.35 : 1.1))
                Repeater {
                    model: Qt.formatDateTime(clock.date, Config.s.time).split(":")
                    Row {
                        required property string modelData
                        required property int index
                        spacing: time.spacing
                        Column {
                            visible: index > 0
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: time.size * 0.22
                            Repeater {
                                model: 2
                                Rectangle {
                                    width: Math.max(2, Math.round(time.size * 0.14))
                                    height: width
                                    radius: width / 2
                                    color: Theme.fg
                                }
                            }
                        }
                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: modelData
                            color: Theme.fg
                            font.family: Theme.fontDisplay
                            font.pixelSize: time.size
                            font.bold: true
                        }
                    }
                }
            }
            Text {
                id: date
                text: clock.date.toLocaleDateString(Qt.locale(), "ddd, d MMM")
                color: Theme.muted
                font.family: Theme.fontText
                font.pixelSize: Theme.fontSize
                anchors.verticalCenter: parent.verticalCenter
                width: c.showDate ? implicitWidth : 0
                clip: true
                Behavior on width { NumberAnimation { duration: Theme.slow; easing.type: Theme.easing } }
            }
        }

        Column {
            id: column
            visible: c.vertical
            Repeater {
                model: c.vertical ? [Qt.formatDateTime(clock.date, "HH"), Qt.formatDateTime(clock.date, "mm")] : []
                Text {
                    required property string modelData
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: modelData
                    color: Theme.fg
                    font.family: Theme.fontDisplay
                    font.pixelSize: Math.round(Theme.fontSize * 1.1)
                    font.bold: true
                }
            }
        }
    }
}

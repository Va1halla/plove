// Блок «Крупные часы» выезжающей панели: время и дата.
import QtQuick
import Quickshell

Column {
    id: big
    property var opts: ({})
    readonly property real size: Math.round(Theme.fontSize * 3)
    spacing: 2
    SystemClock { id: clock; precision: big.opts.seconds ? SystemClock.Seconds : SystemClock.Minutes }
    // Двоеточие рисуем сами, как в Clock.qml: у Doto знак «:» похож на «⁝».
    Row {
        spacing: Math.round(big.size * 0.12)
        Repeater {
            model: Qt.formatTime(clock.date, big.opts.seconds ? "HH:mm:ss" : "HH:mm").split(":")
            Row {
                required property string modelData
                required property int index
                spacing: Math.round(big.size * 0.12)
                Column {
                    visible: index > 0
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: big.size * 0.22
                    Repeater {
                        model: 2
                        Rectangle {
                            width: Math.round(big.size * 0.12)
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
                    font.family: Theme.fontDisplay || Theme.fontText
                    font.pixelSize: big.size
                    font.weight: Font.Bold
                }
            }
        }
    }
    Text {
        text: clock.date.toLocaleDateString(Qt.locale("ru_RU"), "dddd, d MMMM")
        color: Theme.muted
        font.family: Theme.fontText
        font.pixelSize: Theme.fontSize
    }
}

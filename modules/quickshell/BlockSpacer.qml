// Блок «Отступ»: пустое место.
import QtQuick

Item {
    property var opts: ({})
    property bool vertical: false
    implicitWidth: vertical ? 1 : (opts.size || 16)
    implicitHeight: vertical ? (opts.size || 16) : 1
}

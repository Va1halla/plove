// Плашка панели: фон и группы модулей. Размер следует за содержимым
// плавно — например, когда у островка раскрывается дата.
import QtQuick

Rectangle {
    id: seg

    property var groups: []      // списки модулей
    property var names: []       // left, center, right — для выпадающих меню
    property bool vertical: false
    property bool big: false

    readonly property bool empty: groups.every(g => !g || g.length === 0)
    visible: !empty

    color: Theme.fade(Theme.bg, Theme.alpha)
    radius: Math.min(Theme.radius, (vertical ? width : height) / 2)
    border.width: Theme.border
    border.color: Theme.fade(Theme.surface, 0.9)

    // По ширине содержимого; у rail ширину задаёт панель.
    implicitWidth: row.implicitWidth + Theme.pad * 2
    Behavior on implicitWidth { NumberAnimation { duration: Theme.slow; easing.type: Theme.easing } }

    // Горизонтальная раскладка: группы подряд, между ними точка.
    Row {
        id: row
        visible: !seg.vertical
        anchors.centerIn: parent
        spacing: Theme.pad
        Repeater {
            model: seg.vertical ? 0 : seg.groups.length
            Row {
                required property int index
                spacing: Theme.pad
                visible: (seg.groups[index] || []).length > 0
                Rectangle {
                    visible: index > 0 && seg.groups.slice(0, index).some(g => g && g.length > 0)
                    anchors.verticalCenter: parent.verticalCenter
                    width: 4; height: 4; radius: 2
                    color: Theme.muted
                }
                Group {
                    anchors.verticalCenter: parent.verticalCenter
                    names: seg.groups[index] || []
                    group: seg.names[index] || "right"
                    big: seg.big
                }
            }
        }
    }

    // Вертикальная раскладка rail: первая группа сверху, вторая по центру, третья снизу.
    Group {
        visible: seg.vertical
        anchors { top: parent.top; topMargin: Theme.pad; horizontalCenter: parent.horizontalCenter }
        names: seg.vertical ? (seg.groups[0] || []) : []
        group: "left"
        vertical: true
    }
    Group {
        visible: seg.vertical
        anchors.centerIn: parent
        names: seg.vertical ? (seg.groups[1] || []) : []
        group: "center"
        vertical: true
    }
    Group {
        visible: seg.vertical
        anchors { bottom: parent.bottom; bottomMargin: Theme.pad; horizontalCenter: parent.horizontalCenter }
        names: seg.vertical ? (seg.groups[2] || []) : []
        group: "right"
        vertical: true
    }
}

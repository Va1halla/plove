// Группа блоков поверхности: в строку или, у боковой панели, в столбик.
import QtQuick

Grid {
    id: sec

    property var blocks: []
    property bool vertical: false
    property string group: "right"
    property string edge: "top"

    flow: vertical ? Grid.TopToBottom : Grid.LeftToRight
    columns: vertical ? 1 : Math.max(1, blocks.length)
    rows: vertical ? Math.max(1, blocks.length) : 1
    spacing: Math.round(Theme.pad / 2)
    horizontalItemAlignment: Grid.AlignHCenter
    verticalItemAlignment: Grid.AlignVCenter

    Repeater {
        model: sec.blocks
        Block {
            required property var modelData
            block: modelData
            vertical: sec.vertical
            group: sec.group
            edge: sec.edge
        }
    }
}

// Один блок конструктора: по виду (kind) загружает свой файл и отдаёт ему
// параметры (opts). Готовые модули панели — те же, что у обычной панели.
import QtQuick

Loader {
    id: host

    property var block: null
    property bool vertical: false
    property bool inDrawer: false
    property string group: "right"
    property string edge: "top"

    readonly property string kind: block ? block.kind : ""
    readonly property var files: ({
        workspaces: "Workspaces.qml",
        clock: "Clock.qml",
        tray: "Tray.qml",
        volume: "Volume.qml",
        network: "Network.qml",
        battery: "Battery.qml",
        calculator: "Calculator.qml",
        bell: "Bell.qml",
        text: "BlockText.qml",
        button: "BlockButton.qml",
        command: "BlockCommand.qml",
        spacer: "BlockSpacer.qml",
        heading: "BlockHeading.qml",
        slider: "BlockSlider.qml",
        media: "Media.qml",
        calendar: "CalendarGrid.qml",
        clockbig: "BlockClock.qml"
    })
    // Свой виджет райса (раздел widgets) — по имени.
    readonly property bool custom: !files[kind] && Config.s.widgets[kind] !== undefined
    source: files[kind] || (custom ? "CustomWidget.qml" : "")
    visible: item !== null && (item.shown === undefined || item.shown)

    onLoaded: {
        if ("opts" in item)
            item.opts = Qt.binding(() => host.block ? (host.block.opts || {}) : {});
        if ("group" in item)
            item.group = host.group;
        if ("edge" in item)
            item.edge = host.edge;
        // Отступ в выезжающей панели — по высоте, столбик идёт сверху вниз.
        if ("vertical" in item)
            item.vertical = Qt.binding(() => host.vertical || (host.inDrawer && host.kind === "spacer"));
        if (host.custom)
            item.widgetId = host.kind;
        // Месяц в выезжающей панели — на всю её ширину.
        if (host.inDrawer && host.kind === "calendar")
            item.cell = Qt.binding(() => Math.floor(host.width / 7));
    }
}

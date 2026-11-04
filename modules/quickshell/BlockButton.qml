// Блок «Кнопка»: значок с подписью. Запускает команду или открывает
// выезжающую панель по её имени.
import QtQuick

Chip {
    id: chip
    property var opts: ({})
    onClicked: {
        if (opts.opens)
            Config.toggleDrawer(opts.opens);
        else if (opts.command)
            Config.run(opts.command);
    }
    Label {
        vertical: chip.vertical
        icon: chip.opts.icon || ""
        value: chip.vertical ? "" : (chip.opts.label || "")
    }
}

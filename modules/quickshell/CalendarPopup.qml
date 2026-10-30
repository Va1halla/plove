// Календарь по щелчку на часах: стрелки и колесо листают месяцы.
import QtQuick

Popup {
    id: pop
    name: "calendar"
    onOpenChanged: if (open) grid.shift = 0

    CalendarGrid { id: grid }
}

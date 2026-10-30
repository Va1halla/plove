// Калькулятор на панели: щелчок открывает строку ввода.
import QtQuick

Chip {
    id: calc

    onClicked: Config.toggle("calculator")

    Label {
        vertical: calc.vertical
        icon: "󰃬"
        value: ""
    }

    CalcPopup { owner: calc }
}

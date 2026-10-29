// Панель plove на Quickshell: по одной на каждый монитор.
// Сгенерированы только Theme.qml и settings.json, остальное — как есть.
import QtQuick
import Quickshell

ShellRoot {
    Variants {
        model: Quickshell.screens
        Bar {}
    }
}

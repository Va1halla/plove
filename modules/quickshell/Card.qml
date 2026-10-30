// Карточка рабочего стола: рамка по стилю райса и его look (шрифт,
// цвета, фон, скругление, рамка), внутри — вид карточки по kind.
// Размеры шрифтов — как в eww, умноженные на scale карточки.
import QtQuick

Rectangle {
    id: card

    property var data_: ({})
    readonly property var look: data_.look || ({})
    readonly property var opts: data_.opts || ({})
    readonly property real s: data_.scale || 1
    readonly property color accent: look.accent || Theme.accent
    readonly property color ink: look.text || Theme.fg
    readonly property color muted: look.muted_argb || Theme.muted
    // Шрифты райса (как в eww и studio), а не панели; look.font — сильнее.
    readonly property string font: look.font || Theme.fontUi
    readonly property string mono: look.font || Theme.fontSystemMono
    readonly property string display: look.font || Theme.fontUi
    function px(n) { return Math.round(n * s); }

    // Часы по умолчанию без фона, остальные — плашка, если в стиле есть стекло.
    readonly property bool plain: data_.kind === "clock" || data_.kind === "dotclock"
    width: data_.w || 300
    height: data_.h || 100
    color: look.fill_argb ? look.fill_argb : (!plain && Theme.glass ? Theme.fade(Theme.bg, Math.max(0.15, Theme.alpha - 0.25)) : "transparent")
    radius: look.radius !== undefined && look.radius !== null ? look.radius : (Theme.glass ? Theme.radius : 0)
    border.width: look.border || 0
    border.color: accent

    // Появление при запуске и после смены раскладки.
    opacity: 0
    scale: 0.96
    Component.onCompleted: { opacity = 1; scale = 1; }
    Behavior on opacity { NumberAnimation { duration: Theme.slow } }
    Behavior on scale { NumberAnimation { duration: Theme.slow; easing.type: Theme.easing } }

    readonly property var files: ({
        clock: "CardClock.qml",
        dotclock: "CardDotclock.qml",
        worldclock: "CardWorldclock.qml",
        calendar: "CardCalendar.qml",
        system: "CardSystem.qml",
        player: "CardPlayer.qml",
        weather: "CardWeather.qml",
        countdown: "CardCountdown.qml",
        note: "CardNote.qml",
        widget: "CardWidget.qml",
        metric: "CardMetric.qml",
        sparkline: "CardSparkline.qml"
    })

    Loader {
        id: body
        anchors.fill: parent
        anchors.leftMargin: card.px(Theme.pad * 2)
        anchors.rightMargin: card.px(Theme.pad * 2)
        anchors.topMargin: card.px(Theme.pad)
        anchors.bottomMargin: card.px(Theme.pad)
        source: card.files[card.data_.kind] || ""
        onLoaded: item.card = card
    }
}

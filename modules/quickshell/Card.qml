// Карточка рабочего стола. Вид — из настройки qs-desktop (Desk.style):
//   glass   — матовое стекло: прозрачный фон (Hyprland размывает под ним),
//             светлый блик сверху, тонкая светлая рамка, мягкая тень;
//   dots    — почти чёрная плоская карточка с точечной сеткой, один цвет
//             акцента, подписи моноширинными капителями, цифры из точек;
//   soft    — светлая карточка с тёмным текстом, крупными жирными цифрами
//             и мягкой тенью (на светлой палитре — наоборот);
//   classic — как было у eww.
// look карточки (шрифт, цвета, фон, скругление, рамка) — поверх вида.
// Появляются по очереди, под мышью чуть приподнимаются.
import QtQuick
import QtQuick.Effects

Item {
    id: card

    property var data_: ({})
    readonly property var look: data_.look || ({})
    readonly property var opts: data_.opts || ({})
    readonly property real s: data_.scale || 1
    readonly property string style: Desk.style || "glass"
    function px(n) { return Math.round(n * s); }

    // Тёмная ли палитра: от этого зависит, светлые или тёмные карточки soft.
    readonly property bool darkTheme: Theme.bg.hslLightness < 0.5
    readonly property color baseInk: style === "dots" ? "#f2f2f2"
        : style === "soft" ? (darkTheme ? Theme.bg : Theme.fg)
        : Theme.fg
    readonly property color accent: look.accent || Theme.accent
    readonly property color ink: look.text || baseInk
    readonly property color muted: look.muted_argb || Theme.fade(ink, style === "dots" ? 0.55 : 0.62)
    readonly property color track: Theme.fade(ink, style === "soft" ? 0.10 : 0.13)
    // Шрифты райса (как в studio); у dots подписи и цифры — моноширинные.
    readonly property string font: look.font || (style === "dots" ? Theme.fontSystemMono : Theme.fontUi)
    readonly property string mono: look.font || Theme.fontSystemMono
    readonly property string display: look.font || (style === "dots" ? Theme.fontSystemMono : Theme.fontUi)
    // Подписи-заголовки: капителями, с разрядкой.
    readonly property bool caps: style !== "classic"

    // Часы без рамки в classic и glass: только цифры с тенью.
    readonly property bool frameless: (data_.kind === "clock" || data_.kind === "dotclock") && (style === "classic" || style === "glass") && !look.fill_argb
    readonly property color fill: look.fill_argb ? look.fill_argb
        : frameless ? "transparent"
        : style === "glass" ? Theme.fade(Theme.bg, 0.30)
        : style === "dots" ? Qt.rgba(0.055, 0.055, 0.055, 0.94)
        : style === "soft" ? Theme.fade(darkTheme ? Qt.tint(Theme.fg, Theme.fade(Theme.accent, 0.06)) : Theme.bg, 0.95)
        : Theme.glass ? Theme.fade(Theme.bg, Math.max(0.15, Theme.alpha - 0.25)) : "transparent"
    readonly property real radius: look.radius !== undefined && look.radius !== null ? look.radius
        : style === "glass" ? px(26) : style === "dots" ? px(20) : style === "soft" ? px(24)
        : Theme.glass ? Theme.radius : 0

    width: data_.w || 300
    height: data_.h || 100

    // Появление: по очереди, снизу вверх.
    opacity: 0
    transform: Translate { id: shift; y: card.px(16) }
    SequentialAnimation {
        running: true
        PauseAnimation { duration: 90 * (card.data_.index || 0) }
        ParallelAnimation {
            NumberAnimation { target: card; property: "opacity"; to: 1; duration: 420; easing.type: Easing.OutCubic }
            NumberAnimation { target: shift; property: "y"; to: 0; duration: 520; easing.type: Easing.OutBack; easing.overshoot: 1.2 }
        }
    }
    // Под мышью — чуть выше и тень гуще.
    HoverHandler { id: hover }
    states: State {
        when: hover.hovered && !card.frameless
        PropertyChanges { target: lift; y: -card.px(4) }
    }
    transitions: Transition { NumberAnimation { property: "y"; duration: 220; easing.type: Easing.OutCubic } }

    Item {
        id: lift
        width: card.width
        height: card.height

        RectangularShadow {
            visible: !card.frameless && card.style !== "classic" && card.style !== "dots"
            anchors.fill: frame
            radius: card.radius
            offset.y: hover.hovered ? card.px(14) : card.px(8)
            blur: card.style === "glass" ? card.px(40) : card.px(28)
            spread: 0
            color: Qt.rgba(0, 0, 0, card.style === "glass" ? 0.32 : (card.darkTheme ? 0.35 : 0.14))
            Behavior on offset.y { NumberAnimation { duration: 220 } }
        }

        Rectangle {
            id: frame
            anchors.fill: parent
            radius: card.radius
            color: card.fill
            clip: true
            border.width: look.border ? look.border
                : card.frameless ? 0
                : card.style === "glass" ? 1
                : card.style === "dots" ? 1 : 0
            border.color: look.border ? card.accent
                : card.style === "glass" ? Qt.rgba(1, 1, 1, 0.16)
                : Qt.rgba(1, 1, 1, 0.07)

            // Стекло: блик сверху.
            Rectangle {
                visible: card.style === "glass" && !card.frameless
                anchors { left: parent.left; right: parent.right; top: parent.top; margins: 1 }
                height: parent.height * 0.55
                radius: card.radius
                gradient: Gradient {
                    GradientStop { position: 0; color: Qt.rgba(1, 1, 1, 0.13) }
                    GradientStop { position: 1; color: Qt.rgba(1, 1, 1, 0) }
                }
            }
            // Точки: сетка по всей карточке.
            Canvas {
                visible: card.style === "dots" && !card.frameless
                anchors.fill: parent
                onPaint: {
                    const ctx = getContext("2d");
                    ctx.reset();
                    ctx.fillStyle = "rgba(255,255,255,0.06)";
                    const step = card.px(10);
                    for (let y = step / 2; y < height; y += step)
                        for (let x = step / 2; x < width; x += step) {
                            ctx.beginPath();
                            ctx.arc(x, y, card.px(1.1), 0, Math.PI * 2);
                            ctx.fill();
                        }
                }
            }
        }

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
            anchors.leftMargin: card.px(card.style === "classic" ? Theme.pad * 2 : 22)
            anchors.rightMargin: card.px(card.style === "classic" ? Theme.pad * 2 : 22)
            anchors.topMargin: card.px(card.style === "classic" ? Theme.pad : 18)
            anchors.bottomMargin: card.px(card.style === "classic" ? Theme.pad : 18)
            source: lift.files[card.data_.kind] || ""
            onLoaded: item.card = card
        }
    }
}

// Карточка «Погода»: значок, крупная температура, описание, ощущается
// как и строка прогноза на три дня (wttr.in раз в 15 минут).
import QtQuick
import Quickshell
import Quickshell.Io

Item {
    id: root
    property var card
    anchors.fill: parent
    property var report: null

    // Ночь: между закатом и восходом (время из прогноза, «07:35 AM»).
    function minutes(t) {
        const m = (t || "").match(/(\d+):(\d+)\s*(AM|PM)/);
        return m ? (parseInt(m[1]) % 12 + (m[3] === "PM" ? 12 : 0)) * 60 + parseInt(m[2]) : -1;
    }
    readonly property bool night: {
        const a = report ? report.weather[0].astronomy[0] : null;
        if (!a) return false;
        const now = clock.date.getHours() * 60 + clock.date.getMinutes();
        const rise = minutes(a.sunrise), set = minutes(a.sunset);
        return rise >= 0 && set >= 0 && (now < rise || now >= set);
    }
    SystemClock { id: clock; precision: SystemClock.Minutes }

    // Коды погоды WWO → значки Nerd Font; ясно и облачно ночью — луна.
    function icon(code, dark) {
        const c = parseInt(code);
        if (c === 113) return dark ? "󰖔" : "󰖙";
        if (c === 116) return dark ? "󰼱" : "󰖕";
        if (c === 119 || c === 122) return "󰖐";
        if ([143, 248, 260].indexOf(c) >= 0) return "󰖑";
        if ([200, 386, 389, 392, 395].indexOf(c) >= 0) return "󰙾";
        if ([179, 182, 185, 227, 230, 317, 320, 323, 326, 329, 332, 335, 338, 350, 362, 365, 368, 371, 374, 377].indexOf(c) >= 0) return "󰖘";
        if ([176, 263, 266, 293, 296, 353].indexOf(c) >= 0) return "󰖗";
        return "󰖖";
    }
    readonly property var now: report ? report.current_condition[0] : null
    readonly property var days: report ? report.weather.slice(0, 3) : []
    readonly property bool roomy: card ? root.height > card.px(100) : true

    Process {
        id: fetch
        command: ["sh", Quickshell.env("HOME") + "/.local/share/plove/qs-weather.sh", root.card ? (root.card.opts.city || "") : ""]
        running: root.card !== undefined
        stdout: StdioCollector {
            onStreamFinished: {
                try { root.report = JSON.parse(text); } catch (e) { root.report = null; }
            }
        }
    }
    Timer { interval: 900000; running: true; repeat: true; onTriggered: fetch.running = true }

    Row {
        id: top
        spacing: root.card ? root.card.px(14) : 14
        Text {
            id: glyph
            anchors.verticalCenter: parent.verticalCenter
            text: root.now ? root.icon(root.now.weatherCode, root.night) : "󰖪"
            color: root.card ? root.card.accent : Theme.accent
            font.family: Theme.fontIcons
            font.pixelSize: root.card ? root.card.px(52) : 52
        }
        BigValue {
            id: temp
            anchors.verticalCenter: parent.verticalCenter
            card: root.card
            value: root.now ? parseInt(root.now.temp_C) : 0
            text: root.now ? "" : "—"
            prefix: root.now && parseInt(root.now.temp_C) > 0 ? "+" : ""
            suffix: "°"
            size: root.card && root.card.style === "dots" ? 32 : 44
        }
        Column {
            anchors.verticalCenter: parent.verticalCenter
            spacing: 2
            Text {
                width: root.width - glyph.width - temp.width - top.spacing * 2
                text: root.now ? (root.now.lang_ru ? root.now.lang_ru[0].value : root.now.weatherDesc[0].value) : "нет связи"
                elide: Text.ElideRight
                color: root.card ? root.card.ink : Theme.fg
                font.family: root.card ? root.card.font : Theme.fontText
                font.pixelSize: root.card ? root.card.px(15) : 15
                font.weight: Font.DemiBold
            }
            // Две короткие строки вместо одной обрезанной.
            CardLabel {
                width: root.width - glyph.width - temp.width - top.spacing * 2
                elide: Text.ElideRight
                card: root.card
                text: root.now ? "ощущается " + root.now.FeelsLikeC + "°" : ""
            }
            CardLabel {
                width: root.width - glyph.width - temp.width - top.spacing * 2
                elide: Text.ElideRight
                card: root.card
                text: root.now ? "влажность " + root.now.humidity + "%" : ""
            }
        }
    }

    // Прогноз: день, значок, максимум и минимум.
    Row {
        visible: root.roomy && root.days.length > 0
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
        Repeater {
            model: root.days
            Column {
                required property var modelData
                required property int index
                width: root.width / 3
                spacing: root.card ? root.card.px(3) : 3
                CardLabel {
                    card: root.card
                    text: index === 0 ? "сегодня" : new Date(modelData.date + "T12:00:00").toLocaleDateString(Qt.locale(), "ddd")
                }
                Row {
                    spacing: root.card ? root.card.px(6) : 6
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: root.icon(modelData.hourly[4].weatherCode)
                        color: root.card ? root.card.accent : Theme.accent
                        font.family: Theme.fontIcons
                        font.pixelSize: root.card ? root.card.px(17) : 17
                    }
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: modelData.maxtempC + "° " + modelData.mintempC + "°"
                        color: root.card ? root.card.ink : Theme.fg
                        font.family: root.card ? root.card.mono : Theme.fontMono
                        font.pixelSize: root.card ? root.card.px(14) : 14
                    }
                }
            }
        }
    }
}

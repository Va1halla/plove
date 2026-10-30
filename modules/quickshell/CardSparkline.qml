// Карточка «График»: загрузка процессора или памяти за две минуты.
import QtQuick

Column {
    id: root
    property var card
    readonly property bool cpu: !card || (card.opts.source || "cpu") === "cpu"
    readonly property var points: cpu ? Sys.cpuHistory : Sys.ramHistory
    readonly property real now: cpu ? Sys.cpu : Sys.ram
    anchors.fill: parent
    spacing: card ? card.px(6) : 6

    Item {
        width: parent.width
        height: title.height
        Text {
            id: title
            text: (root.cpu ? "ПРОЦЕССОР" : "ПАМЯТЬ") + " · 2 МИН"
            color: root.card ? root.card.muted : Theme.muted
            font.family: root.card ? root.card.font : Theme.fontText
            font.pixelSize: root.card ? root.card.px(12) : 12
            font.letterSpacing: 1
            font.bold: true
        }
        Text {
            anchors { right: parent.right; baseline: title.baseline }
            text: Math.round(root.now * 100) + "%"
            color: root.card ? root.card.accent : Theme.accent
            font.family: root.card ? root.card.mono : Theme.fontMono
            font.pixelSize: root.card ? root.card.px(20) : 20
            font.bold: true
        }
    }

    Canvas {
        id: chart
        width: parent.width
        height: parent.height - title.height - parent.spacing
        readonly property color line: root.card ? root.card.accent : Theme.accent
        onPaint: {
            const ctx = getContext("2d");
            ctx.reset();
            const pts = root.points, n = Sys.samples;
            if (pts.length < 2) return;
            const x = i => width * (i + n - pts.length) / (n - 1);
            const y = v => height - 2 - (height - 4) * Math.max(0, Math.min(1, v));
            ctx.beginPath();
            ctx.moveTo(x(0), y(pts[0]));
            for (let i = 1; i < pts.length; i++) ctx.lineTo(x(i), y(pts[i]));
            ctx.lineWidth = Math.max(1.5, root.card ? root.card.px(2) : 2);
            ctx.strokeStyle = line;
            ctx.lineJoin = "round";
            ctx.stroke();
            // Заливка под линией, к низу прозрачнее.
            ctx.lineTo(x(pts.length - 1), height);
            ctx.lineTo(x(0), height);
            ctx.closePath();
            const g = ctx.createLinearGradient(0, 0, 0, height);
            g.addColorStop(0, Qt.rgba(line.r, line.g, line.b, 0.35));
            g.addColorStop(1, Qt.rgba(line.r, line.g, line.b, 0));
            ctx.fillStyle = g;
            ctx.fill();
        }
        Connections { target: Sys; function onCpuHistoryChanged() { chart.requestPaint(); } }
    }
}

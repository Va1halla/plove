// Карточка «График»: загрузка процессора или памяти за две минуты.
// glass — линия со свечением и градиентом под ней, dots — точки по
// значениям, soft — толстая линия и бледная заливка.
import QtQuick
import QtQuick.Effects

Item {
    id: root
    property var card
    anchors.fill: parent
    readonly property bool cpu: !card || (card.opts.source || "cpu") === "cpu"
    readonly property var points: cpu ? Sys.cpuHistory : Sys.ramHistory
    readonly property real now: cpu ? Sys.cpu : Sys.ram
    readonly property string style: card ? card.style : "classic"
    readonly property color line: card ? card.accent : Theme.accent

    CardLabel { id: title; card: root.card; text: (root.cpu ? "Процессор" : "Память") + " · 2 мин" }
    BigValue {
        anchors { right: parent.right; verticalCenter: title.verticalCenter }
        card: root.card
        value: root.now * 100
        suffix: "%"
        size: 22
        color: root.line
    }

    Canvas {
        id: chart
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom; top: title.bottom; topMargin: root.card ? root.card.px(10) : 10 }
        layer.enabled: root.style === "glass"
        layer.effect: MultiEffect {
            shadowEnabled: true
            shadowColor: root.line
            shadowBlur: 0.5
            shadowOpacity: 0.7
            shadowHorizontalOffset: 0
            shadowVerticalOffset: 0
        }
        onPaint: {
            const ctx = getContext("2d");
            ctx.reset();
            const pts = root.points, n = Sys.samples;
            if (pts.length < 2) return;
            const pad = root.card ? root.card.px(3) : 3;
            const x = i => width * (i + n - pts.length) / (n - 1);
            const y = v => height - pad - (height - pad * 2) * Math.max(0, Math.min(1, v));
            const c = root.line;
            if (root.style === "dots") {
                // Столбики из точек: высота — значение.
                const step = Math.max(4, width / n);
                const dot = Math.max(2, step * 0.55);
                for (let i = 0; i < pts.length; i++) {
                    const cx = x(i);
                    const rows = Math.round(pts[i] * (height / (dot * 1.6)));
                    for (let r = 0; r <= rows; r++) {
                        ctx.fillStyle = r === rows ? Qt.rgba(c.r, c.g, c.b, 1) : Qt.rgba(c.r, c.g, c.b, 0.28);
                        ctx.beginPath();
                        ctx.arc(cx, height - pad - r * dot * 1.6, dot / 2, 0, Math.PI * 2);
                        ctx.fill();
                    }
                }
                return;
            }
            ctx.beginPath();
            ctx.moveTo(x(0), y(pts[0]));
            for (let i = 1; i < pts.length; i++) {
                // Сглаживание: кривая через середины отрезков.
                const mx = (x(i - 1) + x(i)) / 2, my = (y(pts[i - 1]) + y(pts[i])) / 2;
                ctx.quadraticCurveTo(x(i - 1), y(pts[i - 1]), mx, my);
            }
            ctx.lineTo(x(pts.length - 1), y(pts[pts.length - 1]));
            ctx.lineWidth = root.style === "soft" ? (root.card ? root.card.px(4) : 4) : Math.max(1.5, root.card ? root.card.px(2) : 2);
            ctx.strokeStyle = c;
            ctx.lineJoin = "round";
            ctx.lineCap = "round";
            ctx.stroke();
            ctx.lineTo(x(pts.length - 1), height);
            ctx.lineTo(x(0), height);
            ctx.closePath();
            const g = ctx.createLinearGradient(0, 0, 0, height);
            g.addColorStop(0, Qt.rgba(c.r, c.g, c.b, root.style === "soft" ? 0.22 : 0.4));
            g.addColorStop(1, Qt.rgba(c.r, c.g, c.b, 0));
            ctx.fillStyle = g;
            ctx.fill();
        }
        Connections { target: Sys; function onCpuHistoryChanged() { chart.requestPaint(); } }
    }
}

// Карточка уведомления: значок программы или картинка, заголовок,
// текст, кнопки действий. Щелчок — действие по умолчанию.
import QtQuick
import Quickshell
import Quickshell.Widgets
import Quickshell.Services.Notifications

Rectangle {
    id: card

    required property var notification
    property bool toast: false
    signal closed()

    readonly property bool critical: notification.urgency === NotificationUrgency.Critical
    readonly property string picture: notification.image || (notification.appIcon ? Quickshell.iconPath(notification.appIcon, true) : "")

    width: 360
    height: content.height + Theme.pad * 3
    radius: Math.min(Theme.radius, 18)
    color: toast ? Theme.fade(Theme.bg, Math.max(Theme.alpha, 0.95)) : Theme.fade(Theme.fg, area.containsMouse ? 0.09 : 0.05)
    border.width: toast || critical ? Math.max(1, Theme.border) : 0
    border.color: critical ? Theme.red : Theme.fade(Theme.surface, 0.9)
    Behavior on color { ColorAnimation { duration: Theme.fast } }

    MouseArea {
        id: area
        anchors.fill: parent
        hoverEnabled: true
        onClicked: {
            const def = card.notification.actions.find(a => a.identifier === "default");
            if (def)
                def.invoke();
            card.closed();
        }
    }

    Row {
        id: content
        x: Theme.pad * 1.5
        y: Theme.pad * 1.5
        width: parent.width - Theme.pad * 3
        spacing: Theme.pad

        IconImage {
            visible: card.picture !== ""
            implicitSize: Math.round(Theme.fontSize * 2.6)
            source: card.picture
        }

        Column {
            width: content.width - (card.picture !== "" ? Theme.fontSize * 2.6 + Theme.pad : 0) - close.width
            spacing: 2
            Text {
                width: parent.width
                text: card.notification.appName
                visible: text !== ""
                color: Theme.muted
                elide: Text.ElideRight
                font.family: Theme.fontText
                font.pixelSize: Math.round(Theme.fontSize * 0.8)
            }
            Text {
                width: parent.width
                text: card.notification.summary
                color: card.critical ? Theme.red : Theme.fg
                wrapMode: Text.Wrap
                maximumLineCount: 2
                elide: Text.ElideRight
                font.family: Theme.fontText
                font.pixelSize: Theme.fontSize
                font.bold: true
            }
            Text {
                width: parent.width
                text: card.notification.body
                visible: text !== ""
                color: Theme.fg
                opacity: 0.85
                textFormat: Text.StyledText
                wrapMode: Text.Wrap
                maximumLineCount: card.toast ? 3 : 6
                elide: Text.ElideRight
                font.family: Theme.fontText
                font.pixelSize: Math.round(Theme.fontSize * 0.92)
            }
            Row {
                spacing: Theme.pad / 2
                topPadding: Theme.pad / 2
                visible: actions.count > 0
                Repeater {
                    id: actions
                    model: card.notification.actions.filter(a => a.identifier !== "default")
                    Rectangle {
                        required property var modelData
                        height: Math.round(Theme.fontSize * 1.9)
                        width: label.width + Theme.pad * 2
                        radius: Math.min(Theme.radiusChip, height / 2)
                        color: Theme.fade(Theme.accent, act.containsMouse ? 0.3 : 0.18)
                        Text {
                            id: label
                            anchors.centerIn: parent
                            text: parent.modelData.text
                            color: Theme.accent
                            font.family: Theme.fontText
                            font.pixelSize: Math.round(Theme.fontSize * 0.9)
                        }
                        MouseArea {
                            id: act
                            anchors.fill: parent
                            hoverEnabled: true
                            onClicked: { parent.modelData.invoke(); card.closed(); }
                        }
                    }
                }
            }
        }

        Text {
            id: close
            text: "󰅖"
            color: closeArea.containsMouse ? Theme.fg : Theme.muted
            font.family: Theme.fontIcons
            font.pixelSize: Theme.fontSize
            MouseArea {
                id: closeArea
                anchors.fill: parent
                anchors.margins: -6
                hoverEnabled: true
                onClicked: card.closed()
            }
        }
    }
}

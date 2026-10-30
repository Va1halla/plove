// Экран блокировки: размытые обои, часы точками, дата и поле пароля.
// Пароль проверяет PAM (конфиг login, как у hyprlock). В режиме
// предпросмотра экран не блокируется, Esc закрывает.
import QtQuick
import QtQuick.Effects
import Quickshell
import Quickshell.Services.Pam

Item {
    id: view

    property bool preview: false
    signal unlocked()

    property string message: ""
    property bool failed: false

    function submit() {
        if (pam.active || input.text === "")
            return;
        message = "Проверяю…";
        failed = false;
        pam.start();
    }

    SystemClock { id: clock; precision: SystemClock.Minutes }

    PamContext {
        id: pam
        config: "login"
        onResponseRequiredChanged: {
            if (responseRequired)
                respond(input.text);
        }
        onCompleted: result => {
            if (result === PamResult.Success) {
                view.message = "";
                input.text = "";
                view.unlocked();
            } else {
                view.failed = true;
                view.message = "Неверный пароль";
                input.text = "";
                shake.restart();
            }
        }
        onError: err => {
            view.failed = true;
            view.message = "PAM: " + PamError.toString(err);
        }
    }

    Rectangle { anchors.fill: parent; color: Theme.bg }

    Image {
        id: wall
        anchors.fill: parent
        source: Config.s.wallpaper ? "file://" + Config.s.wallpaper : ""
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        visible: false
    }
    MultiEffect {
        anchors.fill: parent
        source: wall
        visible: wall.status === Image.Ready
        blurEnabled: true
        blur: 1
        blurMax: 48
        brightness: -0.15
    }
    Rectangle { anchors.fill: parent; color: Theme.fade(Theme.bg, 0.35) }

    Column {
        anchors.centerIn: parent
        spacing: 28

        DotText {
            anchors.horizontalCenter: parent.horizontalCenter
            text: Qt.formatDateTime(clock.date, "HH:mm")
            dot: 16
            on: Theme.fg
        }
        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: {
                const t = clock.date.toLocaleDateString(Qt.locale(), "dddd, d MMMM");
                return t.charAt(0).toUpperCase() + t.slice(1);
            }
            color: Theme.fg
            font.family: Theme.fontUi
            font.pixelSize: 22
        }

        Item { width: 1; height: 40 }

        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: "󰀄  " + (Quickshell.env("USER") || "")
            color: Theme.fg
            font.family: Theme.fontUi
            font.pixelSize: 16
        }
        Rectangle {
            id: field
            anchors.horizontalCenter: parent.horizontalCenter
            width: 320
            height: 48
            radius: Math.min(Theme.radiusChip, 24)
            color: Theme.fade(Theme.bg, 0.6)
            border.width: 2
            border.color: view.failed ? Theme.red : input.activeFocus ? Theme.accent : Theme.fade(Theme.fg, 0.2)
            Behavior on border.color { ColorAnimation { duration: Theme.fast } }

            SequentialAnimation {
                id: shake
                NumberAnimation { target: field; property: "anchors.horizontalCenterOffset"; to: -14; duration: 50 }
                NumberAnimation { target: field; property: "anchors.horizontalCenterOffset"; to: 12; duration: 70 }
                NumberAnimation { target: field; property: "anchors.horizontalCenterOffset"; to: -8; duration: 70 }
                NumberAnimation { target: field; property: "anchors.horizontalCenterOffset"; to: 0; duration: 60 }
            }

            TextInput {
                id: input
                anchors { fill: parent; leftMargin: 20; rightMargin: 20 }
                verticalAlignment: TextInput.AlignVCenter
                horizontalAlignment: TextInput.AlignHCenter
                echoMode: TextInput.Password
                passwordCharacter: "●"
                color: Theme.fg
                font.family: Theme.fontUi
                font.pixelSize: 18
                focus: true
                enabled: !pam.active
                onTextChanged: if (text !== "") { view.failed = false; view.message = ""; }
                Keys.onReturnPressed: view.submit()
                Keys.onEnterPressed: view.submit()
                Keys.onEscapePressed: {
                    if (view.preview) view.unlocked();
                    else text = "";
                }
                Component.onCompleted: forceActiveFocus()
            }
            Text {
                anchors.centerIn: parent
                visible: input.text === ""
                text: "Пароль"
                color: Theme.muted
                font.family: Theme.fontUi
                font.pixelSize: 16
            }
        }
        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: view.message || (view.preview ? "Предпросмотр: экран не заблокирован, Esc — закрыть" : "")
            color: view.failed ? Theme.red : Theme.muted
            font.family: Theme.fontUi
            font.pixelSize: 14
        }
    }
}

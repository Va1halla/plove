#!/bin/sh
# Сгенерировано plove: погода для карточки «Погода» на рабочем столе.
# Печатает JSON {"icon", "temp", "desc"}: его читают карточки «Погода».
# $1 — город (пусто — wttr.in определит по IP).
city=$(printf '%s' "$1" | sed 's/ /+/g')
line=$(curl -sf --max-time 10 "https://wttr.in/${city}?format=%x|%t|%C&lang=ru")
if [ -z "$line" ]; then
    printf '{"icon": "󰖪", "temp": "—", "desc": "нет связи"}\n'
    exit 0
fi
symbol=${line%%|*}
rest=${line#*|}
temp=${rest%%|*}
desc=${rest#*|}
# %x — погода значками ASCII: переводим в значки Nerd Font.
case "$symbol" in
    o) icon="󰖙" ;;
    m) icon="󰖕" ;;
    mm | mmm) icon="󰖐" ;;
    =) icon="󰖑" ;;
    /) icon="󰖗" ;;
    //) icon="󰖖" ;;
    ///) icon="󰖖" ;;
    x | x/) icon="󰙿" ;;
    '*' | '**' | '*/*' | '*/' | '**/') icon="󰖘" ;;
    '/!/' | '!/' | '*!*') icon="󰖓" ;;
    *) icon="󰖐" ;;
esac
# Кавычки в описании сломали бы JSON.
desc=$(printf '%s' "$desc" | tr -d '"\\')
temp=$(printf '%s' "$temp" | tr -d '"\\')
printf '{"icon": "%s", "temp": "%s", "desc": "%s"}\n' "$icon" "$temp" "$desc"

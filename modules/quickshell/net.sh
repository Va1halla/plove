#!/bin/sh
# Состояние сети для панели Quickshell одной строкой:
#   wifi<TAB>имя сети<TAB>сила 0–100 | ethernet<TAB>интерфейс | off
# Без NetworkManager: интерфейс маршрута по умолчанию и /sys.
dev=$(ip route show default 2>/dev/null | awk '{for (i = 1; i < NF; i++) if ($i == "dev") { print $(i + 1); exit }}')
if [ -z "$dev" ]; then
    echo off
    exit 0
fi
if [ -d "/sys/class/net/$dev/wireless" ]; then
    ssid=""
    if command -v iw >/dev/null 2>&1; then
        ssid=$(iw dev "$dev" link 2>/dev/null | sed -n 's/^[[:space:]]*SSID: //p')
    fi
    [ -z "$ssid" ] && command -v nmcli >/dev/null 2>&1 &&
        ssid=$(nmcli -t -f active,ssid dev wifi 2>/dev/null | sed -n 's/^yes://p' | head -1)
    # Качество из /proc/net/wireless: 0–70 → проценты.
    q=$(awk -v d="$dev:" '$1 == d { print int($3 * 100 / 70) }' /proc/net/wireless 2>/dev/null)
    printf 'wifi\t%s\t%s\n' "${ssid:-$dev}" "${q:-0}"
else
    printf 'ethernet\t%s\n' "$dev"
fi

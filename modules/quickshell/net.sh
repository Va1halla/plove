#!/bin/sh
# Сеть для панели Quickshell одной строкой, поля через табуляцию:
#   вид (wifi, ethernet, off), имя сети или интерфейса, сила 0–100, IP, интерфейс.
#   net.sh wifi — сети Wi-Fi рядом (нужен NetworkManager): «*»,имя,сила,защита.
if [ "$1" = wifi ]; then
    command -v nmcli >/dev/null 2>&1 || exit 0
    nmcli -t -f IN-USE,SSID,SIGNAL,SECURITY dev wifi list 2>/dev/null | awk -F: '$2 != ""' | head -12
    exit 0
fi
dev=$(ip route show default 2>/dev/null | awk '{for (i = 1; i < NF; i++) if ($i == "dev") { print $(i + 1); exit }}')
if [ -z "$dev" ]; then
    echo off
    exit 0
fi
addr=$(ip -4 -o addr show dev "$dev" 2>/dev/null | awk '{print $4; exit}')
if [ -d "/sys/class/net/$dev/wireless" ]; then
    ssid=""
    if command -v iw >/dev/null 2>&1; then
        ssid=$(iw dev "$dev" link 2>/dev/null | sed -n 's/^[[:space:]]*SSID: //p')
    fi
    [ -z "$ssid" ] && command -v nmcli >/dev/null 2>&1 &&
        ssid=$(nmcli -t -f active,ssid dev wifi 2>/dev/null | sed -n 's/^yes://p' | head -1)
    # Качество из /proc/net/wireless: 0–70 → проценты.
    q=$(awk -v d="$dev:" '$1 == d { print int($3 * 100 / 70) }' /proc/net/wireless 2>/dev/null)
    printf 'wifi\t%s\t%s\t%s\t%s\n' "${ssid:-$dev}" "${q:-0}" "$addr" "$dev"
else
    printf 'ethernet\t%s\t0\t%s\t%s\n' "$dev" "$addr" "$dev"
fi

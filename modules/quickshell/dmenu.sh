#!/bin/sh
# dmenu на панели Quickshell: строки со stdin, выбранная — в stdout.
# Им пользуются меню выхода и история буфера обмена, как fuzzel --dmenu.
#   printf 'а\nб\n' | qs-dmenu.sh "Подсказка"
# Отмена (Esc, щелчок мимо) — пустой вывод и код 1.
dir="${XDG_RUNTIME_DIR:-/tmp}/plove-dmenu"
mkdir -p "$dir"
f="$dir/$$"
cat > "$f"
rm -f "$f.out"
if ! qs -c plove ipc call plove dmenu "$f" "${1:-}" >/dev/null 2>&1; then
    rm -f "$f"
    exit 1
fi
while [ ! -e "$f.out" ]; do
    sleep 0.05
done
out=$(cat "$f.out")
rm -f "$f" "$f.out"
[ -n "$out" ] || exit 1
printf '%s\n' "$out"

#!/bin/sh
# Перезагрузить панель после `plove apply`, когда все файлы уже записаны.
# Quickshell сама перезагружается на первом изменённом файле и в это
# время отвечает «Not ready to accept queries yet» (с кодом 0), поэтому
# повторяем, пока вызов не пройдёт молча. Не вышло или панели нет —
# запускаем заново.
pgrep -x qs >/dev/null || { setsid -f qs -c plove >/dev/null 2>&1; exit 0; }
i=0
while [ $i -lt 20 ]; do
    out=$(qs -c plove ipc call plove reload 2>&1) && [ -z "$out" ] && exit 0
    sleep 0.3
    i=$((i + 1))
done
pkill -x qs
setsid -f qs -c plove >/dev/null 2>&1

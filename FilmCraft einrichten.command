#!/bin/bash
cd "$(dirname "$0")" || exit 1
/bin/bash packaging/macos/dev/setup.sh
result=$?
if [ "$result" -eq 0 ]; then
  echo 'Fertig. FilmCraft Dev.app startet den Editor. FilmCraft aktualisieren.app baut Änderungen neu.'
else
  echo 'Die Einrichtung konnte nicht abgeschlossen werden. Details stehen oben.'
fi
read -r -p 'Zum Schließen Enter drücken. ' _answer
exit "$result"

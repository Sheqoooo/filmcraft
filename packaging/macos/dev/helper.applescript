on run argv
    set rootPath to item 1 of argv
    set logPath to rootPath & "/dist/dev/build.log"
    set choice to button returned of (display dialog "Hier wird nur FilmCraft Dev aktualisiert. Das normale FilmCraft kann geöffnet bleiben.\n\nFalls FilmCraft Dev läuft: Projekte dort vor dem Neustart speichern.\n\nLokal neu bauen übernimmt Änderungen in diesem Arbeitsordner. Fork laden holt zusätzlich Updates von Sheqoooo/filmcraft; lokale Änderungen werden dabei nicht überschrieben." with title "FilmCraft aktualisieren" buttons {"Abbrechen", "Fork laden", "Lokal neu bauen"} default button "Lokal neu bauen")
    if choice is "Abbrechen" then return
    set buildMode to "build"
    if choice is "Fork laden" then set buildMode to "update"
    display notification "Der Build läuft. Das kann beim ersten Mal mehrere Minuten dauern." with title "FilmCraft Dev"
    try
        with timeout of 86400 seconds
            do shell script "/bin/mkdir -p " & quoted form of (rootPath & "/dist/dev") & " && /bin/bash " & quoted form of (rootPath & "/packaging/macos/dev/update.sh") & " " & buildMode & " > " & quoted form of logPath & " 2>&1"
        end timeout
        set doneChoice to button returned of (display dialog "FilmCraft Dev wurde erfolgreich aktualisiert." with title "FilmCraft Dev" buttons {"Fertig", "FilmCraft starten"} default button "FilmCraft starten")
        if doneChoice is "FilmCraft starten" then do shell script "/usr/bin/open " & quoted form of (rootPath & "/FilmCraft Dev.app")
    on error errorText number errorNumber
        if errorNumber is -128 then return
        display dialog "Der Build konnte nicht abgeschlossen werden. Die vorherige App bleibt erhalten.\n\nDas Build-Protokoll wird geöffnet und zeigt den Grund." with title "FilmCraft Dev" buttons {"Protokoll öffnen"} default button 1
        do shell script "/usr/bin/open -t " & quoted form of logPath
    end try
end run

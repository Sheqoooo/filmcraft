# Clip in EffectCraft bearbeiten und manuell übernehmen

Die native **FilmCraft Dev**-Version dieses Forks öffnet einen Timeline-Clip in EffectCraft.
**Öffnen und Speichern starten keinen Render.** Erst der ausdrückliche Übernehmen-Befehl rendert
und ersetzt das Bild in FilmCraft.

## Benutzung ohne Terminal

1. Das FilmCraft-Projekt speichern und einen Video-Clip in der Timeline auswählen. Seine
   verknüpfte Audiospur darf mit ausgewählt sein.
2. Rechtsklick → **EffectCraft…**. FilmCraft erstellt eine lokale Arbeitskopie des Quellvideos,
   ohne es zu decodieren oder neu zu encodieren, und öffnet das vorbereitete EffectCraft-Projekt.
   In der Timeline bleibt das bisherige Video stehen.
3. In EffectCraft bearbeiten und **⌘S** speichern. Die Komposition heißt **FilmCraft Clip**;
   diesen Namen sowie den vorhandenen Projektnamen beibehalten.
4. Wenn du fertig bist: In FilmCraft den Clip auswählen → Rechtsklick →
   **Änderungen aus EffectCraft übernehmen**. Erst jetzt wird im Hintergrund gerendert.
5. Nach erfolgreichem Rendern aktualisiert sich derselbe Timeline-Clip. Zum Weiterbearbeiten
   wieder **EffectCraft…** wählen und später erneut ausdrücklich übernehmen.

Beide Befehle stehen auch im Menü **Clip**. Ein Fortschrittsfenster zeigt laufende Arbeit;
Fehlermeldungen bleiben dort bis zum Schließen sichtbar, auch bei ausgeblendeter Statusleiste.
Bei einem noch ungespeicherten FilmCraft-Projekt bietet das Fenster **Projekt speichern** an;
anschließend den EffectCraft-Befehl erneut aufrufen.

## Was erhalten bleibt

- Position, Länge und Kennung des Clips bleiben erhalten. Das Öffnen ändert nur die gespeicherte
  Verknüpfung; erst Übernehmen ersetzt die Videoquelle durch den fertigen Render.
- Die Originaldatei wird nicht überschrieben. Die bisherige Audiospur bleibt unverändert;
  Audioänderungen in EffectCraft werden nicht übernommen.
- Vorhandene FilmCraft-Effekte bleiben auf dem Clip und werden nach dem EffectCraft-Render
  angewendet. EffectCraft erhält das zugrunde liegende Video im verwendeten Quellausschnitt,
  keine gebackene Kopie der FilmCraft-Effekte.
- Das erste Übernehmen lässt sich als ein Schritt rückgängig machen. Das Anlegen der Verknüpfung
  ist ein eigener Schritt. Spätere Render desselben EffectCraft-Projekts aktualisieren eine externe
  Mediendatei; deren Versionsgeschichte verwaltet EffectCraft, nicht FilmCrafts Rückgängig-Funktion.

## Lokale Arbeitskopien und vorhandene Verknüpfungen

Arbeitsprojekte, Quellkopien und Render liegen standardmäßig unter
`~/Library/Application Support/FilmCraft Dev/EffectCraft/`. Das hält die Arbeitsmedien außerhalb
geschützter Dropbox-/Dokumente-Pfade, auf die die EffectCraft-App unter Umständen nicht zugreifen
kann. Abweichendes `FILMCRAFT_DATA_DIR` wird berücksichtigt. Das Kopieren großer Originaldateien
braucht Zeit und zusätzlichen Speicherplatz, erzeugt aber keinen Videorender.

Jeder Arbeitsordner enthält `clip.ecproj`, `link.json`, eine lokale Medienkopie und Protokolle;
`render.mov` entsteht erst beim Übernehmen. Diese Ordner aufbewahren und bei einem Umzug zusammen
mit dem FilmCraft-Projekt sichern. Verknüpfungen verwenden absolute Pfade. Die lokale Quellkopie
ist ein Snapshot; spätere Änderungen an der ursprünglichen Quelldatei werden nicht automatisch kopiert.

Beim erneuten Öffnen einer Verknüpfung aus der ersten Integrationsversion kopiert FilmCraft das
vorhandene EffectCraft-Projekt samt verknüpften Dateimedien in einen neuen lokalen Arbeitsordner.
Gespeicherte Bearbeitungen werden dabei erhalten, fehlende Dateimedien werden erneut eingebunden.
Das alte Projekt und sein bisheriger Render bleiben erhalten. Auch hierbei wird erst nach
**Änderungen aus EffectCraft übernehmen** gerendert. Nicht lesbare Quelldateien führen zu einer
Fehlermeldung; es wird dann kein leerer Ersatzclip eingesetzt.

## Grenzen dieser Entwicklungsversion

Unterstützt wird jeweils **ein normal abgespielter Video-Dateiclip**. Verschachtelte Sequenzen,
Generatoren, Multicam, Rückwärtswiedergabe, Standbilder und Geschwindigkeitsänderungen werden
abgewiesen. Die Vorbereitung ist auf 8192 Pixel je Seite, 240 fps und 24 Stunden Clipdauer begrenzt.
Nach dem Vorbereiten den Quellausschnitt des Originalclips bis zum ersten Übernehmen beibehalten;
Verschieben in der Timeline und Änderungen an FilmCraft-Effekten sind möglich.

Unter macOS wird EffectCraft mit dem Projektpfad als Startargument in einer eigenen Instanz
geöffnet. Damit wird die Home-Ansicht umgangen, die in EffectCraft 0.14.0 geöffnete Dokumente erst
nach dem Erstellen einer Komposition weiterverarbeitet. Ein erneuter Aufruf kann eine weitere
Instanz öffnen; vor paralleler Bearbeitung desselben Arbeitsprojekts die bisherige speichern und
schließen. Andere geöffnete Projekte werden nicht automatisch beendet.

Der Helfer rendert ProRes 4444 mit Alpha ohne Audio. Neue Ausgaben entstehen zunächst separat.
Erst ein erfolgreicher, lesbarer Render ersetzt die letzte Ausgabe. Ein Renderfehler lässt die
zuletzt funktionierende Datei erhalten. Speichert man während des Renderns erneut, wird die
veraltete Ausgabe nicht veröffentlicht; anschließend erneut übernehmen.

## Technische Anbindung und Lizenz

`clip.editInEffectCraft` liefert einen Plan oder eine vorhandene Verknüpfung; die Desktop-Oberfläche
und ihr Control-/MCP-Kanal führen die Vorbereitung aus. `effectcraft.stage` speichert eine pro
Timeline-Clip getrennte ausstehende Verbindung in der Projektmetadatenstruktur.
`clip.applyEffectCraft` startet im Desktop-Host den ausdrücklich angeforderten Render.
`effectcraft.attach` übernimmt das erste Ergebnis als Engine-Transaktion nach erneuter Zielprüfung;
`effectcraft.refresh` lädt spätere Ausgaben gezielt neu. `effectcraft.rebind` migriert die
Verknüpfung, ohne schon die Timeline-Quelle zu ersetzen.

Die Dev-App bündelt den offiziellen **EffectCraft CLI 0.14.0** mit festgeschriebener SHA-256-Prüfung
und beiden Lizenztexten unter `Contents/Resources/EffectCraft/`. Lizenz: **MIT OR Apache-2.0**;
Quelle und Autoren: [storytold/effectcraft](https://github.com/storytold/effectcraft).
Binärdateien werden nicht ins Repository aufgenommen; installierte Apps und Upstream-Code werden
nicht verändert. Entwicklungstests können `EFFECTCRAFT_CLI_PATH` verwenden. Der Renderer nutzt hier
die CPU; die interaktive EffectCraft-App kann weiterhin ihre eigene GPU-Beschleunigung verwenden.

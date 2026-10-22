# Clip in EffectCraft bearbeiten

Diese Funktion gehört zur nativen **FilmCraft Dev**-Version dieses Forks. Sie verbindet einen
Timeline-Clip mit einem EffectCraft-Projekt: **Speichern in EffectCraft** löst einen neuen Render
aus, den FilmCraft automatisch übernimmt. Währenddessen bleibt die letzte fertige Version sichtbar.

## Benutzung ohne Terminal

1. Das FilmCraft-Projekt speichern und einen Video-Clip in der Timeline auswählen. Die verknüpfte
   Audiospur darf mit ausgewählt sein.
2. Rechtsklick auf den Clip → **EffectCraft…** (auch im Menü **Clip** verfügbar).
3. FilmCraft bereitet den Clip im Hintergrund vor und öffnet anschließend die `.ecproj`-Datei in
   der auf dem Mac installierten EffectCraft-App. Falls macOS nach einer App fragt, EffectCraft wählen.
4. Den Clip in EffectCraft bearbeiten und mit **⌘S** speichern. Nach dem Rendern erscheint die
   Änderung automatisch in FilmCraft; für längeres Material kann das etwas dauern.
5. **EffectCraft…** auf demselben Clip öffnet später dasselbe EffectCraft-Projekt wieder.

Die Komposition **FilmCraft Clip** behält ihren Namen. Das Projekt unter seinem vorhandenen Namen
speichern; „Speichern unter“ an einen anderen Ort aktualisiert die bestehende Verbindung nicht.
FilmCraft muss für automatische Aktualisierungen geöffnet sein; beim erneuten Öffnen holt es
zwischenzeitlich gespeicherte Änderungen nach.

## Was erhalten bleibt

- Der Clip behält seine Position, Länge und Kennung in der Timeline. Nur seine Videoquelle wird
  durch den generierten Render ersetzt. Die Originaldatei wird nicht überschrieben.
- Die bisherige Audiospur bleibt unverändert. Audioänderungen in EffectCraft werden nicht übernommen.
- Vorhandene FilmCraft-Effekte bleiben auf dem Timeline-Clip und werden nach dem EffectCraft-Render
  angewendet. EffectCraft erhält das zugrunde liegende Video im verwendeten Quellausschnitt,
  keine fertig gebackene Kopie der FilmCraft-Effekte.
- Das erstmalige Verknüpfen lässt sich in FilmCraft als ein Schritt rückgängig machen. Spätere
  EffectCraft-Speicherstände aktualisieren eine externe Mediendatei; deren Versionsgeschichte
  verwaltet EffectCraft, nicht FilmCrafts Rückgängig-Funktion.

## Dateien und Grenzen dieser ersten Version

Neben dem gespeicherten FilmCraft-Projekt entsteht der Ordner `EffectCraft/`. Jede Verbindung
besitzt dort einen eigenen Unterordner mit `clip.ecproj`, `render.mov`, `link.json` und
Diagnoseprotokollen. Diese Dateien zusammen mit dem FilmCraft-Projekt aufbewahren. Die Verbindung
und die importierte Quelle verwenden absolute Dateipfade; nach einem Ordnerumzug müssen die Pfade
wiederhergestellt werden. Die ursprüngliche Quelldatei wird weiterhin benötigt.

Unterstützt wird jeweils **ein normal abgespielter Video-Dateiclip**. Verschachtelte Sequenzen,
Generatoren, Multicam, Rückwärtswiedergabe, Standbilder und Geschwindigkeitsänderungen werden mit
einer erklärenden Meldung abgewiesen. Die Vorbereitung ist auf maximal 8192 Pixel je Seite,
240 fps und 24 Stunden Clipdauer begrenzt. Das ist ein Entwicklungsstand, keine vollständige
Dynamic-Link-Implementierung. Änderungen werden erst nach dem Speichern und fertigem Render sichtbar.

Der Hintergrundprozess rendert ProRes 4444 mit Alpha ohne Audio. Neue Ausgaben entstehen zunächst
in einem separaten Arbeitsordner. Erst ein erfolgreicher, lesbarer Render ersetzt `render.mov`.
Fehler erscheinen in FilmCraft und im `render.log`; die letzte gute Datei bleibt bei Renderfehlern
erhalten. Bei einem erneuten Speichern oder erneuten Aufrufen von **EffectCraft…** wird es wieder
versucht. Einen Render nicht gleichzeitig von mehreren FilmCraft-Instanzen bearbeiten lassen.

## Technische Anbindung und Lizenz

`clip.editInEffectCraft` liefert im Engine- oder Headless-Betrieb einen vorbereitenden Plan oder
die vorhandene Verbindung. In der Desktop-Oberfläche und ihrem Control-/MCP-Kanal startet derselbe
Befehl den Hintergrundjob. `effectcraft.attach` übernimmt den fertigen Render als eine Engine-
Transaktion und prüft zuvor, dass Projekt, Clip und Sperrsituation noch zum Plan passen.
`effectcraft.refresh` lädt fertig gerenderte, im aktuellen Projekt verknüpfte Ausgaben gezielt neu,
unabhängig von allgemeinen Dateistempelscans.

Die Dev-App bündelt den unveränderten **EffectCraft CLI 0.14.0** als signierten Helfer.
Der Builder lädt das offizielle Universal-macOS-Archiv einmalig, prüft dessen festgeschriebenen
SHA-256 und legt beide Lizenztexte im Bundle unter `Contents/Resources/EffectCraft/` ab.
Lizenz: **MIT OR Apache-2.0**; Quelle und Autoren: [storytold/effectcraft](https://github.com/storytold/effectcraft).
Es werden keine EffectCraft-Binärdateien ins Git-Repository aufgenommen und keine Änderungen an
EffectCraft oder FilmCraft-Upstream vorgenommen. Für Entwicklungstests kann
`EFFECTCRAFT_CLI_PATH` auf eine lokale CLI-Datei zeigen. Der Renderer verwendet hier die CPU.

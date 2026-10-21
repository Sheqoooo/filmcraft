# FilmCraft Dev auf Apple Silicon

Diese Einrichtung baut **Sheqoooo/filmcraft** als native ARM64-App. Der normale Start braucht
keine laufenden Entwicklungsdienste und keine Terminalbefehle.

## Benutzung

- **FilmCraft Dev.app** im Hauptordner startet den zuletzt erfolgreich gebauten Editor.
  Das türkisfarbene Eulen-Symbol mit DEV-Markierung unterscheidet sie im Dock vom Original.
  Die App lässt sich ins Dock ziehen und auch an einen anderen Ort kopieren.
- **FilmCraft aktualisieren.app** im Hauptordner bietet zwei Möglichkeiten:
  - **Lokal neu bauen** übernimmt den aktuellen Code dieses Arbeitsordners, einschließlich
    uncommitteter Änderungen. Danach im Erfolgsdialog **FilmCraft starten** wählen.
  - **Fork laden** lädt `origin/main` von `https://github.com/Sheqoooo/filmcraft.git` und baut
    danach neu. Das geht nur auf `main`, mit sauberem Arbeitsordner und per Fast-forward.
    Bei lokalen Änderungen oder auseinanderlaufenden Branches stoppt der Helfer mit einem
    erklärenden Protokoll. Er verwirft keine Änderungen und pusht nichts.

Das normale FilmCraft kann während Start und Aktualisierung geöffnet bleiben. Vor einem Neustart
der Dev-Version deren eigene Projekte speichern. Eine bereits laufende Dev-Version verwendet
weiter den alten Code; neue Änderungen erscheinen nach ihrem nächsten Start.
Der Helfer zeigt während des Builds eine Mitteilung und am Ende einen Erfolgs- oder Fehlerdialog.
Er kann währenddessen einige Minuten beschäftigt erscheinen. Details stehen in
`dist/dev/build.log`; die Datei kann jederzeit im Finder geöffnet werden.

Der Arbeitsordner muss an seinem Ort bleiben, solange der daraus erzeugte Helfer benutzt wird.
Nach einem Umzug den Helfer mit **FilmCraft einrichten.command** neu erzeugen.
Eine Kopie von FilmCraft Dev.app wird durch den Helfer nicht aktualisiert: Er ersetzt ausschließlich
die App im Arbeitsordner. Daher für das Dock und Schreibtisch-Verknüpfungen die Original-App nutzen.

## Erstinstallation / Wiederherstellung

**FilmCraft einrichten.command** im Hauptordner doppelklicken. Das Einrichtungsfenster führt
alle Schritte automatisch aus; es müssen keine Befehle eingegeben werden. Benötigt werden
Apple Silicon, Xcode Command Line Tools und Internet für den ersten Build. Fehlt Rust,
installiert die Einrichtung Rust 1.95.0 projektlokal unter `.dev-tools/`, ohne die Shell-Einstellungen
zu ändern. Eine bereits vorhandene Rust-Installation muss mindestens Version 1.95 unterstützen.
Nachher bleiben App-Start und Aktualisierung über die beiden `.app`-Dateien verfügbar.

Die erste Kompilierung lädt Rust-Abhängigkeiten; spätere Builds nutzen den lokalen Cache.
Der `dev-app`-Build ist optimiert und nutzt inkrementelle Kompilierung statt des langsameren
Release-Linkens mit LTO. Die optionalen Whisper-/Kokoro-Funktionen sind wie im vorhandenen
macOS-Packaging nicht eingeschaltet. System- und gebündelte Schriften funktionieren ohne den
optionalen separaten craft-fonts-Checkout. FFmpeg wird weder benötigt noch eingebaut.

## Daten und Rückkehr zur vorigen Version

Die Entwicklerversion hat die Bundle-ID `com.sheqoooo.filmcraft.dev` und verwendet standardmäßig
`~/Library/Application Support/FilmCraft Dev` für Einstellungen, Autosaves und Logs. Der
Starthelfer respektiert ein ausdrücklich gesetztes `FILMCRAFT_DATA_DIR` und übergibt Dateipfade
und weitere Argumente unverändert an die native App. Projektdateien behalten ihr normales Format.

Der Builder erstellt und prüft zunächst ein vollständiges neues Bundle. Erst danach ersetzt er
`FilmCraft Dev.app`. Frühere Bundles bleiben unter `dist/dev/FilmCraft Dev.previous.…app` erhalten.
Sie können im Finder gestartet oder zurückkopiert werden. Die alten Versionen werden nicht
automatisch gelöscht. Nach einem abgebrochenen Build kann `dist/dev/build.lock` zurückbleiben;
nur wenn wirklich kein Build mehr läuft, diesen leeren Ordner im Finder löschen und neu versuchen.

Der erste lokale Build ist ad-hoc signiert und für den eigenen Mac gedacht. Er hat keine Apple-
Notarisierung. Aus dem Internet geladene CI-Builds können deshalb beim ersten Öffnen eine
Freigabe in macOS unter Datenschutz & Sicherheit erfordern; Gatekeeper wird nicht global deaktiviert.
Upstream-Markengrafiken werden in diesem unabhängigen Fork nicht ins Programm eingebettet.

## Automatische GitHub-Builds

`.github/workflows/macos-dev.yml` verwendet einen ARM64-Runner (`macos-14`) und Rust 1.95.0.
Nach Pushes auf `main` oder einem manuellen Start baut es dieselbe App. Unter **Actions →
macOS ARM64 developer app → erfolgreicher Lauf → Artifacts** gibt es ein ZIP samt SHA-256-Prüfsumme.
Die Aufbewahrung ist sieben Tage. Das innere ZIP erhält die App-Struktur, Signaturen und
Ausführungsrechte. Es enthält eine eigenständig startbare App, aber keinen an einen fremden
Arbeitsordner gebundenen Aktualisierungshelfer.

Der Workflow läuft ausdrücklich nur in `Sheqoooo/filmcraft`, benötigt keine Apple-Zertifikate
und hat nur Lesezugriff auf den Repository-Inhalt. Er veröffentlicht keine Releases und ändert
keinen Upstream. Die lokalen Scripts lesen oder schreiben ebenfalls ausschließlich den Fork.

## Für künftige Code-Arbeit

- `packaging/macos/dev/build.sh`: ARM64-Build, Bundle, Prüfung und Austausch.
- `packaging/macos/dev/install-helper.sh`: erstellt den GUI-Helfer für den aktuellen Checkout-Pfad.
- `packaging/macos/dev/update.sh`: lokaler Neubau oder geschütztes Fork-Update.
- `packaging/macos/dev/check.sh`: sämtliche Workspace-Prüfungen; für die zahlreichen Testdateien
  wird nur das teure LTO-Linken deaktiviert, alle Tests und die Rust-Optimierung bleiben aktiv.
- `packaging/macos/package.sh`: unveränderter bisheriger Release-Pfad mit DMG/CLI und optionaler
  Signierung/Notarisierung; der Entwicklungs-Workflow benötigt diesen Pfad nicht.

Akzeptanz: App aus Finder/Launch Services starten, `ui.inspect` und `ui.screenshot` ausführen,
ARM64 und Bundle-Signatur prüfen, einen erneuten Build durchführen und die Ablehnung eines
Fork-Updates bei lokalen Änderungen nachweisen. Zusätzlich gelten die Workspace-Prüfungen
`cargo xtask ci`; deren tatsächliche Ergebnisse sind im Abschlussbericht zu nennen.

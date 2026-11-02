# FilmCraft Dev: Experiment, Anpassungen und Upstream-Update

Stand: **30. Oktober 2026**. Dieses Dokument beschreibt den unabhängigen Fork
**Sheqoooo/filmcraft** und ergänzt die allgemeinen FilmCraft-Unterlagen.

## Zweck des Forks

Dieser Fork wurde als **Experiment** benutzt, um bestehende Funktionen zu reparieren und neue
Funktionen dazuzupatchen. Ausgangspunkt waren konkrete Probleme beim Schneiden sowie Versuche
mit einer eigenständigen macOS-Entwicklerversion und der Verbindung zu EffectCraft.
Die Änderungen sind Entwicklungsstände; sie sind keine Freigabe als stabile Version und keine
Behauptung, dass diese Funktionen bereits im ursprünglichen FilmCraft enthalten sind.

## Ausgangspunkt und Aktualisierung

Die ersten Anpassungen DEV.1–DEV.10 bauen auf `fb4cbda4` des ursprünglichen Projekts auf.
Vor dieser Dokumentation wurde der aktuelle `main` von **storytold/filmcraft** abgerufen:
[`fed0f42f`](https://github.com/storytold/filmcraft/commit/fed0f42f),
„UPD.1: honor app pretend versions in update checks (#3668)“.
Der Update-Branch führt dessen 501 zusätzliche Commits mit dem Fork-Stand
[`06115e56`](https://github.com/Sheqoooo/filmcraft/commit/06115e56) zusammen.
„Aktuell“ bezeichnet diesen am 30. Oktober abgerufenen Stand, keine fortlaufende Synchronisierung.

Upstream hat unter anderem Engine-Helfer, App-Lebenszyklus und Timeline in kleinere Module
aufgeteilt. Die weiterhin relevanten lokalen Erweiterungen werden in diese Struktur übertragen:
Effekt-Zwischenablage, Monitorumschaltung, Timeline-Griffe und EffectCraft-Menüs bleiben Teil des
Experiments. Die frühere Link-/Unlink-Anpassung wurde durch den neueren Upstream-Fix ersetzt.
Upstream bleibt unverändert; der Pull Request richtet sich an `Sheqoooo/filmcraft:main`.

## Anpassungen im Überblick

Die Commit-Verweise dokumentieren die Einführung. Bei späteren Änderungen gilt das hier
beschriebene aktuelle Verhalten. Historische Prüfergebnisse sind im
[Validierungsprotokoll](../packaging/macos/dev/VALIDATION.md) festgehalten.

| Änderung | Verhalten und Zweck | Einführung |
| --- | --- | --- |
| Native Mac-Entwicklerversion | Einrichtung per Doppelklick, optimiertes inkrementelles Build-Profil, getrennte Einstellungen und Wiederherstellungsdaten, GUI-Helfer für Neubau oder geschütztes Fast-forward-Update. Frühere App-Bundles bleiben erhalten. | [DEV.1](https://github.com/Sheqoooo/filmcraft/commit/be544150), [Plattformprüfung](https://github.com/Sheqoooo/filmcraft/commit/54cfd133) |
| Eigene Kennzeichnung | Türkises DEV-Dock-Symbol mit dokumentierter Herkunft; Textkennung „FilmCraft Dev“ anstelle eingebetteter Upstream-Markengrafiken. | [DEV.2](https://github.com/Sheqoooo/filmcraft/commit/c82de0f8) |
| Clips verknüpfen | Historisch: Menü an die damalige Engine-Regel angeglichen. Inzwischen durch Upstream #2011 ersetzt: eine gemischte Auswahl wird aufgehoben, sobald ein Clip verknüpft ist. Der lokale Menü-Override entfällt; die deutsch-/englischsprachigen Tests prüfen die neue Regel samt Undo. | [DEV.3](https://github.com/Sheqoooo/filmcraft/commit/02008048), [Upstream #2011](https://github.com/storytold/filmcraft/pull/2011) |
| Ganze Effekte kopieren | Ausgewählten Effekt samt Einstellungen, Masken und Animation auf kompatible Zielclips übertragen, ohne Timeline-Medien zu duplizieren; Rückgängig/Wiederholen bleibt möglich. | [DEV.4](https://github.com/Sheqoooo/filmcraft/commit/fc4c184a) |
| Einzelne Parameter kopieren | Parameterlabel auswählen und einschließlich Animation übertragen. Andere Parameter bleiben erhalten; veraltete Inspektorauswahlen sollen keine alten Timeline-Clips einfügen. | [DEV.5](https://github.com/Sheqoooo/filmcraft/commit/464ab35a) |
| In-/Out-Griffe | Sichtbare Timeline-Griffe mit Vorschau, Frame-Ausrichtung, Snapping an Clipkanten, einem Undo-Schritt und Abbruch per Esc. | [DEV.6](https://github.com/Sheqoooo/filmcraft/commit/931a5576) |
| Gemeinsame Monitor-Tabs | Bei Wiedergabestart den zugehörigen Quell- oder Programmmonitor in der gemeinsamen Tab-Gruppe anzeigen und den Tastaturfokus erhalten. | [DEV.7](https://github.com/Sheqoooo/filmcraft/commit/6afe4d54) |
| Monitor-Scrubbing | Linker Mausdruck bewegt den Playhead sofort; gehaltenes Ziehen führt ihn weiter. Option/Alt-Ziehen verschiebt den ganzen markierten Bereich. | [DEV.8](https://github.com/Sheqoooo/filmcraft/commit/b95c55be) |
| EffectCraft-Anbindung | Video-Dateiclip in einem eigenen Arbeitsprojekt öffnen und das gerenderte Ergebnis auf demselben Timeline-Clip verwenden. Gepinnter CLI-Helfer mit Prüfsumme und Lizenztexten im Dev-Bundle. | [DEV.9](https://github.com/Sheqoooo/filmcraft/commit/75aff038), [Builder-Dateirechte](https://github.com/Sheqoooo/filmcraft/commit/c86d80ff) |
| Manuelles EffectCraft-Übernehmen | Öffnen erstellt eine lokale Quellkopie; Speichern rendert nicht. Erst „Änderungen aus EffectCraft übernehmen“ rendert und aktualisiert. Bestehende Verknüpfungen lassen sich migrieren. | [DEV.10](https://github.com/Sheqoooo/filmcraft/commit/1e742bd3) |
| Downloadbare Entwicklungsstände | Fork-eigener ARM64-Workflow mit ZIP, SHA-256 und sieben Tagen Artifact-Aufbewahrung. Ein ausdrücklicher Veröffentlichungsauftrag erzeugt zusätzlich einen Entwicklungs-Prerelease. | [DEV.RELEASE](https://github.com/Sheqoooo/filmcraft/commit/06115e56) |
| Dokumentation und Entwicklungspolitik | README-Übersicht, Anleitungen und tatsächliche Prüfergebnisse; Entwicklungs-Snapshots dürfen mit dokumentierten offenen Checks gespeichert werden. Asset-, Lizenz- und Absturzschutzregeln gelten weiter. | [DEV.1](https://github.com/Sheqoooo/filmcraft/commit/be544150), [DOC.1](https://github.com/Sheqoooo/filmcraft/commit/acfe3d39) |

## EffectCraft: aktueller Ablauf und Grenzen

Die automatische Aktualisierung beim Speichern aus DEV.9 wurde durch DEV.10 ersetzt.
**Öffnen → bearbeiten → speichern → ausdrücklich übernehmen** ist der aktuelle Ablauf.
Timeline-Position, Clip-Länge und bestehendes Audio bleiben erhalten; FilmCraft-Effekte werden
weiterhin nach dem EffectCraft-Ergebnis angewendet. Ein fehlgeschlagener Render behält die letzte
funktionierende Ausgabe. Das erste Anhängen ist rückgängig machbar; spätere externe Renderdateien
haben keine eigene Versionsgeschichte in FilmCrafts Undo.

Unterstützt werden normal abgespielte Video-Dateiclips, keine verschachtelten Sequenzen,
Multicam oder Geschwindigkeitsänderungen. Arbeitskopien benötigen zusätzlichen Speicher und
verwenden absolute Pfade. Die EffectCraft-Oberfläche wird separat installiert; der gebündelte
CLI-Helfer rendert hier auf der CPU. Ein synthetischer Roundtrip ersetzt keinen Test mit allen
Effekten, Formaten oder den zuvor nicht zugänglichen Dropbox-Medien des Benutzers.
Details: [EffectCraft-Anleitung](effectcraft.md).

## Validierung und offene Punkte

Die ursprünglichen DEV-Commits dokumentieren gezielte Engine-/UI-Regressionstests, Clippy,
native ARM64-Builds und Signaturprüfungen. Das sind historische Ergebnisse vor dem Upstream-Update,
keine automatisch gültige Freigabe des zusammengeführten Standes. Aktuelle Ergebnisse des Updates
werden separat im [Validierungsprotokoll](../packaging/macos/dev/VALIDATION.md) aufgeführt.

Der frühere vollständige Workspace-Testlauf stoppte im VFR-Exporttest
`phone_vfr_clip_extract_gain_title_to_h264_mp4`: Pixelabweichung 19,04 bei Grenze 6,0;
bis dahin 417 Tests bestanden, acht ignoriert. Die Ursache war nicht gegen die damalige Basis
verifiziert. Der damalige WASM-Check blieb wegen blockierter Downloads unvollständig.
Diese Angaben werden durch das Update weder als behoben noch als erneut reproduziert ausgegeben.
Kein Test wurde zur Umgehung dieser Grenzen abgeschwächt.

Die macOS-Download-App ist für Apple Silicon, ad-hoc signiert und nicht notarisiert.
Der Build-Workflow prüft Paket und Versionsaufruf, führt aber weder alle Workspace-Tests noch
eine interaktive GUI-Abnahme aus. Für Installation und Build-Verhalten siehe
[Mac-Anleitung](../packaging/macos/dev/README.md).

## Vorschläge für spätere Upstream-Pull-Requests

### Prüfung gegen den neuen Upstream-Stand

Die folgenden Verhaltensprüfungen wurden auf einem separaten Checkout von `fed0f42f` ausgeführt.
Dabei blieb der Produktcode unverändert; nur die benötigten Tests wurden übertragen und an die
öffentlichen aktuellen Docking-Typen angepasst. Direkte Prüfungen von ausschließlich im Fork
vorhandenen Zustandsfeldern wurden für den Vergleich entfernt; Bedienung und sichtbares Ergebnis
blieben die Prüfkriterien. Ein fehlgeschlagener Test auf eine experimentelle neue Funktion bedeutet
hier „dort noch nicht vorhanden“, nicht automatisch einen Fehler nach Upstream-Spezifikation.

| Anpassung | Ergebnis im unveränderten Upstream | Konsequenz |
| --- | --- | --- |
| DEV.3 Link-/Unlink | Vier UI-Tests mit der neuen Regel bestehen, einschließlich gemischter Auswahl, Deutsch/Englisch und Undo. [Commit 869e4070](https://github.com/storytold/filmcraft/commit/869e4070) hat die Engine auf „mindestens ein Clip verknüpft“ umgestellt. | Bereits upstream behoben; lokaler Override entfernt, kein weiterer Fix-PR. |
| DEV.4 Effektkopie / DEV.5 Parameterkopie | Vier Bedienungstests scheitern: Transform-Kopie, Motion über natives Menü, einzelner Scale-Parameter und Schutz vor alter Clip-Zwischenablage. | Noch nicht in diesem Upstream-Stand gelöst; als zusammenhängende Basis und spätere Erweiterung vorschlagen. |
| DEV.6 Timeline-Griffe | Alle vier Tests fehlen funktional, weil Upstream die betreffenden Griffe nicht anbietet. | Weiterhin eine neue Funktion, als eigener Vorschlag behandeln. |
| DEV.7 Monitorumschaltung | Drei von vier Tests scheitern: Timeline-Wiedergabe, schwebende gemeinsame Monitorgruppe und maximierte angedockte Monitorgruppe. Getrennte Gruppen bleiben korrekt unverändert. | Problem besteht weiterhin; lokale Anpassung auf das gemeinsame Docking-Modell übertragen. |
| DEV.8 Scrubbing | Beide gezielten Tests für bewegten Mausdruck und Wiedergabestopp beim Drücken scheitern. | Problem besteht weiterhin; lokaler Fix bleibt relevant. |

Die neue Docking-Version maximiert nur angedockte Panels. Der frühere Test mit einer gleichzeitig
schwebenden und maximierten Monitorgruppe wurde daher in getrennte Tests für schwebende Gruppen
und gültig maximierte angedockte Gruppen aufgeteilt. Die Auswahl- und Undo-Prüfungen von DEV.3
bleiben bestehen, prüfen jetzt aber bewusst die neue Upstream-Regel statt die überholte Fork-Regel.

### Aufteilung und Umgang mit dem Projekt

Der Update-/Dokumentations-PR bleibt im Fork. Für das ursprüngliche Projekt sollten Änderungen
einzeln vorgeschlagen werden, nach Prüfung der dortigen Beitragsregeln sowie vorhandener Issues
und PRs. Der Vergleich mit `fed0f42f` zeigt noch lokale Unterschiede; eine Suche nach bereits
offenen Vorschlägen gehört zur Vorbereitung jedes einzelnen Upstream-PRs.

| Möglicher PR | Abgrenzung |
| --- | --- |
| Sofortiges Monitor-Scrubbing | Mausdruck und gehaltenes Ziehen einschließlich Option/Alt-Bereichsbewegung und Regressionstests. |
| Wiedergabe in gemeinsamen Monitor-Tabs | Nur Aktivierung des richtigen Tabs, Fokuserhalt und Tests mit dem aktuellen gemeinsamen Docking-Modell. |
| Effekt-Zwischenablage | Engine und UI zusammen reviewbar halten; einzelne Parameter gegebenenfalls als nachfolgenden PR ergänzen. Kollisionen mit Clip- und Caption-Copy abdecken. |
| Timeline-In-/Out-Griffe | Als eigenständige Funktion mit Bedienbeschreibung, Undo-/Abbruch-Tests und Herkunft der gezeichneten Griffe. |

Experimentelle EffectCraft-Integration und Fork-spezifisches Packaging nicht in diese Fix-PRs
mischen. Vor einem größeren Funktionsvorschlag zunächst die gewünschte Richtung mit den
Maintainern abstimmen. PR-Beschreibungen nennen Reproduktion, Vorher/Nachher, tatsächlich
ausgeführte Tests und offene Grenzen. Keine pauschale Stabilitätszusage und kein ungefragtes
Zuweisen von Reviewern oder wiederholtes Nachfragen. Dieses Dokument eröffnet keine Upstream-PRs.
Die bereits durch Upstream #2011 ersetzte Link-/Unlink-Anpassung ist kein PR-Kandidat mehr.

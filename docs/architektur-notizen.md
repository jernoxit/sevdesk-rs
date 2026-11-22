# Architektur-Notizen

Regeln und verworfene Optionen, die unabhängig von einer einzelnen Stufe
gelten. Eine verworfene Option steht hier mit Grund, damit sie niemand
erneut vorschlägt.

## Verworfene Optionen

### Client aus der OpenAPI-Spec generieren (2026-11-22)

Verworfen, auch als „Generierung plus Patch-Datei“. Der Abgleich vom
2026-11-22 zeigte: Keine der zwölf gemessenen Abweichungen (Tippfehler in
Filtern, falsche Typen wie Schalter als `boolean`, fehlende Felder und
Endpunkte, falsche Enum-Werte) ist in der Spec korrigiert. Ein Patch müsste
fast jedes Schema anfassen; Geld in Cent, tolerante Enums und gemessenes
Verhalten (Fehler in HTTP 200, Nummernersatz beim `sendBy`, Festschreib-Sperren)
kann ein Patch nicht abbilden. Stattdessen: Aufzeichnungen, lesende Erhebung,
Vollständigkeitswächter und der Spec-Wächter (`just spec-check`), der neue
Felder, Werte und Endpunkte nur als Hinweis meldet.

### `rust_decimal` für Mengen und Stückpreise (2026-11-22)

Verworfen zugunsten des eigenen exakten Typs `Decimal`. Gemessen: Über die
eigene Serde-Anbindung liest `rust_decimal` JSON-Zahlen ohne das globale
`serde_json`-Feature `arbitrary_precision` über `f64` (ab etwa 17 Stellen still
gekürzt) und rundet Strings mit mehr als 28 Stellen still. Eine fremde
Bibliothek im öffentlichen API bindet die Crate an deren Major-Versionen und
bietet Arithmetik an, die die Crate bewusst nicht anbietet.

### `taxSet` an Rechnung und Gutschrift (2026-11-22)

Nicht angeboten. Einzeln per Update gemessen wurde es gespeichert; beim Anlegen
zusammen mit dem Pflichtfeld `taxRule` lehnt sevDesk ab (HTTP 422, Code 255,
„TaxSet is not allowed in new bookkeeping“).

### `Decode` als Box (2026-11-22)

Verworfen. `SevdeskError::Decode` bleibt eine Struct-Variante mit direkten
Feldern; mit `path: Box<str>` statt `String` bleibt der Fehler bei 48 Byte,
ohne die Details hinter eine Box zu legen.

### `#[non_exhaustive]` an Query- und Update-Typen (2026-11-22)

Verworfen. Sie werden mit `..Default::default()` gebaut; `#[non_exhaustive]`
würde das bei Verbrauchern brechen. Entwürfe ohne `Default` teilen sich
stattdessen in Pflichtfelder und ein `#[non_exhaustive]`-`options`-Struct.

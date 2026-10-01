# sevdesk-rs — Projektanweisungen

sevdesk-rs ist ein typisierter Rust-Client (Crate `sevdesk`) für die sevDesk
REST API v1, herausgelöst aus einem Dienst, der ihn nutzt. Die Regeln unten
stammen aus dessen Projektanweisungen; übernommen ist, was für eine reine
Client-Bibliothek gilt.
Was gemessen von der Spezifikation abweicht, steht im [`README.md`](README.md).

## Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

## Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

## Goal-Driven Execution

**Define success criteria. Loop until verified.** Strong criteria let you loop
independently; weak ones ("make it work") require constant clarification.

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan — one line per step, as
`1. [Step] → verify: [check]`.

## Arbeitsweise: Opus koordiniert, Sonnet implementiert, Fable berät

**Gilt nur für die Hauptsession.** Wenn du als Subagent läufst, gilt für dich nur die Eskalationsregel unten.

### Rolle der Hauptsession
- Du planst, zerlegst, delegierst und prüfst. **Jede Implementierung geht an einen Subagent, auch eine triviale** (ein klarer Fix, wenige Zeilen) — Code, `justfile`, Konfiguration (Projektinhaber, 2026-09-30). Selbst machst du nur, was keine Implementierung ist: lesen, prüfen, Checks fahren, Messungen, die eine Frage beantworten, committen, und Entscheidungen des Projektinhabers nachtragen.
- Bei allem Größeren erstellst du zuerst einen Plan. Dann zerlegst du ihn in abgeschlossene Schritte und delegierst jeden an den general-purpose-Subagent mit `model: "sonnet"`. Unabhängige Schritte startest du parallel.
- Jeder Delegations-Prompt muss für sich allein verständlich sein, denn der Subagent sieht nur diesen Prompt. Er enthält:
  - das Ziel
  - die betroffenen Dateien und Module
  - die relevanten Entscheidungen aus dem Plan
  - die Constraints
  - ein prüfbares Akzeptanzkriterium, z. B. `just test`, `just lint` und `just doc` ohne Fehler
- Wenn ein MCP-Server für den Schritt relevant ist, nennst du ihn im Prompt. Subagents haben dieselben Tools wie du.
- Übernimm die Eskalationsregel unten wörtlich in jeden Delegations-Prompt.
- Prüfe jedes Subagent-Ergebnis selbst (Diff lesen, Tests), bevor du den nächsten Schritt startest.

### Eskalation (Sonnet → Opus → Fable)
- Meldet ein Subagent eine Blockade, löst du sie zuerst selbst.
- Erst wenn du selbst unsicher bist, konsultierst du den Advisor.
- Danach setzt du den Subagent mit der Antwort fort, per SendMessage, falls verfügbar. Sonst startest du einen neuen Subagent mit dem vollständigen Kontext.
- Den Advisor konsultierst du außerdem, bevor du dich auf eine Architektur festlegst und bevor du eine größere Aufgabe als erledigt meldest.

### Eskalationsregel für Subagents
Wenn du blockiert bist, hör auf, konsultiere nicht den Advisor und melde an den Koordinator zurück. Blockiert bist du zum Beispiel, wenn ein Test aus unklarem Grund fehlschlägt, der Plan nicht zum Code passt oder eine Designentscheidung nötig ist.
Deine Rückmeldung enthält: was du versucht hast, woran es hängt und welche Optionen du siehst.

## Arbeit in einem Schwester-Repo geht an dessen Session

sevdesk-rs und die Repos, die es nutzen, haben je eine eigene Claude-Session,
benannt nach dem Repo (`/rename <repo>`). **Muss eine Änderung in einem anderen
dieser Repos geschehen, macht sie dessen Session, nicht du.**

1. **Erst nachsehen:** `ListAgents` aufrufen und prüfen, ob eine Session mit dem
   Namen des Ziel-Repos läuft.
2. **Läuft sie, delegierst du:** per `SendMessage` an diesen Namen, mit einem
   Auftrag, der für sich allein verständlich ist: Ziel, Grund, betroffene
   Stellen, was schon entschieden ist, Akzeptanzkriterium. Sie plant und baut
   nach ihren eigenen Regeln; du schreibst ihr nicht vor, wie. Auf ihre Antwort
   wartest du ohne Abfrageschleife (`notify_when_idle` statt Nachfragen).
3. **Läuft keine, fragst du den Projektinhaber**, bevor du selbst etwas im
   fremden Repo änderst. Erst nach seinem Ja arbeitest du dort, nach den Regeln
   jenes Repos (dessen `CLAUDE.md` zuerst lesen).

**Lesen ist immer erlaubt**, Schreiben nicht. Ein Subagent fasst ein fremdes Repo
nie schreibend an. Was dir in deiner Session verwehrt ist, verlangst du
keiner anderen Session; das wäre Umgehung der Freigaben.

## Bestandsdeckel: eine Zahl, die nur sinken darf

Regeln mit Altbestand folgen alle demselben Muster: **sie gelten ab sofort für
Neues und für alles, was ohnehin angefasst wird**, und den Rest deckelt ein
Hygiene-Test mit einer benannten Zahl bzw. Liste, verglichen per **exakter
Gleichheit, nie `<=`**. Ein Zuwachs scheitert — der Zweck —, ein Rückgang
ebenso, mit der neuen Zahl in der Meldung, damit sie im selben Commit gesenkt
wird; sonst schüfe jedes Aufräumen stillschweigend Platz für Neues. Steht die
Zahl auf null, ist der Deckel ein Verbot.

## Dateizuschnitt: von Anfang an trennen, nicht später zerlegen

**Ist beim Entwurf absehbar, dass ein Modul >500 Zeilen wird, entsteht es sofort
als Verzeichnis-Modul mit getrennten Dateien** — „später aufteilen" kommt selten,
und bis dahin ist die Datei unübersichtlich und jeder Diff darauf breit. Zwei
Signale, dass der Schnitt schon fällig ist:

- **Abschnittsbanner.** Wer `// ==== Vouchers ====`, `// ==== Invoices ====`
  schreibt, hat die Schnittlinien erkannt — dann sind es Dateien, keine
  Kommentare.
- **Die Fassade wächst.** Eine `mod.rs` ist Doc + `mod`-Deklarationen +
  Re-Exports und sonst nichts; sobald dort Logik landet, ist der Schnitt
  überfällig.

**Trait und Implementierungen liegen getrennt**, jede Implementierung in ihrer
eigenen Datei.

Das ist eine **Layout**-Regel: sie sagt, WO ein Trait liegt, wenn es ihn gibt. Ob
es ihn geben soll, entscheidet einer von vier Gründen; „sieht sauberer aus" ist
keiner davon:

1. **Austausch.** Eine benennbare zweite Implementierung, die eine zugesagte
   oder absehbar nötige Fähigkeit ist; sie muss noch nicht existieren.
2. **Abhängigkeitsumkehr.** Ein Modul darf ein anderes nicht nennen; dann
   erklärt der Trait, was das *untere* Modul braucht, und fordert **nur, was
   diese eine Kante braucht**.
3. **Typlöschung.** Verschiedene Typen gleicher Form hinter einem Nachschlagen
   zur Laufzeit.
4. **Die Grenze nach außen.** Ein fremder Dienst hat eine eigene Form, die nicht
   durchsickern darf. In dieser Crate ist sevDesk selbst das Fremde: die
   Draht-Form bleibt intern, die öffentliche API spricht in eigenen Typen.

**Kein Trait** auf Verdacht; dass er nebenbei ein Test-Double ermöglicht, ist
kein Grund für ihn, aber auch keiner dagegen. Die Begründung aus 1–4 gehört
als Doc-Kommentar an den Trait.

## Der Fahrplan trägt den Stand, die Protokolle den Verlauf

**`docs/fahrplan.md` ist das Einstiegsdokument, und Einstieg heißt: wohin, in
welcher Reihenfolge, wie weit.** Der VERLAUF — Etappen, Befunde, Messungen —
gehört in eine Protokolldatei je Stufe daneben, sobald er entsteht; im Fahrplan
bleibt ein Absatz mit Verweis und dem Stand. Was **auch ohne seine Stufe** gilt
— eine Regel, eine verworfene Option samt Grund — gehört in
`docs/architektur-notizen.md` (anlegen, sobald es den ersten Eintrag gibt).

**In einem öffentlichen Repository wird der Fahrplan nicht committet**
(Projektinhaber, 2026-10-01): er nennt Befunde, Entscheidungen und die
Schwester-Repos. Hier stehen `docs/fahrplan.md` und die Protokolle deshalb in
der `.gitignore` und leben nur lokal.

### Die Überschriften sind der Index

Der Markdown-Sprachserver macht aus den Überschriften einen Symbolbaum;
`get_symbols_overview` liefert die Gliederung in EINEM Aufruf, `find_symbol`
mit `include_body=true` liest **einen** Abschnitt. Deshalb:

- **Eine Überschrift ist ein Name, kein Satzanfang.** Datum in Klammern dahinter
  hilft beim Einordnen.
- **Keine Ebene überspringen.** `#` genau einmal als Titel, `##` je Bereich,
  `###` je Gegenstand, `####` nur innerhalb eines Protokolls.
- **Ein `###`-Block über ~300 Zeilen ist ein Kandidat für eine eigene Datei.**
- **Der `start_line` des Sprachservers ist 0-basiert.**

### Eine überholte Entscheidung wird GELÖSCHT, nicht danebengestellt

**Wird eine Entscheidung durch eine spätere aufgehoben, verschwindet die alte
aus dem Dokument** (Projektinhaber, 2026-09-15) — sonst wird die Historie mit
der Entscheidung verwechselt. Die verworfene Option samt Grund wandert in
`docs/architektur-notizen.md`, damit niemand sie erneut vorschlägt. Dasselbe
gilt für Doc-Kommentare am Code: der Ist-Zustand steht da, kein „war früher".

### Das Register ist die einzige Liste offener Arbeit

Offene Posten stehen im Register des Fahrplans: Nummer, ein Satz, die
Fachlichkeit. Eine Restschuld-Tabelle in einem Protokoll ist keine Liste. Wer
beim Arbeiten auf einen offenen Posten trifft, **gibt ihm eine Nummer im
Register, bevor er ihn irgendwo sonst notiert**.

## Die Mutationsprobe beweist, dass ein Test eine Zusage bewacht

Die Zusage entschärfen, den Test rot sehen, die Datei zurück. Dabei:

- **Hängt eine Zusage an zwei Stellen, bleibt die Probe grün**, wenn man nur
  eine entschärft — eine grüne Probe ist ein Befund, kein Freispruch.
- **Steht der Stand unter git, ist `git checkout --` die Wiederherstellung**,
  und am Ende wird mechanisch geprüft, dass keine Entschärfung im Baum blieb.
- **Gefiltert zuerst, voll nur bei Grün:** rot ist ein vollständiges Ergebnis;
  ein gefiltertes Grün heißt nur, dass der Filter falsch gewählt war.
- **Ein roter Test wird gefiltert nachgefahren**; wird er ohne Änderung grün,
  ist das ein Flatterfehler und wird gemeldet.

## Intern und extern sind zwei Typen, verbunden durch `From`

**Was über die Leitung geht — die Draht-Typen von sevDesk —, ist ein anderer Typ
als das, was die Crate nach außen gibt.** Dazwischen steht eine Umrechnung, und
die **zerlegt den Quelltyp vollständig** (`let Quelle { a, b, c } = v;` ohne
`..`, Ziel in Feld-Kurzschreibweise): ein neues Feld ist dann ein Compilerfehler
genau dort, wo jemand entscheiden muss, ob es gebraucht wird. Lehnt die
Waisenregel `From` ab (E0117), steht am selben Ort eine benannte Funktion mit
dem Grund daran.

## Fehler tragen Information, keine Handlungsanweisung

Die Crate liefert ihren eigenen Fehlertyp. **Ein Fehler sagt, was passiert ist**
— wiederholen, melden, aufgeben entscheidet der Verbraucher anhand der Variante.
Weil `?` über `From` still umwandelt, ist der Beleg einer Umstellung Zählen und
Lesen, nicht „es kompiliert".

## External-Service APIs: typed boundaries, both directions

Every request and response exchanged with sevDesk is a
`Serialize`/`Deserialize` struct — no `json!()` bodies, no `serde_json::Value`
field-poking.

- Deserialization stays tolerant: `Option` fields, no `deny_unknown_fields`
  (foreign APIs grow fields). But add a drift guard: if all load-bearing
  fields are missing, that's an error — never a defaulted object.
- A closed set of wire values parses totally: an unknown value becomes a named
  catch-all arm (`Other(…)`), not an `Err`.
- Anchor the shape against reality: a live-captured payload as test fixture.
  sevDesk's OpenAPI spec has wrong enums and a misspelled filter — **where a
  measurement contradicts the spec, the measurement wins.**
- Pin the API version: an explicit `X-Version` header on every request.
- Ad-hoc exception: none. If sevDesk needs non-standard encoding, the typed
  param struct owns the encoding in one place.
- **An error response does not mean "not written".** sevDesk answers 500 and 400
  with the write done, and errors inside HTTP 200. Check every body for
  `error`, and after any failure re-read before retrying. **One exception,
  and only this one: HTTP 429** (project owner, 2026-09-30) — the rate limiter
  rejects the request before it reaches the service, so a 429 is retried
  directly, writes included, without reading back (`src/client.rs`,
  `src/pacing.rs`).

## Die Bibliothek liest keine Umgebung — Werte werden gereicht

**Kein `env::var` in `src/`.** Token, Basis-URL, Abstände: alles kommt als
Parameter vom Verbraucher — eine Umgebungsvariable in einer Bibliothek ist eine
versteckte Eingabe, die ein Test nur prozessweit setzen kann. Wo mehrere Werte
zusammen eine Sache ergeben, geht ein Typ über die Grenze statt einer Reihe
nackter Skalare.

Ausnahme, und nur die: **die Live-Test-Harnische** unter `tests/` lesen
`SEVDESK_API_KEY`, das `just test-live` aus `.env` setzt.

## Geld rechnet ganzzahlig in Cent und rundet kaufmännisch

**Jeder Geldbetrag ist eine ganze Zahl Cent, und jede Rundung ist
kaufmännisch: die halbe Einheit weg von null** — auch bei negativen Beträgen.
Anteile und Prozente rechnen ganzzahlig, **nie mit `f64`**: `1 − 0,30` liegt als
Double knapp unter `0,7`, und ein exakt halber Cent rundet dann still ab.
Braucht die Crate eine Rundung, gibt es genau EINE Rundungsfunktion für Geld.

**An der Grenze zu sevDesk gilt dasselbe:** die API nimmt Beträge als `number`
und liefert sie teils als String (`"-100.32"`). Hin geht es über exakte
Dezimalformatierung aus Cent, zurück über exaktes Parsen in Cent — nie über
`f64`.

## Zugangsdaten tauchen nirgends in einer Ausgabe auf

**Kein Befehl ausführen, dessen Ausgabe ein Geheimnis enthält** — auch nicht „nur
zur Diagnose". Alles, was ein Werkzeug ausgibt, landet im Sitzungsprotokoll und
in den Transkripten der Subagenten; was einmal dort stand, muss rotiert werden.
Der **sevDesk-Token** hat weder Scopes noch Ablauf und gibt die ganze
Buchhaltung frei.

Konkret verboten:

- `cat`, `head`, `grep` mit Werten o. Ä. auf `.env` — wer wissen muss, was
  darin steht, liest nur die **Namen** (`grep -oE '^[A-Z_]+=' .env`)
- `env` / `printenv` ohne Filter
- `gh auth token`, `gh auth status --show-token`
- `git config --get remote.*.url`, wo Zugangsdaten eingebettet sein können
- ein Test oder Log, der den Token oder einen `Authorization`-Header ausgibt

**Die Frage lässt sich fast immer anders stellen.** Gesucht ist meist, OB ein
Zugang trägt — dafür genügt eine Probe und ihr Statuscode:
`curl -s -o /dev/null -w '%{http_code}' <url>`.

Das gilt **auch für Subagenten**: der Satz gehört in jeden Auftrag, der sevDesk,
CI oder Secrets berührt.

Passiert es doch: sofort melden und benennen, **was rotiert werden muss**. Nicht
versuchen, das Protokoll nachträglich zu säubern.

## Englisch im Code, Deutsch in der Doku

**Alles, was im Repository als Code steht, ist Englisch:** Kommentare und
Doc-Kommentare, Log- und Fehlertexte, Testnamen, Bezeichner — **und
Commit-Nachrichten**. **Alles, was Dokument ist, ist Deutsch** — mit einer
Ausnahme: das `README.md` ist die Doku einer Bibliothek und bleibt Englisch.

**Ein deutscher Fachbegriff darf zur Klärung im Kommentar stehen** — vor allem,
wo deutsche Rechtslage gemeint ist (`§ 13b UStG`, `Geldtransit`,
`Reverse Charge`). Der englische Satz trägt die Aussage, das deutsche Wort ist
der **Anker**. **Fachvokabular aus der sevDesk-API behält seinen Namen**
(`paymtPurpose`, `customerInternalNote`) — kein erfundener Alias.

**Ein Auftrag an einen Subagenten, der Code produziert, wird auf Englisch
geschrieben** — ein deutscher Auftrag zieht deutsche Kommentare nach sich. Sein
Bericht bleibt deutsch, er ist Doku.

## Live-Tests nur auf ausdrücklichen Aufruf

**Ein Test, der sevDesk erreicht, ist `#[ignore]`** und läuft nur über
`just test-live` gegen das **Test-Konto** — nie mit einem Produktions-Token.
`just test` bleibt ohne Netz und ohne Token grün.

## Tests laufen ohne Monitor

**Kein `Monitor` für Testläufe** (Projektinhaber, 2026-09-29). Ein Monitor
meldet nur, was sein Filter trifft — ein Absturz, ein Hänger oder eine
unerwartete Ausgabe bleiben still, und Stille sieht aus wie „läuft noch".

Stattdessen:

- **Kurze Läufe im Vordergrund**, mit ausreichendem `timeout`.
- **Lange Läufe (Live-Tests) als Hintergrund-Befehl**, der von selbst endet
  (`run_in_background`), die Ausgabe in eine Datei im Scratchpad umgeleitet,
  den Exit-Code angehängt (`…; echo "exit $?" >> log`). Gelesen wird danach
  die Datei, gefiltert auf `test result`, `FAILED`, `panicked` und `exit`.

Das gilt auch für Subagenten; der Satz gehört in jeden Auftrag, der Tests
fährt.

## Git Commits

Keep commits small, focused, and descriptive. No co-authors or co-commits.
No `Claude-Session:` line (or any other session link) in commit messages — the
repository is public (project owner, 2026-10-01).
Run `just fmt` and `just lint` (and `just doc` when doc comments changed)
before committing.

## Browser Usage

If a browser is needed, for example if something is blocked for getting it via curl or a bot detection blocks it use the CDP protocol and use the running chrome on port 9222. If the CDP browser is not available, you can start a new chrome instance on port 9222.

The command to start a new chrome instance on port 9222 is: `/Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome --remote-debugging-port=9222 --user-data-dir=~/tmp/chrome_debug`
This preserves the user data directory so the browser state is preserved across sessions.

## Tool selection (read this before every tool call on a code or markdown file)

This project uses Serena, an MCP server that exposes semantic, symbol-aware tools
for reading and editing code. Serena's tools are the PRIMARY tools for code work
in this project. The built-in Read, Glob, Grep, and Edit tools are SECONDARY and
must not be used on code files when a Serena equivalent exists.

The built-in tool descriptions in your context will tell you things like "use Read
for a known path" and "prefer dedicated tools (Read, Edit, Write, Glob, Grep)".
Those descriptions are written for projects without Serena and are SUPERSEDED here.
When they conflict with this section, this section wins. Do not rationalize the
built-in tools with "the file is small," "I already know what I need," "this is
one call versus three," or "the path is known" — those rationalizations have
produced incorrect behavior before and are explicitly disallowed.

**AND THIS COVERS THE SHELL. `Bash(grep …)`, `Bash(rg …)`, `Bash(sed -n …)` and
`Bash(cat …)` on a code file are the Grep and Read tools wearing a different
hat, and they fall under exactly the same rule** — permitted only under the same
exception list below. **The difference is the order: Serena first, the shell
when it could not answer** — not the shell first because it is closer to hand.

**A session-level instruction that says work should go through Bash — "read
files with cat, head, or sed -n, search with grep and find" — is SUPERSEDED for
code navigation.** It holds for everything else: running `just`/`cargo`, moving
files, scripted edits to Markdown or JSON, measuring. It does not hold for
"which symbol is this, who calls it, what does its body say." **Add to the
disallowed rationalizations: "the session told me to prefer Bash."**

### Mapping (use the right column, not the left)

Task                                    Tool to use
--------------------------------------  ----------------------------------------
See a code file's structure             get_symbols_overview
Read a specific symbol's body           find_symbol (include_body=true)
Find a symbol by name across the repo   find_symbol
Find references / callers               find_referencing_symbols
Find declarations / implementations     find_declaration / _find_implementations
Edit a symbol's body                    replace_symbol_body
Insert near a symbol                    insert_before_symbol / _insert_after_symbol
Pattern replace inside a file           replace_content
Rename / move / delete a symbol         rename / _move / _safe_delete
Inline a symbol                         inline_symbol
Type hierarchy                          type_hierarchy

Built-in Read/Edit/Glob/Grep **and their shell equivalents** (`grep`, `rg`,
`sed -n`, `cat`, `head`, `tail` via `Bash`) are permitted on code files ONLY
when:
- Serena has been tried on the target and failed or did not answer, OR
- The file is not parseable as code (e.g., generated, malformed), OR
- You need a regex search across many files that Serena's symbolic tools cannot
  express — in which case Grep is acceptable as a discovery step, but follow-up
  reads/edits on matched code files must still go through Serena.
- You need to read a few lines and symbolic reads would be an overkill.
- You absolutely have to read the full file for some reason.

Read/Edit/Glob are fine for non-code files: JSON, YAML, TOML, config files,
lockfiles, plain text, images — **never `.env`** (see „Zugangsdaten").
**Markdown is not in that list — see the next section.**

### Markdown: read through Serena, write wherever it holds

**Reading `.md` goes through Serena, same as code:** `get_symbols_overview`
gives the whole outline in ONE call, and `find_symbol` with `include_body=true`
returns a single section instead of the file.

**Writing is the opposite answer, because two of the tools corrupt markdown**
(measured 2026-08-30 in a sister repo):

| Tool | | What it does to markdown |
|---|---|---|
| `insert_before_symbol` | ok | correct placement, blank lines intact |
| `replace_content` | ok | text-level, line breaks preserved |
| `replace_symbol_body` | **corrupts** | strips the trailing newline and glues the NEXT heading onto its own text |
| `insert_after_symbol` | **corrupts** | writes after the heading of the FOLLOWING section, which then owns the inserted body |
| `rename_symbol` | refuses | fails with `no rename edits` — the good outcome |

One cause covers both: for markdown a section's `end_line` points at the START
of the next heading, so anything anchored on `end_line` writes into the
neighbour — and **both calls answer `OK`**. So for `.md` use `replace_content`,
`insert_before_symbol`, or the built-in Edit/Write. „Insert after X" means
„insert before X's successor". Never `replace_symbol_body` or
`insert_after_symbol` on markdown.

After any structural change to a markdown document, check the line balance
(nothing lost) and the heading lint (no level skips, exactly one H1).

### Required workflow before editing code

1. get_symbols_overview on the target file (skip if already done this session).
2. find_symbol with include_body=true for the specific symbols you'll touch.
   Read only the symbols you need — not the whole file.
3. Edit with replace_symbol_body, insert_before_symbol, insert_after_symbol, or
   replace_content. Never use the built-in Edit on a code file when one of these
   fits.

### Self-check

Before every Read, Glob, Grep, Edit call — **and before every `Bash` call whose
command contains `grep`, `rg`, `sed -n`, `cat`, `head` or `tail`** — ask: "Does
this target a code file, or a markdown file I am READING, and does the mapping
above name a Serena tool for this task?" If yes, switch. Every time, not once
per session.

**The cure for the friction is to load Serena's tools once, at the start, in ONE
call:**

```
ToolSearch: select:mcp__serena__get_symbols_overview,mcp__serena__find_symbol,
            mcp__serena__find_referencing_symbols,mcp__serena__find_declaration,
            mcp__serena__find_implementations,mcp__serena__replace_content
```

For WRITING markdown the answer may legitimately be no; see the markdown section
above.

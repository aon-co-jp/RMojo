# RMojo 開発方針 / Development notes

**日本語**: 共通ルールは [`open-raid-z/CLAUDE.md`](https://github.com/aon-co-jp/open-raid-z) と [`runo`](https://github.com/aon-co-jp/runo) の `CLAUDE.md` を参照。チャットは日本語のみ。確認なしで開発・TEST・DEBUG・commit・push。UI・インストーラーはリリース前に実機/エミュレーターで確認。heredocでバックスラッシュを含むコードを書かない(Writeツールを使う)。
**English**: shared rules live in `open-raid-z/CLAUDE.md` and `runo/CLAUDE.md`. Chat in Japanese only. Develop, test, debug, commit and push without asking. Verify UI/installers on real devices/emulators before each release. Do not write code containing backslashes through bash heredocs (use the Write tool).

## 状態 / Status
- 日本語: v0.3.0。Windows x64インストーラー・Android実機(Android 14)・Android TVエミュレーター(i686)で確認済み。Windows ARM64(Snapdragon)ネイティブビルドとRMojo-Setup-arm64.exeをCIに追加(CI成功、実機未確認)。
- English: v0.3.0. Verified on the Windows x64 installer, a real Android 14 phone and the Android TV emulator (i686). Native Windows ARM64 (Snapdragon) build and RMojo-Setup-arm64.exe added in CI (CI green, no real device).

## 未了 / Open
- 日本語: Snapdragon実機、BSD実機、iOS実行パッケージ、`while`+添字代入中心のループ(CPythonの約1.5倍遅い、バイトコード化が要る)、WSLでのAndroidビルド(Linux版NDKが必要。Windows側cargoでは scripts/android-i686-wsl.sh の代わりに cargo ndk -t x86)。
- English: Snapdragon hardware, BSD hardware, iOS runnable package, while/index-assignment loops (~1.5x slower than CPython; needs a bytecode stage), Android build in WSL (needs a Linux NDK; with Windows cargo use cargo ndk -t x86).

詳細・再開用メッセージ / details and the resume message: [`PORTING.md`](PORTING.md)

## API 方針: REST 全廃・gRPC / Connect へ (2026-11-04)

### 日本語
- **REST は全廃する。新しい API は REST で作らない。** ブラウザや `curl` から呼ぶ API は **Connect の単純呼び出し**(`POST /<パッケージ>.<Service>/<Method>`、`content-type: application/json`、本文は JSON)、サービス間は **gRPC / gRPC-web**(RPoem の管理API `open_runo.admin.v1.Admin` と同じ流儀)。ストリーミングは、必要になってから。
- **既存の REST は、触るときに順に移す。** 移したら旧 REST の経路は消す。移行が終わるまで残るが、**新規には追加しない**。
- **JSON は RS-JSON に統一**する(読み込みは `from_str_secure` / `parse_secure` でサイズ・深さ・重複キーの上限つき、書き出しは `rust_json::fast`)。ブラウザの WASM に `prost` は入れない(サイズが増えるため)。
- **例外(自分たちの API ではなく、相手の規格が決めているもの)**: ACME、OAuth/OIDC のコールバック、第三者(GitHub など)からの Webhook、ヘルスチェック、静的ファイル配信、OTLP など。これらは規格どおりの HTTP を受ける。
- 参考実装: `RPoem`(`open_runo.admin.v1.Admin`)、`open-easy-web`(`server/src/connect.rs`、`proto/open_easy_web/admin/v1/admin.proto`)。
- ※ このリポジトリのコードが既に方針どおりかどうかは、各リポジトリの現状による。この節は方針の記録であり、コードの変更を意味しない。

### English
- **REST is being abolished. New APIs are not built as REST.** APIs called from browsers or `curl` use **Connect unary calls** (`POST /<package>.<Service>/<Method>`, `content-type: application/json`, JSON body); service-to-service calls use **gRPC / gRPC-web** (the same style as RPoem's admin API `open_runo.admin.v1.Admin`). Streaming only when it is actually needed.
- **Existing REST endpoints are migrated one by one, when they are touched.** The old REST route is removed once migrated. Until migration is finished they remain, but **no new ones are added**.
- **JSON is unified on RS-JSON**: reading with `from_str_secure` / `parse_secure` (size, depth and duplicate-key limits), writing with `rust_json::fast`. `prost` is not put into browser WASM (it would grow the binary).
- **Exceptions (shapes dictated by someone else's standard, not our own API)**: ACME, OAuth/OIDC callbacks, webhooks from third parties (e.g. GitHub), health checks, static file serving, OTLP, and so on. These accept plain HTTP as the standard requires.
- Reference implementations: `RPoem` (`open_runo.admin.v1.Admin`) and `open-easy-web` (`server/src/connect.rs`, `proto/open_easy_web/admin/v1/admin.proto`).
- Note: whether the code in a given repository already follows this policy depends on that repository. This section records the policy; it does not mean the code has been changed.

### Deutsch (KI-Übersetzung, redaktionell überarbeitet; noch nicht von Muttersprachlern geprüft)
- **REST wird vollständig abgeschafft; neue APIs entstehen nicht mehr als REST.** APIs, die aus dem Browser oder per `curl` aufgerufen werden, verwenden **unäre Connect-Aufrufe** (`POST /<Paket>.<Service>/<Methode>`, `content-type: application/json`, JSON-Body). Zwischen Diensten kommen **gRPC bzw. gRPC-web** zum Einsatz, im selben Stil wie die Admin-API von RPoem (`open_runo.admin.v1.Admin`). Streaming wird erst ergänzt, wenn es tatsächlich gebraucht wird.
- **Bestehende REST-Endpunkte werden schrittweise umgestellt, und zwar immer dann, wenn man ohnehin an ihnen arbeitet.** Sobald ein Endpunkt umgestellt ist, wird die alte REST-Route entfernt. Bis zum Abschluss der Migration bleiben die übrigen bestehen, es werden aber **keine neuen mehr hinzugefügt**.
- **JSON wird einheitlich über RS-JSON verarbeitet**: Gelesen wird mit `from_str_secure` bzw. `parse_secure` (mit Obergrenzen für Größe, Verschachtelungstiefe und doppelte Schlüssel), geschrieben mit `rust_json::fast`. `prost` wird nicht in das Browser-WASM aufgenommen, da es die Datei vergrößern würde.
- **Ausnahmen (Schnittstellen, deren Form ein fremder Standard vorgibt und die nicht zu unserer eigenen API gehören)**: ACME, OAuth/OIDC-Callbacks, Webhooks von Drittanbietern (z. B. GitHub), Health-Checks, die Auslieferung statischer Dateien, OTLP und Ähnliches. Diese nehmen, wie vom jeweiligen Standard verlangt, gewöhnliches HTTP entgegen.
- Referenzimplementierungen: `RPoem` (`open_runo.admin.v1.Admin`) und `open-easy-web` (`server/src/connect.rs`, `proto/open_easy_web/admin/v1/admin.proto`).
- Hinweis: Ob der Code eines einzelnen Repositorys diese Richtlinie bereits erfüllt, hängt vom jeweiligen Repository ab. Dieser Abschnitt dokumentiert lediglich die Richtlinie; er besagt nicht, dass der Code schon angepasst wurde.

### Русский (перевод выполнен с помощью ИИ и отредактирован; носителями языка пока не проверен)
- **От REST мы полностью отказываемся; новые API больше не создаются в стиле REST.** API, к которым обращаются из браузера или через `curl`, используют **унарные вызовы Connect** (`POST /<пакет>.<Service>/<Method>`, `content-type: application/json`, тело запроса — JSON). Для взаимодействия между сервисами применяются **gRPC и gRPC-web** — в том же стиле, что и административный API RPoem (`open_runo.admin.v1.Admin`). Потоковая передача данных добавляется лишь тогда, когда в ней действительно возникает необходимость.
- **Существующие REST-эндпоинты переводятся поэтапно — по мере того, как с ними приходится работать.** После перевода старый REST-маршрут удаляется. Пока миграция не завершена, оставшиеся эндпоинты сохраняются, но **новые не добавляются**.
- **Работа с JSON унифицирована на основе RS-JSON**: чтение — через `from_str_secure` или `parse_secure` (с ограничениями на размер, глубину вложенности и повторяющиеся ключи), запись — через `rust_json::fast`. `prost` в браузерный WASM не включается, поскольку это увеличило бы размер файла.
- **Исключения (интерфейсы, форму которых определяет чужой стандарт, а не наш собственный API)**: ACME, обратные вызовы OAuth/OIDC, вебхуки сторонних сервисов (например, GitHub), проверки работоспособности, раздача статических файлов, OTLP и тому подобное. Такие интерфейсы принимают обычный HTTP, как того требует соответствующий стандарт.
- Эталонные реализации: `RPoem` (`open_runo.admin.v1.Admin`) и `open-easy-web` (`server/src/connect.rs`, `proto/open_easy_web/admin/v1/admin.proto`).
- Примечание: соответствует ли код конкретного репозитория этой политике, зависит от самого репозитория. Данный раздел лишь фиксирует политику и не означает, что код уже изменён.

## API 方針の補足: 用語の整理と使い分け (2026-11-04)

### 日本語
**1つに統一するものではなく、別の層の話です。** 「REST の代わり」になるのは Connect・gRPC・GraphQL だけで、WASM は画面を作る技術です。

| 用語 | 何か | REST の代わりか |
|---|---|---|
| REST | URL と HTTP メソッドで操作を表す API の作り方 | 全廃する側 |
| **Connect** | `POST /サービス名/メソッド名` に JSON を送る形式。`curl` でもブラウザの `fetch` でも呼べる | はい(**採用**) |
| **gRPC** | 同じ「サービス/メソッド」の考え方を、バイナリ(protobuf)と HTTP/2 で行う形式。速く、ストリーミングも得意 | はい(**サービス同士に採用**) |
| gRPC-web | gRPC のブラウザ向けの変形版 | はい(必要なら) |
| GraphQL | 欲しい項目を問い合わせる形式 | 一部だけ(**問い合わせ(読み取り)専用**) |
| WASM | Rust をブラウザの中で動かす仕組み | いいえ(画面を作る技術) |

- **ブラウザの画面や `curl` から呼ぶ API は Connect**(POST、JSON を RS-JSON で読み書き)。ブラウザの WASM は素の gRPC を話せず、Connect なら `fetch` のままで、`prost` を足さずに済む(画面のサイズも増えない)。
- **サービス同士の通信(サーバー間、重い処理、ストリーミング)は gRPC。** 多くは Connect で足りるので、ストリーミングが必要になった所だけ gRPC に上げる。
- **GraphQL は、まとめて問い合わせたいときの読み取り専用。** 書き込み(操作)には使わない。
- **WASM は画面を作る技術として、そのまま使う。** 画面から呼ぶ API が Connect になっただけ。
- 外部の規格が形を決めるもの(ACME、OAuth のコールバック、第三者の Webhook、ヘルスチェック、静的ファイル、OTLP)は、規格どおりの HTTP のまま。
- **迷ったら Connect。**

### English
**These are not alternatives to unify; they sit at different layers.** Only Connect, gRPC and GraphQL can replace REST; WASM is a way to build the UI.

| Term | What it is | Replaces REST? |
|---|---|---|
| REST | API style that expresses operations through URLs and HTTP methods | The thing being abolished |
| **Connect** | `POST /Service/Method` with a JSON body; callable from `curl` and from a browser `fetch` | Yes (**adopted**) |
| **gRPC** | The same service/method idea over binary (protobuf) and HTTP/2; fast, good at streaming | Yes (**for service-to-service**) |
| gRPC-web | A browser-friendly variant of gRPC | Yes (if needed) |
| GraphQL | A query language: ask for exactly the fields you want | Partly (**read-only queries**) |
| WASM | Runs Rust inside the browser | No (a UI technology) |

- **APIs called from browser screens or `curl` use Connect** (POST, JSON read and written with RS-JSON). Browser WASM cannot speak plain gRPC; with Connect it stays a `fetch`, and `prost` is not needed (the binary does not grow).
- **Service-to-service calls (server to server, heavy work, streaming) use gRPC.** Connect is enough in most places; move a call to gRPC only where streaming is actually needed.
- **GraphQL is read-only**, for queries that gather several pieces of data. It is not used for writes (operations).
- **WASM stays as the way to build the UI.** The API it calls simply became Connect.
- Interfaces whose shape an outside standard dictates (ACME, OAuth callbacks, third-party webhooks, health checks, static files, OTLP) stay plain HTTP.
- **When in doubt, use Connect.**

### Deutsch (KI-Übersetzung, sprachlich überarbeitet; noch nicht von Muttersprachlern geprüft)
**Es geht nicht darum, eine einzige Technik auszuwählen – die Begriffe liegen auf unterschiedlichen Ebenen.** Als Ersatz für REST kommen nur Connect, gRPC und GraphQL in Frage; WASM dagegen ist eine Technik, mit der die Benutzeroberfläche gebaut wird.

| Begriff | Was ist das? | Ersatz für REST? |
|---|---|---|
| REST | Ein API-Stil, bei dem Operationen über URLs und HTTP-Methoden ausgedrückt werden | wird abgeschafft |
| **Connect** | `POST /Dienst/Methode` mit JSON im Body; aufrufbar mit `curl` ebenso wie mit `fetch` im Browser | ja (**gewählt**) |
| **gRPC** | Dasselbe Prinzip aus Dienst und Methode, aber über binäres protobuf und HTTP/2; schnell und gut für Streaming geeignet | ja (**zwischen Diensten**) |
| gRPC-web | Eine für den Browser angepasste Variante von gRPC | ja (bei Bedarf) |
| GraphQL | Abfragesprache, mit der man genau die benötigten Felder anfordert | teilweise (**nur für lesende Abfragen**) |
| WASM | Führt Rust-Code direkt im Browser aus | nein (Technik für die Oberfläche) |

- **APIs, die von Browseroberflächen oder per `curl` aufgerufen werden, verwenden Connect** (POST; JSON wird mit RS-JSON gelesen und geschrieben). WASM im Browser kann kein reines gRPC sprechen. Mit Connect bleibt es bei einem gewöhnlichen `fetch`, und `prost` ist nicht nötig – die WASM-Datei wird dadurch nicht größer.
- **Für die Kommunikation zwischen Diensten (Server zu Server, rechenintensive Aufgaben, Streaming) wird gRPC eingesetzt.** In den meisten Fällen reicht Connect aus; auf gRPC wird nur dort umgestellt, wo Streaming tatsächlich gebraucht wird.
- **GraphQL dient ausschließlich lesenden Abfragen**, wenn mehrere Informationen gebündelt abgefragt werden sollen. Für Schreibzugriffe (Operationen) wird es nicht verwendet.
- **WASM bleibt die Technik für die Oberfläche.** Geändert hat sich lediglich, dass die aufgerufene API nun auf Connect beruht.
- Schnittstellen, deren Form durch einen externen Standard festgelegt ist (ACME, OAuth-Callbacks, Webhooks von Drittanbietern, Health-Checks, statische Dateien, OTLP), bleiben gewöhnliches HTTP.
- **Im Zweifel: Connect.**

### Русский (перевод выполнен с помощью ИИ и литературно отредактирован; носителями языка пока не проверен)
**Это не варианты, из которых нужно выбрать один, а технологии разных уровней.** Заменить REST могут только Connect, gRPC и GraphQL; WASM же — это способ создания пользовательского интерфейса.

| Термин | Что это | Заменяет REST? |
|---|---|---|
| REST | Стиль построения API, при котором операции выражаются через URL и HTTP-методы | отменяется |
| **Connect** | `POST /Сервис/Метод` с телом в формате JSON; вызывается как из `curl`, так и из браузерного `fetch` | да (**выбран**) |
| **gRPC** | Тот же принцип «сервис — метод», но поверх двоичного формата protobuf и HTTP/2; работает быстро и хорошо подходит для потоковой передачи | да (**для связи между сервисами**) |
| gRPC-web | Адаптированный для браузера вариант gRPC | да (при необходимости) |
| GraphQL | Язык запросов: клиент запрашивает ровно те поля, которые ему нужны | частично (**только запросы на чтение**) |
| WASM | Позволяет выполнять код на Rust прямо в браузере | нет (технология для интерфейса) |

- **API, которые вызываются из браузерных интерфейсов или через `curl`, строятся на Connect** (POST; JSON читается и записывается через RS-JSON). Браузерный WASM не умеет работать с «чистым» gRPC. С Connect достаточно обычного `fetch`, а `prost` не требуется — размер WASM-файла не увеличивается.
- **Для обмена между сервисами (сервер — сервер, ресурсоёмкие задачи, потоковая передача) применяется gRPC.** В большинстве случаев достаточно Connect; на gRPC переходят только там, где потоковая передача действительно необходима.
- **GraphQL используется только для чтения**, когда нужно получить сразу несколько связанных данных. Для записи (операций) он не применяется.
- **WASM по-прежнему остаётся технологией для интерфейса.** Изменилось лишь то, что вызываемый им API теперь построен на Connect.
- Интерфейсы, форма которых задана внешним стандартом (ACME, обратные вызовы OAuth, вебхуки сторонних сервисов, проверки работоспособности, статические файлы, OTLP), остаются обычным HTTP.
- **Если есть сомнения — выбирайте Connect.**

## 補足: ストリーミングの対応状況 (2026-11-07)

### 日本語
- **server-streaming は対応済み**(`RPoem` の `open-runo-connect-*`、RPoem main `1570371`・`34cb595`)。Connect は `application/connect+json` のエンベロープ、gRPC / gRPC-web でも受ける。途中エラーは終端フレームで返し、件数・時間の上限、切断時の資源解放、実 TCP での `WatchEvents` 試験を通過。**VPS には未反映**。
- **使いどころ**: 変更の監視、進捗の逐次通知、長い生成の逐次返却など、サーバーから複数回返したい所。1回で済む呼び出しは従来どおり Connect の単純呼び出し。client-streaming / 双方向は、必要になってから。
- 独語・露語は未追記(日英のみ)。

### English
- **Server-streaming is supported** (`open-runo-connect-*` in RPoem, main `1570371` / `34cb595`): `application/connect+json` envelopes, also via gRPC / gRPC-web; mid-stream errors end with a terminal frame; item/time limits, resource release on disconnect, and a real-TCP `WatchEvents` test pass. **Not yet deployed to the VPS.**
- **When to use**: watching changes, progress updates, incremental results. One-shot calls stay plain Connect unary. Client-streaming / bidirectional only when needed.
- German and Russian not yet added.

## Windows インストーラー方針 / Windows installer policy
**日本語**: インストーラーを持つ製品の Windows 版は、すべて `.exe`(Inno Setup/NSIS、必要なら PyInstaller 同梱)で配布する。PowerShell スクリプトは開発用に残すだけで、利用者向けの入口にしない。RMojo は `RMojo-Setup-x64/arm64.exe` を配布済み、2FastAPI は v0.3.1 から `2FastAPI-Setup-x64/arm64.exe`。MFastAPI は pip の wheel 配布でインストーラーを持たない(Windows でも `pip install mfastapi`)。
**English**: Every product that has an installer ships its Windows build as an `.exe` (Inno Setup/NSIS, with PyInstaller bundling where needed); PowerShell scripts stay for development only. RMojo ships `RMojo-Setup-x64/arm64.exe`; 2FastAPI ships `2FastAPI-Setup-x64/arm64.exe` since v0.3.1. MFastAPI has no installer (wheel only: `pip install mfastapi`, also on Windows).

**2026-11-16**: 集合・連鎖代入・try/else・bare raise・`**`展開・入れ子書式・型別except・例外クラス継承に対応、リスト速度は CPython の約1.2倍まで改善(詳細 `PORTING.md`)。 / Added sets, chained assignment, try/else, bare raise, `**` unpacking, nested format specs, typed except and exception inheritance; list speed is now ~1.2x of CPython (see `PORTING.md`).

## 2026-11-23 組み込み上限 / Embedding limits
`Limits{max_steps,max_ms}` は `CompiledProgram::call_limited` でループ反復ごとに効く(時計は1024回ごと)。変更したら `tests/api.rs` と `tests/run.py`(差分0)を確認し、MFastAPI の `DESIGN-MOJO.md` も同日に更新する。未了=メモリ上限・単発の重い処理。

## 2026-11-24 メモリ上限 / Memory caps
`MAX_COLL = 20_000_000`(`src/lib.rs`)。`append`・`list*n`・`str*n`・`range` が超えると `MemoryError`。試験 `memory_caps_stop_blowup`。実メモリ(RSS)の上限は未実装。
`MAX_COLL = 20_000_000`; exceeding it in `append`, `list*n`, `str*n` or `range` raises `MemoryError`. Test: `memory_caps_stop_blowup`. A true RSS cap is not implemented.

## 2026-11-25 現況・未了 / Status and open items

### 日本語

現状: Mojo との互換を目指す言語。見た目は Python 構文、中身は Rust(C++ なし)。最近は実行の上限を追加した(ループ回数、時間 Limits.max_ms、リスト・文字列・range の要素数の上限で MemoryError)。POLICY.md も新設した。残り: Mac・iPhone の実機での確認(ここには実機がありません)。翻訳: AI 翻訳(ja/en 以外)、ネイティブの確認はまだです。

### English

Status: a language aiming at Mojo compatibility: Python syntax on the surface, Rust inside (no C++). Recently it gained execution limits (loop steps, a time limit Limits.max_ms, and element-count caps on lists, strings and ranges that raise MemoryError). POLICY.md was also added. Open: checks on a real Mac and iPhone (no such devices here). Translations: AI-translated (other than ja/en), not yet checked by native speakers.

<!-- directive-20261125-1130 -->
## 2026-11-25 指示(11:30 まで確認不要) / Directive (no confirmation until 11:30)
**日本語**: 2026-11-25 11:30 まで、未了は確認なしで完成まで自動で進める。需要が少ない・必要性が低いと判断される、あるいは1から大規模に作り直す必要があっても、別の方式で済ませず、正面から作り直す。同時に開発する対象: COBOL_to・Java_to・MFastAPI・2MFastAPI(他の言語・フレームワークから MFastAPI 互換へ。Node.js・TypeScript は例外的に選べ、選ばない選択も可能)・RMojo・RPoRea(RPoem と Reaxt の統合)・2RPoRea(他の言語・フレームワークから RPoRea へ)・Reaxt(React+Next.js+Nuxt.js+Nest.js の統合)。1つ変えたら他を確認し、README・CLAUDE・PORTING・POLICY を日英で同じ日に更新する。
**English**: Until 2026-11-25 11:30, carry unfinished work through to completion automatically without asking. Even if demand looks low or a large from-scratch rewrite is needed, do not substitute another approach; do the real work. Developed together: COBOL_to, Java_to, MFastAPI, 2MFastAPI (converts other languages and frameworks to MFastAPI-compatible code; Node.js and TypeScript are an optional exception and choosing neither is allowed), RMojo, RPoRea (RPoem and Reaxt integrated), 2RPoRea (converts other languages and frameworks to RPoRea) and Reaxt (React + Next.js + Nuxt.js + Nest.js integrated). When one changes, check the others and update README, CLAUDE, PORTING and POLICY in Japanese and English the same day.

<!-- directive-20261125-mojo -->
## 2026-11-25 追記: MFastAPI は Mojo から使えること / Addendum: MFastAPI must be usable from Mojo
**日本語**: MFastAPI は **Mojo から使用できる仕様**を保つ(Python 構文・Rust 実装・Mojo 対応、Windows では RMojo の `compat/wsl-bridge` 経由で WSL 上の Mojo を使う)。2MFastAPI の変換結果も Mojo から使えること。現状: `mojo/` は参照実装で、段階B・Cのサーバーは実験的。Mojo からの呼び出しの実機確認は、確認済みの範囲だけを「確認済み」と書く。11:30 まで確認なしで自動で進める。
**English**: MFastAPI keeps the **specification of being usable from Mojo** (Python syntax, Rust core, Mojo support; on Windows the real Mojo in WSL is driven through RMojo's `compat/wsl-bridge`). Output of 2MFastAPI must be usable from Mojo too. Current state: `mojo/` is a reference implementation and its stage B/C servers are experimental. Only what was actually run is called "verified". Continue automatically without asking until 11:30.

## Versionless API の方針 / Versionless API policy (2026-11-25)
**日本語**: このリポジトリのAPIは **Versionless**(URL・パッケージ名に `v1` 等を付けず、互換性を壊さず足すだけで進化させる)。欄は足すだけ・足す欄は省略可能・知らない欄は無視・名前変更は新名を足して旧を非推奨に(期限は既定で6か月以上、使用0を確認してから削除)・enumは未知の値を許容・安全上の緊急修正は例外。既存の `.v1` は呼び出し側を壊さないよう当面残し、版なしの別名を追加して段階的に畳む。詳細・出典は [`runo/API-POLICY.md`](https://github.com/aon-co-jp/runo/blob/main/API-POLICY.md) 末尾。適用はこれから順に行う。
**English**: This repository's APIs are **Versionless**: no `v1` in URLs or package names; evolve by adding only, without breaking compatibility. Only add fields, new fields are optional, ignore unknown fields, rename by adding the new name and deprecating the old (default deadline 6 months or more, remove only after confirming zero use), tolerate unknown enum values, emergency security fixes are the exception. Existing `.v1` names stay for now so callers do not break; a versionless alias is added and `.v1` is retired in stages. Details and sources: the end of [`runo/API-POLICY.md`](https://github.com/aon-co-jp/runo/blob/main/API-POLICY.md). Rollout is step by step.
## 2026-11-26 cobol2mojo の拡張(12/20本が一致) / cobol2mojo extended (12 of 20 match)
**日本語**: `tools/cobol2mojo/cobol_py2mojo.py` を拡張した。追加: 表(`[0]*N` → `List[Int]`)、段落内の局所変数、`abs`・`10 ** k`、改行なしの DISPLAY、文字列操作(連結・UNSTRING の分割・数え上げ・置換・大文字化・数字判定・INSPECT の検索/切り出し/先頭一致回数)、ファイル入出力(`_FILES` の固定長レコードの OPEN/READ/WRITE/CLOSE と FILE STATUS)、SORT/MERGE(ファイル読込・安定ソート・書き出し、INPUT/OUTPUT PROCEDURE)。CALL は Python と同じく例外で停止する。WSL(Ubuntu-24.04、Mojo 1.2)の `verify.sh` で、**hello・calc・eval・eval2・comp3・features・search1・inspect2・fileio・merge1・sort1・sort2 の12本が Python 出力と完全一致**(Mojo の警告・エラーなし)。**TODO(8本)**: sql1〜sql7(Python 側の `SQL_HANDLER`=SQL 処理系を Mojo に写す必要があり未着手)、misc(Python 自身が CALL "GREET" で NotImplementedError になり正常出力が無いため、一致を確認できない。変換自体はでき、Mojo も同じ例外で止まる)。数値の読み取り `_num` は Python の `int()` のうち符号と数字だけを扱う(`1_0` 等は未対応)。
**English**: Extended `tools/cobol2mojo/cobol_py2mojo.py` with tables (`[0]*N` to `List[Int]`), paragraph locals, `abs` and `10 ** k`, DISPLAY without newline, string operations (concatenation, UNSTRING split, count, replace, upper, numeric test, INSPECT find/slice/leading count), file I/O (fixed-length records from `_FILES`: OPEN/READ/WRITE/CLOSE with FILE STATUS) and SORT/MERGE (read files, stable sort, write, INPUT/OUTPUT PROCEDURE). CALL raises, as in Python. With `verify.sh` in WSL (Ubuntu-24.04, Mojo 1.2), **12 programs match the Python output exactly: hello, calc, eval, eval2, comp3, features, search1, inspect2, fileio, merge1, sort1, sort2** (no Mojo warnings or errors). **TODO (8)**: sql1 to sql7 (the Python `SQL_HANDLER`, an SQL engine, must be mirrored in Mojo; not started) and misc (Python itself raises NotImplementedError at CALL "GREET", so there is no normal output to compare; it converts, and Mojo stops with the same error). `_num` handles only sign and digits of Python's `int()` (no `1_0` etc.).

## 2026-11-26 cobol2mojo: SQL・CALL を Python 連携で対応(チェックポイント) / SQL and CALL through the Python bridge (checkpoint)
**日本語**: SQL(`SQL_HANDLER`・カーソル・PREPARE)と CALL の文は、Mojo の Python 連携(`std.python`)で、生成元の COBOL_to モジュールのソースを埋め込んで実行する(sqlite3 は Python 側の標準のまま)。項目は文ごとに Mojo⇔Python で同期する。CALL の登録先は `tools/cobol2mojo/c2m_external.py`(`register(EXTERNAL)`)で、`verify.sh` が Python 側・Mojo 側の両方に同じ登録をする。実測: **misc と sql1 は Python 出力と一致**(Mojo の警告・エラーなし)。sql2〜sql7 は変換はできており、20本まとめての再検証が実行中(未確認)。注意: SQL は Mojo 自前ではなく Python の sqlite3 を呼ぶ方式で、純 Mojo の SQL 処理系は未着手。
**再開用メッセージ**: `F:\RMojo` で `python -X utf8 tools/cobol2mojo/cobol_py2mojo.py F:\_work_mojo3\programs F:\_work_mojo4m` を実行し、WSL(Ubuntu-24.04)で `bash /mnt/f/RMojo/tools/cobol2mojo/verify.sh /mnt/f/_work_mojo3 /mnt/f/_work_mojo4m sql2 sql3 sql4 sql5 sql6 sql7` を実行して結果を確認する。DIFF なら `/tmp/c2m_err_<名前>.txt` を見て直す。確認できたものだけを日英で記録して push する。
**English**: SQL statements (`SQL_HANDLER`, cursors, PREPARE) and CALL now run through Mojo's Python interop (`std.python`): the generated COBOL_to module source is embedded and exec'd, so sqlite3 stays on the Python side. Fields are synced between Mojo and Python around each such statement. CALL targets are registered in `tools/cobol2mojo/c2m_external.py` (`register(EXTERNAL)`), and `verify.sh` applies the same registration on both sides. Measured: **misc and sql1 match the Python output** (no Mojo warnings or errors). sql2 to sql7 convert; the full 20-program re-check is still running (unverified). Note: SQL calls Python's sqlite3, not a pure-Mojo SQL engine (not started).
**Resume message**: in `F:\RMojo`, run `python -X utf8 tools/cobol2mojo/cobol_py2mojo.py F:\_work_mojo3\programs F:\_work_mojo4m`, then in WSL (Ubuntu-24.04) run `bash /mnt/f/RMojo/tools/cobol2mojo/verify.sh /mnt/f/_work_mojo3 /mnt/f/_work_mojo4m sql2 sql3 sql4 sql5 sql6 sql7`. For a DIFF, read `/tmp/c2m_err_<name>.txt` and fix. Record only verified results in Japanese and English, then push.

## 2026-11-26 cobol2mojo: 20本すべて一致 / all 20 samples match
**日本語**: 直前のチェックポイントの再検証が終わり、WSL(Ubuntu-24.04、Mojo 1.2)の `verify.sh` で **COBOL_to の試験用20本すべて(hello・calc・eval・eval2・comp3・features・search1・inspect2・fileio・merge1・sort1・sort2・misc・sql1〜sql7)が Python 出力と完全一致**(Mojo の警告・エラーなし)。前節の「sql2〜sql7 は未確認」はこれで解消。ただし SQL と CALL は Mojo の Python 連携で Python 側(sqlite3・登録関数)を呼ぶ方式で、純 Mojo の SQL 処理系は未着手。misc の CALL は試験用の登録先 `c2m_external.py` を使った比較。
**English**: The re-check from the previous checkpoint finished. With `verify.sh` in WSL (Ubuntu-24.04, Mojo 1.2), **all 20 COBOL_to sample programs (hello, calc, eval, eval2, comp3, features, search1, inspect2, fileio, merge1, sort1, sort2, misc, sql1 to sql7) match the Python output exactly** (no Mojo warnings or errors), which resolves the "sql2 to sql7 unverified" note above. SQL and CALL still go through Mojo's Python interop (sqlite3 and the registered functions run in Python); a pure-Mojo SQL engine is not started. misc's CALL is compared using the test registry `c2m_external.py`.

## 2026-11-27 追記: 純 Mojo の SQL への手がかり(FFI で SQLite を直接呼べる) / Lead for a pure-Mojo SQL path (calling SQLite directly over FFI)
**日本語**: SQL を Python の sqlite3 に頼らず Mojo だけで動かす道を調べた。**実測**: WSL(Ubuntu-24.04、Mojo 1.2)で、Mojo から C の `libsqlite3.so.0` を直接呼べた(`from std.ffi import OwnedDLHandle`、`OwnedDLHandle("libsqlite3.so.0").get_function[Int32]("sqlite3_libversion_number")()` が `3045001` を返す)。書き方の落とし穴: `get_function` の型引数は関数型ではなく**戻り値の型**(`get_function[Int32]("名前")`)で、引数は呼ぶときに渡す。これで `sqlite3_open`・`sqlite3_prepare_v2`・`sqlite3_bind_*`・`sqlite3_step`・`sqlite3_column_*` を使う純 Mojo の `SQL_HANDLER` を書けば、Python を介さずに済む見込み。**まだ作っていない理由(正直に)**: 現在の変換は COBOL_to の Python 出力を Mojo へ訳す方式で、SQL の文は `_c, _r = SQL_HANDLER(..)`・`self._cur[名前] = [_r, 0]`・`SQL_NULL` との比較など、Python のタプル展開・辞書・リストを使う形で出る(`COBOL_to/src/convert.rs` の約3500〜3700行)。これらを全部 Mojo の文へ訳す(カーソル・PREPARE・NULL 指示変数を含む)のは大きな作業で、20本の一致を保つ再検証が要る。**次の手**: (1) 純 Mojo の `SQL_HANDLER`(open/exec/行の取得/SQLCODE)を試作して sql1 で Python 版と比較、(2) 変換器にタプル展開・カーソルの辞書の訳を足す。それまで SQL は従来どおり Mojo の Python 連携(20本すべて一致)。
**English**: I looked into running SQL in pure Mojo without Python's sqlite3. **Measured**: in WSL (Ubuntu-24.04, Mojo 1.2), Mojo called the C library `libsqlite3.so.0` directly (`from std.ffi import OwnedDLHandle`; `OwnedDLHandle("libsqlite3.so.0").get_function[Int32]("sqlite3_libversion_number")()` returned `3045001`). Pitfall: the type argument of `get_function` is the **return type**, not a function type (`get_function[Int32]("name")`), and the arguments are passed at the call. A pure-Mojo `SQL_HANDLER` built on `sqlite3_open`, `sqlite3_prepare_v2`, `sqlite3_bind_*`, `sqlite3_step` and `sqlite3_column_*` should therefore avoid Python entirely. **Why it is not built yet (honestly)**: the converter translates COBOL_to's Python output into Mojo, and SQL statements come out as Python tuple unpacking, dicts and lists (`_c, _r = SQL_HANDLER(..)`, `self._cur[name] = [_r, 0]`, comparisons with `SQL_NULL`; about lines 3500-3700 of `COBOL_to/src/convert.rs`). Translating all of that to Mojo statements (cursors, PREPARE, NULL indicator variables) is a large job that needs the 20-program match to be re-verified. **Next**: (1) prototype a pure-Mojo `SQL_HANDLER` (open/exec/fetch/SQLCODE) and compare with the Python version on sql1; (2) teach the converter tuple unpacking and the cursor dict. Until then SQL keeps using Mojo's Python interop (all 20 programs match).

## 2026-11-27 追記(2): 純 Mojo の SQL_HANDLER を試作して動いた / Pure-Mojo SQL_HANDLER prototype works
**日本語**: 前節で「次の手」にした (1) を実施。`tools/cobol2mojo/c2m_sql.mojo` に、C の `libsqlite3.so.0` を FFI で直接呼ぶ純 Mojo の `SqlRt.handler(sql, params) -> SQLCODE`(行は `rt.rows` に文字列で入る。NULL は COBOL_to と同じ `\x00NULL`)を試作した。WSL(Ubuntu-24.04、Mojo 1.2)で `mojo tools/cobol2mojo/c2m_sql.mojo` を実行し、sql1 と同じ文の結果が出た: `CREATE`=0、`COUNT(*)`=3、`SELECT … WHERE ID = ?`=`BOB 20.25`、該当なし=0行(変換側の `_r` が空なら SQLCODE 100 にする既存の規則と同じ形)、カーソル相当の `AMT > 6 ORDER BY ID`=`ROW 1 ALICE` `ROW 2 BOB`、構文エラー=-1(Python 版の `sqlite3.Error` → -1 と同じ)。**Mojo の警告・エラーなし**(`alloc` は使わず `List[Int]` を出力引数の置き場にした)。**まだ**: 変換器への組み込み(タプル展開 `_c, _r = SQL_HANDLER(..)`・カーソルの辞書 `self._cur[名前] = [_r, 0]`・`SQL_NULL` との比較を Mojo の文へ訳す)と、20本の再検証。浮動小数の文字列表現(Python の `str(float)` と SQLite の `column_text`)が桁によって違う可能性は未検証。
**English**: Did step (1) of the previous section's "next steps". `tools/cobol2mojo/c2m_sql.mojo` prototypes a pure-Mojo `SqlRt.handler(sql, params) -> SQLCODE` that calls the C `libsqlite3.so.0` over FFI (rows land in `rt.rows` as strings; NULL is the same `\x00NULL` as COBOL_to). Running `mojo tools/cobol2mojo/c2m_sql.mojo` in WSL (Ubuntu-24.04, Mojo 1.2) reproduced sql1's statements: `CREATE` = 0, `COUNT(*)` = 3, `SELECT … WHERE ID = ?` = `BOB 20.25`, no match = 0 rows (the same shape as the existing rule where an empty `_r` becomes SQLCODE 100), a cursor-like `AMT > 6 ORDER BY ID` = `ROW 1 ALICE` and `ROW 2 BOB`, and a syntax error = -1 (same as the Python version's `sqlite3.Error` → -1). **No Mojo warnings or errors** (no `alloc`; a `List[Int]` holds the output arguments). **Not yet**: wiring it into the converter (translating the tuple unpacking `_c, _r = SQL_HANDLER(..)`, the cursor dict `self._cur[name] = [_r, 0]` and comparisons with `SQL_NULL` into Mojo statements) and the 20-program re-check. Whether Python's `str(float)` and SQLite's `column_text` differ for some magnitudes is unverified.

## 完成まで作り切る方針 / Finish everything policy (2026-11-27)
**日本語**: 全リポジトリで、**未確認は自動で確認し、未実施は実施し、未了は完成させる**(確認を求めず進める)。使用頻度が低いと判断される機能や、**1から作り直す大規模開発になる内容でも、やめずに最後まで開発・TEST・DEBUGする**(別の方式で済ませない、後回しにしない)。進め方: 調査(英語・日本語の検索とGitHub) → 設計 → 実装 → 実機/実データで確認 → 失敗は原因を直して再試験 → README・CLAUDE・PORTING・POLICY を日英で更新 → push。「できた」と書くのは実測した範囲だけ、確認できなかった所は「未確認」と正直に書く。例外(確認を取る): 元に戻せない削除、秘密の公開、所有者の認証・決済・実機操作が要るもの、利用者のシステム設定の変更。
**English**: In every repository, **verify what is unverified, do what is not yet done, and finish what is unfinished** automatically, without asking. Even features that look rarely used, or work that means **a large rewrite from scratch, are developed, tested and debugged to the end** (no substituting another approach, no postponing). Method: research (English and Japanese search plus GitHub) -> design -> implement -> verify on real devices or data -> fix causes and retest on failure -> update README, CLAUDE, PORTING and POLICY in Japanese and English -> push. Write "done" only for what was measured, and say "unverified" honestly for the rest. Exceptions that still need confirmation: irreversible deletions, publishing secrets, anything needing the owner's authentication, payment or real-device actions, and changes to the user's system settings.
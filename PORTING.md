# RMojo PORTING / 再開メモ

## 日本語
**状態**: v0.3.0。Windows x64インストーラー・Android実機(Android 14)・Android TVエミュレーター(i686)で確認済み。Windows ARM64(Snapdragon)ネイティブビルドとRMojo-Setup-arm64.exeをCIに追加(CI成功、実機未確認)。
**未了**: Snapdragon実機、BSD実機、iOS実行パッケージ、リスト多用時の速度(CPythonの約3.5倍遅い)、set内包・ネストしたf-string書式・WSLでのAndroidビルド(Linux版NDKが必要。Windows側cargoでは scripts/android-i686-wsl.sh の代わりに cargo ndk -t x86)。
**再開用メッセージ**: 「`F:\runo\PORTING.md` 末尾の「2026-11-15 別アカウントでの再開用メモ」と、`F:$1\PORTING.md` を読んで、未了の続きを確認なしで開発・TEST・DEBUG・commit・pushして。チャットは日本語のみ。」
**テスト**: python tests/run.py target/release/rmojo (CPythonとの差分 / differential vs CPython)

## English
**Status**: v0.3.0. Verified on the Windows x64 installer, a real Android 14 phone and the Android TV emulator (i686). Native Windows ARM64 (Snapdragon) build and RMojo-Setup-arm64.exe added in CI (CI green, no real device).
**Open**: Snapdragon hardware, BSD hardware, iOS runnable package, list-heavy speed (~3.5x slower than CPython), set comprehensions / nested f-string specs / Android build in WSL (needs a Linux NDK; with Windows cargo use cargo ndk -t x86).
**Resume message**: "Read the 2026-11-15 section at the end of `F:\runo\PORTING.md` and `F:$1\PORTING.md`, then continue the open items without asking (develop, test, debug, commit, push). Chat in Japanese only."
**Tests**: python tests/run.py target/release/rmojo (CPythonとの差分 / differential vs CPython)

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

## 2026-11-16 対応した構文 / Newly supported syntax
**日本語**: 集合型(`{1,2}`・集合内包表記・`set()`・`in`/`len`/反復・`add`/`remove`/`discard`/`clear`/`union`/`intersection`/`difference`)と連鎖代入(`a = b = v`、名前のみ)を追加。CPython と出力が一致(`tests/t8_sets.py`・`t9_chain.py`、差分0)。**未対応**: `try/else`、引数なしの `raise`、dict の `**` 展開、f文字列の入れ子の書式指定、集合の演算子(`|` `&` `-`)。
**English**: Added the set type (literals, set comprehensions, `set()`, `in`/`len`/iteration, `add`/`remove`/`discard`/`clear`/`union`/`intersection`/`difference`) and chained assignment (`a = b = v`, names only); output matches CPython (`tests/t8_sets.py`, `t9_chain.py`, 0 differences). **Still unsupported**: `try/else`, bare `raise`, `**` dict unpacking, nested f-string format specs, set operators (`|` `&` `-`).

## 2026-11-16 追加対応(続き) / More syntax supported
**日本語**: 集合演算子(`|` `&` `^` `-`)、整数のビット演算(`|` `&` `^` `<<` `>>`)、`try/else`、引数なし `raise`(再送出)、dict の `**` 展開、f文字列の入れ子の書式指定(`{x:{w}.{p}f}`)に対応。`tests/t10_flow.py`・`t11_misc.py` を追加し CPython との差分0。**`except` は型で振り分け**(`except A:` / `except (A, B):` / `Exception`・`ArithmeticError`・`LookupError`、`tests/t12_except.py`)。型が合わなければ外側へ伝播。**制限**: 利用者定義の例外クラスの継承は未対応、`raise` の再送出は直近の例外。
**English**: Added set operators (`|` `&` `^` `-`), integer bit operations (`|` `&` `^` `<<` `>>`), `try/else`, bare `raise` (re-raise), `**` dict unpacking and nested f-string format specs (`{x:{w}.{p}f}`). `tests/t10_flow.py` and `t11_misc.py` added; 0 differences vs CPython. **`except` now dispatches by type** (`except A:`, `except (A, B):`, `Exception`/`ArithmeticError`/`LookupError`; `tests/t12_except.py`); unmatched errors propagate outward. **Limits**: user-defined exception class inheritance is unsupported, and a bare `raise` re-raises the most recent exception.

## 2026-11-16 速度と例外の継承 / Speed and exception inheritance
**日本語**: `list.append` を専用の高速経路にし、素数ふるい(`tests/bench_list.py`、300万要素)が CPython の約3.5倍遅い → **約1.2倍**になった。利用者定義の例外クラスの継承に対応(`class NotFound(AppError)` を `except AppError:` / `except Exception:` が受ける、`tests/t13_userexc.py`、CPython と差分0)。**未了**: Snapdragon・BSD・iOS の実機/実行パッケージ(実機が手元に無い)、`while` と添字代入が中心のループは CPython の約1.5倍遅い(バイトコード化が要る)、WSL での Android ビルド。
**English**: `list.append` got a dedicated fast path: the sieve benchmark (`tests/bench_list.py`, 3M elements) went from ~3.5x slower than CPython to **~1.2x**. User-defined exception class inheritance is supported (`class NotFound(AppError)` is caught by `except AppError:` / `except Exception:`; `tests/t13_userexc.py`, 0 differences vs CPython). **Open**: Snapdragon / BSD / iOS hardware or runnable packages (no devices at hand); `while` + index-assignment heavy loops are ~1.5x slower than CPython (needs a bytecode stage); Android build in WSL.

## 2026-11-17 到達点 / Status
**日本語**: v0.4.1 公開(macOS/Linux x64+arm64、Android aarch64、`RMojo-Setup-x64/arm64.exe`)。Windows・Android実機・TVエミュレーター(i686)で確認済み。`sorted(set/tuple/dict/str)`を修正(`tests/t8_sorted_iter.py`)。未了: 実機Snapdragon、WSLでのLinux版NDKビルド。
**English**: v0.4.1 released (macOS/Linux x64+arm64, Android aarch64, Windows x64/arm64 installers); verified on Windows, a real Android phone and the Android TV (i686) emulator. `sorted()` over set/tuple/dict/str fixed. Open: real Snapdragon hardware, Linux-NDK build inside WSL.

## open-cpu の同梱と、アクセラレーターの扱い(2026-11-17)
- **同梱済み(実行時に AVX-512BW > AVX2 > NEON > スカラーを自動選択、`OPEN_CPU_MAX_ISA`で上限指定可)**: MFastAPI(`--cpu`、長いクエリ文字列の探索)、RMojo(`rmojo cpu`、`str.find`/`str.count`)、open-web-server・open-easy-web(起動時の要約、`System/GetCpu`、FastCGIヘッダー走査)。実機はNEONをSnapdragon端末2台で、AVX-512BWはCIで確認(開発PCはAVX2まで)。
- **2FastAPI**: 変換ツール(Python)で、実行時の計算処理を持たないため open-cpu は使わない。
- **open-cuda / aruaru-llm / open-directx は同梱しない**: 共有基盤を1回だけVPSに入れ、各サイトはConnect/gRPCで呼ぶ方針(本ファイル冒頭の配備アーキテクチャ方針)に従う。
- **ブラウザ(Windows/Linux/Mac/Android/iPhone)**: 計算はサーバー側で行うため、どの端末のブラウザからでも利用できる。RMojoのWASM(`wasm32-unknown-unknown`の`cargo check`は通過)は将来の遊び場候補。WASMでは open-cpu のSIMD自動選択は使わない。
- **スマホのアクセラレーター化**: `open-cpu/remote`(`open-cpu-remote`)。実測では手元PCが速いため自動判断は「手元のみ」で、手元が非力な場合に効く。詳細は`open-cpu/remote/README.md`。

## 起動時の自動バージョンアップ(2026-11-17)/ Self-update at startup
**日本語**: 起動のたびに前回の確認から24時間以上たっていれば、切り離した子プロセスで GitHub の最新リリースを調べ、新しければ取得して入れ替える(起動は遅くならない。更新は次回の起動から有効で、その旨を一度だけ表示)。**リリースの `<ファイル名>.sha256` と一致したときだけ**入れ替え、`.sha256` が無いリリース・GitHub 以外の取得先・先行版(-alpha 等)へは更新しない。信頼の根拠は GitHub への TLS と SHA-256(電子署名ではない)。手動実行と停止は下表。失敗は黙って次回に回し、理由は状態ディレクトリの `last_error` に残る。
**English**: On each start, if 24 h have passed since the last check, a detached child process queries the latest GitHub release and, if newer, downloads and swaps it in (startup is not delayed; the update takes effect on the next start, announced once). It installs **only if the SHA-256 matches the release's `<file>.sha256`**; releases without `.sha256`, non-GitHub hosts and pre-releases are refused. Trust = TLS to GitHub + SHA-256 (not a signature). Failures are silent and recorded in `last_error` in the state directory.
- RMojo: Linux/macOS/Android は `tar.gz` の `rmojo` を同じ場所へ rename で入れ替え(実行中でも安全)、Windows は公式 `RMojo-Setup-<arch>.exe` を静かに実行(利用者権限)。手動: `rmojo update`、停止: `RMOJO_NO_UPDATE=1`(`CI` 設定時も停止)。状態: `~/.rmojo/`。Windows 実機で v0.4.3 のダウンロード→SHA-256 検証→Setup 静か実行→アンインストールまで確認済み(2026-11-17)。

## 上流の新版への追従(2026-11-17)/ Following upstream releases
- **定期試験**: CI を毎日(UTC 03:23)実行し、最新の Python・Flask・Django・FastAPI・Starlette(RMojo は最新の CPython との差分試験)で再試験。失敗したら日付つきの issue を1件起こす。
- **easy-web.tokyo**: `runo/scripts/refresh_versions.py` を VPS のタイマー(`easy-web-intros-refresh.timer`、1時間ごと)で実行し、MFastAPI・RMojo・2FastAPI・2FastAPI-VSC の最新版と上流(Python・Flask・Django・FastAPI・Starlette・Rust・Mojo)の最新版を各紹介ページと `/demo` に表示(`<製品>/versions.json`)。ページはダウンロード専用のまま。
- **未了**: 上流の新版が出たときに、利用者の環境(Flask/Django 等)まで自動で上げる機能は無い(本製品が更新するのは本製品自身)。aruaru-llm の同梱は未着手(open-cpu は同梱済み)。iPhone・iPad には対応していない。

## 他OSでの自動更新の試験(2026-11-17)/ Updater tests on other OSes
**日本語**: Android実機(arm64、Android 14)で、版を0.0.1に偽装したビルドが `rmojo update` で実リリース v0.4.3 を取得→SHA-256検証→置換し、次回起動で更新通知が出ることを確認(`cargo ndk -t arm64-v8a`、`adb`、Git Bashでは `MSYS_NO_PATHCONV=1` が要る)。macOS(x64/arm64)・Linux(x64/arm64)は CI の `update-test` ジョブ(版を0.0.1に偽装→実リリースへ更新→通知まで)で毎回確認。**未了**: iOS(実行パッケージが無く対象外)、Snapdragon/BSD実機。
**English**: On a real Android 14 arm64 phone, a build disguised as 0.0.1 ran `rmojo update`, fetched the real v0.4.3, verified SHA-256, replaced itself, and announced the update on the next start (`cargo ndk -t arm64-v8a`; Git Bash needs `MSYS_NO_PATHCONV=1` for adb). macOS (x64/arm64) and Linux (x64/arm64) are covered on every CI run by the `update-test` job. **Open**: iOS (no runnable package), Snapdragon/BSD hardware.

## 2026-11-18 追記 / Addendum (v0.4.4)
**日本語**: 更新の競合を修正。起動時に裏で動く更新ワーカーと明示の`rmojo update`が同じ`rmojo.new`を奪い合い、`No such file`で失敗した(CIのLinux/macOSで再現)。作業ファイル名にPIDを入れて解消、GitHub APIが取れない(レート制限)ときは`releases/latest`のリダイレクトで代替、失敗箇所をエラーに明記。`updater-e2e`ワークフロー(週次・手動)を追加し、Linux x64/arm64・macOS arm64で実リリースへの更新を確認。macOS Intelはランナーが動かず未確認。**再開用メッセージ**: 「`F:\RMojo` を開き `cargo test --release` で確認、PORTING.mdの未了(Snapdragon/BSD実機、macOS Intel)を続けて。」
**English**: Fixed an updater race: the background worker spawned at startup and an explicit `rmojo update` fought over the same `rmojo.new` (reproduced in CI on Linux/macOS). The staged name now includes the PID; if the GitHub API fails (rate limit) it falls back to the `releases/latest` redirect; errors name the failing step. A weekly/manual `updater-e2e` workflow verifies updating to the real release on Linux x64/arm64 and macOS arm64 (macOS Intel: runner never started, unverified). **Resume**: open `F:\RMojo`, run `cargo test --release`, continue with Snapdragon/BSD hardware and macOS Intel.

## 2026-11-18 追記2 / Addendum 2
**日本語**: **Snapdragon実機**: M06(SM7435=Snapdragon 7s Gen 2、Cortex-A55×4+A78×4)で v0.4.4 を実行し、`rmojo cpu`(neon/dotprod/lse等)と、版を0.0.1に偽装した `cargo ndk` ビルドの `rmojo update` → 実リリース v0.4.4 への置換を確認。**BSD**: GitHub の FreeBSD 14.2 仮想マシン(`freebsd.yml`、週次・手動)でビルド・全11試験・`rmojo cpu` が成功。BSD向けの配布物は無いので、更新は何もしないのが正しい動作(試験もそう直した)。実機のBSDは未確認。
**English**: **Snapdragon hardware**: on the M06 (SM7435 = Snapdragon 7s Gen 2, Cortex-A55 x4 + A78 x4) v0.4.4 ran `rmojo cpu` (neon/dotprod/lse...), and a `cargo ndk` build disguised as 0.0.1 updated itself to the real v0.4.4 with `rmojo update`. **BSD**: the build, all 11 tests and `rmojo cpu` pass in a GitHub FreeBSD 14.2 VM (`freebsd.yml`, weekly/manual). There is no BSD release asset, so the updater correctly does nothing there (the test now says so). Real BSD hardware is unverified.

## 2026-11-22 追記: FreeBSD 14.5 での試験 / FreeBSD 14.5 test
**日本語**: Windows 上の QEMU(WHPX)で起動した FreeBSD 14.5-RELEASE で、`cargo test --release` 11件成功、`rmojo update` は「最新です(0.4.4)」。BSD 用のリリース添付は無いので更新機能は何もしないのが正しい。詳細は `F:\runo\PORTING.md` 末尾。実機 BSD は未確認。
**English**: On FreeBSD 14.5-RELEASE booted in QEMU (WHPX) on Windows, `cargo test --release` passes 11 tests and `rmojo update` prints "up to date (0.4.4)". With no BSD release asset the updater correctly does nothing. See the end of `F:\runo\PORTING.md`. Real BSD hardware is unverified.

## 2026-11-23 追記: MFastAPI からの利用設計
RMojo を FastAPI 互換サーバー(MFastAPI)のハンドラ実行系にする設計を `MFastAPI/DESIGN-MOJO.md` に書いた。RMojo 側の必要作業は(1)`rmojo-core` へのライブラリ化(動作を変えない分割、`tests/run.py` 差分0)、(2)ステップ/時間/メモリ上限(実測で無限ループ・メモリ増加は現状止まらない、再帰のみ上限あり)、(3)`Program::call` API。実装は未着手。

## 2026-11-23 追記: ライブラリ化(段階1) / Library split (stage 1)
**日本語**: `src/main.rs` を `src/lib.rs`(コア)と薄い `main.rs` に分け、公開API `compile_source(src)` / `CompiledProgram::call(name, &[Value])`(`Value`=Int/Float/Bool/Str/Null)を追加。CPython との差分0(`tests/run.py`)・単体11件・`tests/api.rs` 成功。**未了**: `Limits`(ステップ数・時間・メモリ。`while True` と `list.append` は5秒でも止まらない実測のため新規実装が要る)、リスト/辞書の値、MFastAPI(`runtime:"rmojo"`)からの呼び出し。設計は `MFastAPI/DESIGN-MOJO.md`。
**English**: `src/main.rs` is split into `src/lib.rs` (core) and a thin `main.rs`, with a public API `compile_source(src)` / `CompiledProgram::call(name, &[Value])`. Zero differences against CPython, 11 unit tests and `tests/api.rs` pass. **Open**: `Limits` (steps/time/memory; infinite loops are not stopped today), list/dict values, and calling from MFastAPI (`runtime:"rmojo"`).

## 2026-11-23 追記: ステップ上限(段階2の一部) / Step limit
**日本語**: `Limits{max_steps}` と `CompiledProgram::call_limited` を追加。while/for(型付き整数パスを含む)の反復ごとに1減らし、尽きると `LimitError: step limit exceeded`。無限の `while True`(list.append・型付き整数)が止まることを試験で確認、CPython との差分0。**未了**: 経過時間・メモリ上限、再帰以外の大きな単発処理(巨大な `range`・文字列の掛け算)、ForIn の反復、MFastAPI からの呼び出し。
**English**: Added `Limits{max_steps}` and `call_limited`: each while/for iteration (including the typed-int path) costs one step; at zero the call fails with `LimitError`. Infinite loops now stop (tested); zero differences vs CPython. **Open**: wall-time and memory limits, large one-shot operations, ForIn iterations, and the MFastAPI call path.

## 2026-11-23 追記: 時間上限(段階1の続き) / Wall-time limit
**日本語**: `Limits` に `max_ms` を追加(ループ1024回ごとに時計を確認、超過で `LimitError: time limit exceeded`)。無限ループ(整数・list.append)が100msで止まる試験を追加、`cargo test` 14件成功、CPython との差分0。**未了**: メモリ上限、巨大な `range`・文字列の掛け算など反復を伴わない単発処理、MFastAPI からの呼び出し(段階2: `rmojo-py`)。
**English**: Added `Limits.max_ms` (clock checked every 1024 loop iterations; `LimitError: time limit exceeded`). Infinite loops stop at 100 ms (tested); all tests pass, zero diffs vs CPython. **Open**: memory limit, one-shot heavy operations, and the MFastAPI call path (stage 2).

## 2026-11-24 メモリ上限 / Memory caps
上記の上限を追加。`cargo test` 全成功。MFastAPI の段階0の未了(メモリ上限)は、要素数の上限で一部解消(RSS 上限は未了)。次は段階2 `rmojo-py`。
Caps added; `cargo test` passes. MFastAPI stage-0's memory item is partly covered by the element cap (RSS cap open). Next: stage 2 `rmojo-py`.

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

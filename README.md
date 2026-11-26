> Latest translations / 翻訳の最新版: [0-LATEST-TRANSLATIONS.md](0-LATEST-TRANSLATIONS.md)

# RMojo

**日本語**: RMojo — Mojoとの互換性を重視した言語。見た目はPython構文、中身はRust(C++なし)。旧 MojoW / MojoL / MojoM / MojoA / Mojoi(旧称 MojoU)を1つに統合したもの。対象: Windows・Linux・UNIX・macOS・TV(アプリのダウンロード機能がある場合)・Android・iPhone・iPad(**目標**。検証済みの範囲は下の表と制限を参照)。
プラットフォームごとの違いは**インストーラーだけ**(ソースは共通)。

**English**: RMojo - a language that aims for Mojo compatibility: Python syntax on the surface, memory-safe Rust inside (no C++). Successor of MojoW / MojoL / MojoM / MojoA / Mojoi (formerly named MojoU). Targets: Windows, Linux, UNIX, macOS, TV (where apps can be downloaded), Android, iPhone, iPad (goals; see the table and limits for what is verified). Platforms differ only in the installer.

```
rmojo run hello.py        rmojo --version
```

## Installers
| Platform | Installer | Status |
|---|---|---|
| Windows x64 | `installers/windows/rmojo.iss` -> `RMojo-Setup-x64.exe` (per-user, PATH) | **v0.4.1 のインストール/実行/アンインストール試験OK** (PATH追加・削除も確認) |
| Windows ARM64 (Snapdragon / Copilot+ PC) | `rmojo.iss /DArch=arm64` -> `RMojo-Setup-arm64.exe` (ネイティブ aarch64-pc-windows-msvc) | CIでビルド+差分試験(windows-11-arm)。実機(Snapdragon)は未確認 |
| Linux x86_64 / aarch64 | `installers/linux/install.sh` (tar.gz) | built by CI |
| macOS Intel / Apple silicon | `installers/macos/install.sh` (tar.gz) | built by CI, unsigned |
| Android aarch64 | `installers/android/install.sh` (Termux / adb) | **実機で確認** (moto g53y, Android 14: v0.4.1 の version / .mojo / .py 実行OK) |
| iOS | `installers/ios/README.md` | compile check only |

`compat/wsl-bridge/` keeps the old MojoW feature (run the *official* Mojo inside WSL from Windows; unofficial).

## Scope / limits (honest)
Subset of Python syntax; 64-bit integers (overflow is an error); no lists/dicts/classes/closures yet. Typed `int -> int` functions are compiled
(about 1.6-1.9x faster than CPython 3.14 on fib/collatz); untyped code is slower than CPython. Tests: `tests/run.py` compares output with CPython (differences=0).

**Mojo互換 / Mojo compatibility (honest)**: 現状は Python構文のサブセットで、Mojoの `fn` / `struct` / `var` / SIMD 等は未対応。互換はこれから広げる目標。 / Currently a Python-syntax subset; Mojo's `fn` / `struct` / `var` / SIMD are not supported yet. Compatibility is a goal to be widened. 未検証: UNIX(BSD等)、TV、iOS/Androidの実機。 / Unverified: BSD/UNIX, TV, real iOS/Android devices.

**Mojo構文 / Mojo syntax**: `.mojo` ファイルで次が動く(`src/mojo_syntax.rs`、テストは `tests/mojo_*.mojo` と `*.expected`)。 / Working in `.mojo` files:
- `fn` / `var` / `let` / `alias` / 引数規約(`inout` 等) / 型名(Int, Float64, String, Bool)
- `struct`(`var`フィールド、`fn`メソッド、`__init__`、`__str__`=`print`に反映、`@value`=フィールド順のコンストラクタ)
- **値渡し(コピー)**: `var b = a` はリスト・辞書・structを深いコピーにする(`.py`ファイルは従来どおり参照)
- `trait` ブロックと適合リスト `struct P(Shape)` は受け付けるが**適合の検査はしない**(ダックタイピング)
- `List[T](...)` / `Dict[K, V]()` / `SIMD[T, n](...)`(要素ごとの演算、`reduce_add`、添字。**SIMD命令ではなく通常の計算**であり高速化はしない)
- リスト・辞書・文字列メソッド(append/pop/insert/sort/get/keys/values/items/split/join等)、`for x in リスト/辞書/文字列`、`in`、`len/sum/sorted/list`(`tests/t3_collections.py`はCPythonと差分0)
- **未対応**: スライス、タプル型、パラメータ化(`fn f[T: ...]`・`struct S[T]`)、他のデコレーター、ポインタ、Mojoモジュールのimport、`def`の引数型検査(型は注釈のみで検査しない)、整数は64bit。 / Not supported: slices, tuples as a type, parametric fn/struct, other decorators, pointers, Mojo module imports, type checking of annotations; integers are 64-bit.

**BSD/UNIX**: FreeBSD・NetBSD・illumos向けに `cargo check` が通る(`scripts/bsd-check.sh`)。実機・VMでの実行は未確認。 / `cargo check` passes for FreeBSD, NetBSD and illumos; not run on real systems. **Android TV**: エミュレーター(Android 13 TV x86、leanback)で v0.4.1 の `.mojo`/`.py` 実行を確認(i686-linux-android ビルド)。TV実機は未確認。 / **iPhone/iPad**: コンパイル確認のみ。

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

## RMojo - 多言語の説明 / Multilingual description (13 languages)

公開ページ / Pages: https://easy-web.tokyo/RMojo ・ https://easy-web.tokyo/RMojo/demo (ダウンロード専用 / download-only)

- **日本語**: Rustで書かれた、MojoスタイルとPythonスタイルのプログラムを実行するインタープリタ。Windows・Android・Linux・macOS向け。
- **English**: An interpreter written in Rust that runs Mojo-style and Python-style programs, for Windows, Android, Linux and macOS.
- **中文(简体)**: 使用 Rust 编写的解释器,可运行 Mojo 风格和 Python 风格的程序,支持 Windows、Android、Linux 和 macOS。
- **中文(台灣・繁體)**: 以 Rust 撰寫的直譯器,可執行 Mojo 風格與 Python 風格的程式,支援 Windows、Android、Linux 與 macOS。
- **한국어**: Rust로 작성된 인터프리터로, Mojo 스타일과 Python 스타일의 프로그램을 실행합니다. Windows, Android, Linux, macOS를 지원합니다.
- **Italiano**: Un interprete scritto in Rust che esegue programmi in stile Mojo e in stile Python, per Windows, Android, Linux e macOS.
- **Français**: Un interpréteur écrit en Rust qui exécute des programmes de style Mojo et de style Python, pour Windows, Android, Linux et macOS.
- **Deutsch**: Ein in Rust geschriebener Interpreter, der Programme im Mojo- und im Python-Stil ausführt, für Windows, Android, Linux und macOS.
- **Русский**: Интерпретатор на Rust, выполняющий программы в стиле Mojo и Python, для Windows, Android, Linux и macOS.
- **Українська**: Інтерпретатор на Rust, що виконує програми у стилі Mojo та Python, для Windows, Android, Linux і macOS.
- **فارسی (ایران)**: مفسری نوشته‌شده با Rust که برنامه‌هایی به سبک Mojo و پایتون را اجرا می‌کند؛ برای Windows، Android، Linux و macOS.
- **العربية**: مفسِّر مكتوب بلغة Rust يشغّل برامج بأسلوب Mojo وأسلوب Python، لأنظمة Windows وAndroid وLinux وmacOS.
- **עברית (ישראל)**: מפרש שנכתב ב-Rust ומריץ תוכניות בסגנון Mojo ובסגנון פייתון, עבור Windows, Android, Linux ו-macOS.

Download: https://github.com/aon-co-jp/RMojo/releases/latest

## Windows インストーラー方針 / Windows installer policy
**日本語**: インストーラーを持つ製品の Windows 版は、すべて `.exe`(Inno Setup/NSIS、必要なら PyInstaller 同梱)で配布する。PowerShell スクリプトは開発用に残すだけで、利用者向けの入口にしない。RMojo は `RMojo-Setup-x64/arm64.exe` を配布済み、2FastAPI は v0.3.1 から `2FastAPI-Setup-x64/arm64.exe`。MFastAPI は pip の wheel 配布でインストーラーを持たない(Windows でも `pip install mfastapi`)。
**English**: Every product that has an installer ships its Windows build as an `.exe` (Inno Setup/NSIS, with PyInstaller bundling where needed); PowerShell scripts stay for development only. RMojo ships `RMojo-Setup-x64/arm64.exe`; 2FastAPI ships `2FastAPI-Setup-x64/arm64.exe` since v0.3.1. MFastAPI has no installer (wheel only: `pip install mfastapi`, also on Windows).

**2026-11-16**: 集合・連鎖代入・try/else・bare raise・`**`展開・入れ子書式・型別except・例外クラス継承に対応、リスト速度は CPython の約1.2倍まで改善(詳細 `PORTING.md`)。 / Added sets, chained assignment, try/else, bare raise, `**` unpacking, nested format specs, typed except and exception inheritance; list speed is now ~1.2x of CPython (see `PORTING.md`).

**2026-11-23**: 組み込み用 API(`compile_source` / `call_limited`)に、ループのステップ上限と時間上限(`Limits{max_steps,max_ms}`)を追加。メモリ上限は未了。 / Embedding API now has step and wall-time limits; memory limit is open.

### 2026-11-24 メモリ上限 / Memory caps
リスト・文字列の増殖(`append`・`*`・`range`)は1つあたり2,000万要素(文字列は×8バイト)で `MemoryError` として止まります(組み込みでも CLI でも)。時間上限と合わせて暴走を防ぎます。
Lists and strings (`append`, `*`, `range`) stop with `MemoryError` at 20M elements (strings: x8 bytes), in both the embedding API and the CLI, on top of the time limit.

## 2026-11-25 現況・未了 / Status and open items

*日本語・English が基本、ほかの言語は AI 翻訳(ネイティブ確認前)。 / Japanese and English are the base; other languages are AI-translated, not native-checked.*

### 日本語

現状: Mojo との互換を目指す言語。見た目は Python 構文、中身は Rust(C++ なし)。最近は実行の上限を追加した(ループ回数、時間 Limits.max_ms、リスト・文字列・range の要素数の上限で MemoryError)。POLICY.md も新設した。残り: Mac・iPhone の実機での確認(ここには実機がありません)。翻訳: AI 翻訳(ja/en 以外)、ネイティブの確認はまだです。

### English

Status: a language aiming at Mojo compatibility: Python syntax on the surface, Rust inside (no C++). Recently it gained execution limits (loop steps, a time limit Limits.max_ms, and element-count caps on lists, strings and ranges that raise MemoryError). POLICY.md was also added. Open: checks on a real Mac and iPhone (no such devices here). Translations: AI-translated (other than ja/en), not yet checked by native speakers.

### 简体中文

现状:以兼容 Mojo 为目标的语言:外观是 Python 语法,内部是 Rust(没有 C++)。最近新增了执行上限(循环步数、时间上限 Limits.max_ms,以及列表、字符串、range 的元素数上限,超出时抛出 MemoryError)。同时新增了 POLICY.md。待办:在真实的 Mac 和 iPhone 上验证(这里没有这些设备)。翻译:由 AI 翻译,尚未经母语者确认。

### 繁體中文

現況:以相容 Mojo 為目標的語言:外觀是 Python 語法,內部是 Rust(沒有 C++)。最近新增了執行上限(迴圈步數、時間上限 Limits.max_ms,以及清單、字串、range 的元素數上限,超出時擲出 MemoryError)。同時新增了 POLICY.md。待辦:在實體 Mac 和 iPhone 上驗證(這裡沒有這些裝置)。翻譯:由 AI 翻譯,尚未經母語人士確認。

### 한국어

현황: Mojo 호환을 목표로 하는 언어로, 겉모습은 Python 문법이고 내부는 Rust(C++ 없음)입니다. 최근 실행 상한(반복 횟수, 시간 Limits.max_ms, 리스트·문자열·range의 요소 수 상한 초과 시 MemoryError)을 추가했고 POLICY.md도 새로 만들었습니다. 남은 일: 실제 Mac과 iPhone에서의 확인(여기에는 해당 기기가 없습니다). 번역: AI 번역이며 원어민의 확인은 아직 받지 않았습니다.

### Deutsch

Stand: eine Sprache mit dem Ziel der Mojo-Kompatibilität: außen Python-Syntax, innen Rust (ohne C++). Zuletzt kamen Ausführungslimits hinzu (Schleifenschritte, Zeitlimit Limits.max_ms sowie Obergrenzen für Elemente von Listen, Strings und ranges mit MemoryError). Außerdem wurde POLICY.md neu angelegt. Offen: Prüfung auf einem echten Mac und iPhone (solche Geräte haben wir hier nicht). Übersetzungen: KI-übersetzt, noch nicht von Muttersprachlern geprüft.

### Français

État : un langage visant la compatibilité avec Mojo : syntaxe Python en surface, Rust à l'intérieur (sans C++). Récemment, il a reçu des limites d'exécution (pas de boucle, limite de temps Limits.max_ms, plafonds du nombre d'éléments des listes, chaînes et range avec MemoryError). POLICY.md a aussi été créé. Reste à faire : vérifier sur un vrai Mac et un vrai iPhone (nous n'avons pas ces appareils ici). Traductions : faites par IA, pas encore relues par des locuteurs natifs.

### Español

Estado: un lenguaje que busca la compatibilidad con Mojo: sintaxis de Python por fuera y Rust por dentro (sin C++). Hace poco recibió límites de ejecución (pasos de bucle, límite de tiempo Limits.max_ms y topes de elementos en listas, cadenas y range, con MemoryError). También se creó POLICY.md. Pendiente: comprobar en un Mac y un iPhone reales (aquí no tenemos esos dispositivos). Traducciones: hechas por IA, aún sin revisión de hablantes nativos.

### Português

Situação: uma linguagem que busca compatibilidade com Mojo: sintaxe Python por fora e Rust por dentro (sem C++). Recentemente ganhou limites de execução (passos de laço, limite de tempo Limits.max_ms e tetos de elementos em listas, strings e range, com MemoryError). O POLICY.md também foi criado. Pendente: verificar em um Mac e um iPhone de verdade (não temos esses aparelhos aqui). Traduções: feitas por IA, ainda não revisadas por falantes nativos.

### Русский

Состояние: язык, нацеленный на совместимость с Mojo: снаружи синтаксис Python, внутри Rust (без C++). Недавно добавлены ограничения выполнения (число шагов цикла, лимит времени Limits.max_ms, предел числа элементов списков, строк и range с ошибкой MemoryError). Также создан POLICY.md. Осталось: проверка на настоящих Mac и iPhone (таких устройств у нас здесь нет). Переводы: выполнены ИИ, носители языка их пока не проверяли.

### العربية

الوضع: لغة تستهدف التوافق مع Mojo: صياغة Python من الخارج وRust من الداخل (بلا C++). أضيفت مؤخرًا حدود للتنفيذ (عدد خطوات الحلقة، وحد زمني Limits.max_ms، وسقف لعدد عناصر القوائم والسلاسل وrange مع MemoryError). كما أُنشئ ملف POLICY.md. المتبقي: التحقق على جهازَي Mac وiPhone حقيقيين (لا تتوفر لدينا هذه الأجهزة هنا). الترجمات: أنجزها الذكاء الاصطناعي ولم يراجعها بعدُ متحدثون أصليون.

### فارسی

وضعیت: زبانی با هدف سازگاری با Mojo: ظاهر آن نحو Python و درونش Rust (بدون C++) است. اخیراً محدودیت‌های اجرا افزوده شد (تعداد گام حلقه، حد زمانی Limits.max_ms و سقف تعداد عناصر فهرست، رشته و range همراه با MemoryError). فایل POLICY.md نیز ساخته شد. باقی‌مانده: بررسی روی Mac و iPhone واقعی (این دستگاه‌ها را اینجا نداریم). ترجمه‌ها: با هوش مصنوعی انجام شده و هنوز گویشوران بومی آن‌ها را بررسی نکرده‌اند.

<!-- directive-20261125-1130 -->
## 2026-11-25 指示(11:30 まで確認不要) / Directive (no confirmation until 11:30)
**日本語**: 2026-11-25 11:30 まで、未了は確認なしで完成まで自動で進める。需要が少ない・必要性が低いと判断される、あるいは1から大規模に作り直す必要があっても、別の方式で済ませず、正面から作り直す。同時に開発する対象: COBOL_to・Java_to・MFastAPI・2MFastAPI(他の言語・フレームワークから MFastAPI 互換へ。Node.js・TypeScript は例外的に選べ、選ばない選択も可能)・RMojo・RPoRea(RPoem と Reaxt の統合)・2RPoRea(他の言語・フレームワークから RPoRea へ)・Reaxt(React+Next.js+Nuxt.js+Nest.js の統合)。1つ変えたら他を確認し、README・CLAUDE・PORTING・POLICY を日英で同じ日に更新する。
**English**: Until 2026-11-25 11:30, carry unfinished work through to completion automatically without asking. Even if demand looks low or a large from-scratch rewrite is needed, do not substitute another approach; do the real work. Developed together: COBOL_to, Java_to, MFastAPI, 2MFastAPI (converts other languages and frameworks to MFastAPI-compatible code; Node.js and TypeScript are an optional exception and choosing neither is allowed), RMojo, RPoRea (RPoem and Reaxt integrated), 2RPoRea (converts other languages and frameworks to RPoRea) and Reaxt (React + Next.js + Nuxt.js + Nest.js integrated). When one changes, check the others and update README, CLAUDE, PORTING and POLICY in Japanese and English the same day.

<!-- directive-20261125-mojo -->
## 2026-11-25 追記: MFastAPI は Mojo から使えること / Addendum: MFastAPI must be usable from Mojo
**日本語**: MFastAPI は **Mojo から使用できる仕様**を保つ(Python 構文・Rust 実装・Mojo 対応、Windows では RMojo の `compat/wsl-bridge` 経由で WSL 上の Mojo を使う)。2MFastAPI の変換結果も Mojo から使えること。現状: `mojo/` は参照実装で、段階B・Cのサーバーは実験的。Mojo からの呼び出しの実機確認は、確認済みの範囲だけを「確認済み」と書く。11:30 まで確認なしで自動で進める。
**English**: MFastAPI keeps the **specification of being usable from Mojo** (Python syntax, Rust core, Mojo support; on Windows the real Mojo in WSL is driven through RMojo's `compat/wsl-bridge`). Output of 2MFastAPI must be usable from Mojo too. Current state: `mojo/` is a reference implementation and its stage B/C servers are experimental. Only what was actually run is called "verified". Continue automatically without asking until 11:30.

## 2026-11-25 追記: COBOL→Mojo 変換(最小範囲) / COBOL to Mojo (small scope)
**日本語**: `tools/cobol2mojo/cobol_py2mojo.py` を追加。COBOL_to の Python 出力(`--target fastapi` の `programs/*.py`)を読み、整数・文字列の項目、段落、PERFORM、IF、WHILE、加減乗算、`_tdiv`/`_rdiv`/`_fmt`、DISPLAY、GO TO、STOP RUN だけを `.mojo` に変換する。それ以外は変換せず TODO として報告する。WSL(Ubuntu-24.04、`/root/mojo-venv`、Mojo 1.2)で `tools/cobol2mojo/verify.sh` を使い、COBOL_to の試験用20本のうち **hello・calc・eval・eval2 の4本で Python 出力と完全一致**(警告なし、1本あたり約7〜11秒。ほぼコンパイル時間)。**TODO(16本)**: comp3・features・search1(表=リスト項目)、fileio・merge1・sort1・sort2・sql1(ファイル入出力)、sql2〜sql7(SQL)、inspect2(INSPECT)、misc(DISPLAY の別形式)。
**English**: Added `tools/cobol2mojo/cobol_py2mojo.py`. It reads COBOL_to's Python output (`programs/*.py` from `--target fastapi`) and converts only integer/string fields, paragraphs, PERFORM, IF, WHILE, + - *, `_tdiv`/`_rdiv`/`_fmt`, DISPLAY, GO TO and STOP RUN into `.mojo`; anything else is reported as TODO, not converted. Verified in WSL (Ubuntu-24.04, `/root/mojo-venv`, Mojo 1.2) with `tools/cobol2mojo/verify.sh`: of COBOL_to's 20 sample programs, **hello, calc, eval and eval2 match the Python output exactly** (no warnings; about 7 to 11 s each, mostly compile time). **TODO (16)**: comp3, features, search1 (tables/list fields), fileio, merge1, sort1, sort2, sql1 (file I/O), sql2 to sql7 (SQL), inspect2 (INSPECT), misc (another DISPLAY form).

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

## 2026-11-27 追記: SQL を純 Mojo で変換(Python 連携なし) / Update: SQL translated to pure Mojo
**日本語**: `tools/cobol2mojo` が、COBOL_to の Python 出力にある SQL 文(`SQL_HANDLER`・SQLCODE・NULL 指示変数・カーソルの OPEN/FETCH/CLOSE・PREPARE/EXECUTE・`_sqlnum`)を、Python の埋め込みなしの Mojo に訳すようになった。実行時は `c2m_sql.mojo`(C の `libsqlite3` を FFI で直接呼ぶ `SqlRt`)を生成物へ取り込む。**実測**(WSL Ubuntu-24.04、Mojo 1.2): COBOL_to の試験用20本すべてが Python 出力と一致(`verify.sh`)、うち sql1〜sql7 は生成物に `_pyx`(Python 連携)が0。認識できない SQL の形が出たら、従来どおり Python 連携に戻る(失敗させない)。COMMIT/ROLLBACK は開いたトランザクションが無ければ 0 を返す(Python 版と同じ)。**未了**: ファイル SORT/MERGE の一部(merge1・sort1・sort2)と misc(CALL)は、まだ Python 連携を使っている。`COBOL_SQL_DB`(DB ファイル指定)は純 Mojo 版では未対応(常にメモリ DB)。
**English**: `tools/cobol2mojo` now translates the SQL statements found in COBOL_to's Python output (`SQL_HANDLER`, SQLCODE, NULL indicator variables, cursor OPEN/FETCH/CLOSE, PREPARE/EXECUTE, `_sqlnum`) into Mojo with no embedded Python. At run time the generated file includes `c2m_sql.mojo` (`SqlRt`, which calls C `libsqlite3` directly over FFI). **Measured** (WSL Ubuntu-24.04, Mojo 1.2): all 20 COBOL_to sample programs match the Python output (`verify.sh`), and sql1 to sql7 contain zero `_pyx` (Python bridge) calls. If an unrecognised SQL shape appears, it falls back to the Python bridge as before (it does not fail). COMMIT/ROLLBACK return 0 when no transaction is open, as in the Python version. **Open**: file SORT/MERGE parts (merge1, sort1, sort2) and misc (CALL) still use the Python bridge; `COBOL_SQL_DB` (DB file selection) is not supported in the pure-Mojo path (always an in-memory DB).

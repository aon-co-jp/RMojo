# POLICY(方針) / Policy

## 日本語
- **Rust のみ・C++ なし**。Mojo 風構文(`fn`/`var`/`struct`)と Python 構文の一部を実行する。互換性は CPython との差分試験(`tests/run.py`、`differences=0`)で守る。
- **組み込み上限**: ループ回数(`max_steps`)・実時間(`max_ms`)・再帰深さ・要素数(2,000万、`MemoryError`)。信頼できない入力を実行する側(MFastAPI 等)はこれを必ず使う。実メモリ(RSS)上限は未実装。
- 新しい REST は作らない(Connect のみ)。秘密は書かない。本物で確認してから「完了」と書く。
- 姉妹(MFastAPI・RPoem・Reaxt・RPoRea・open-cosmo・pl2pl)と同時に開発し、1つ変えたら同日に文書を更新する。

## English
- **Rust only, no C++**. Runs Mojo-style syntax (`fn`/`var`/`struct`) and a Python subset. Compatibility is guarded by the CPython diff suite (`tests/run.py`, `differences=0`).
- **Embedding limits**: loop steps (`max_steps`), wall time (`max_ms`), recursion depth, element count (20M, `MemoryError`). Hosts that run untrusted input (MFastAPI etc.) must use them. A true RSS cap is not implemented.
- No new REST (Connect only); never write secrets; say "done" only after verifying on real systems.
- Developed together with the sisters (MFastAPI, RPoem, Reaxt, RPoRea, open-cosmo, pl2pl); update docs the same day when one changes.

<!-- directive-20261125-1130 -->
## 2026-11-25 指示(11:30 まで確認不要) / Directive (no confirmation until 11:30)
**日本語**: 2026-11-25 11:30 まで、未了は確認なしで完成まで自動で進める。需要が少ない・必要性が低いと判断される、あるいは1から大規模に作り直す必要があっても、別の方式で済ませず、正面から作り直す。同時に開発する対象: COBOL_to・Java_to・MFastAPI・2MFastAPI(他の言語・フレームワークから MFastAPI 互換へ。Node.js・TypeScript は例外的に選べ、選ばない選択も可能)・RMojo・RPoRea(RPoem と Reaxt の統合)・2RPoRea(他の言語・フレームワークから RPoRea へ)・Reaxt(React+Next.js+Nuxt.js+Nest.js の統合)。1つ変えたら他を確認し、README・CLAUDE・PORTING・POLICY を日英で同じ日に更新する。
**English**: Until 2026-11-25 11:30, carry unfinished work through to completion automatically without asking. Even if demand looks low or a large from-scratch rewrite is needed, do not substitute another approach; do the real work. Developed together: COBOL_to, Java_to, MFastAPI, 2MFastAPI (converts other languages and frameworks to MFastAPI-compatible code; Node.js and TypeScript are an optional exception and choosing neither is allowed), RMojo, RPoRea (RPoem and Reaxt integrated), 2RPoRea (converts other languages and frameworks to RPoRea) and Reaxt (React + Next.js + Nuxt.js + Nest.js integrated). When one changes, check the others and update README, CLAUDE, PORTING and POLICY in Japanese and English the same day.

<!-- directive-20261125-mojo -->
## 2026-11-25 追記: MFastAPI は Mojo から使えること / Addendum: MFastAPI must be usable from Mojo
**日本語**: MFastAPI は **Mojo から使用できる仕様**を保つ(Python 構文・Rust 実装・Mojo 対応、Windows では RMojo の `compat/wsl-bridge` 経由で WSL 上の Mojo を使う)。2MFastAPI の変換結果も Mojo から使えること。現状: `mojo/` は参照実装で、段階B・Cのサーバーは実験的。Mojo からの呼び出しの実機確認は、確認済みの範囲だけを「確認済み」と書く。11:30 まで確認なしで自動で進める。
**English**: MFastAPI keeps the **specification of being usable from Mojo** (Python syntax, Rust core, Mojo support; on Windows the real Mojo in WSL is driven through RMojo's `compat/wsl-bridge`). Output of 2MFastAPI must be usable from Mojo too. Current state: `mojo/` is a reference implementation and its stage B/C servers are experimental. Only what was actually run is called "verified". Continue automatically without asking until 11:30.

## 2026-11-26 cobol2mojo の拡張(12/20本が一致) / cobol2mojo extended (12 of 20 match)
**日本語**: `tools/cobol2mojo/cobol_py2mojo.py` を拡張した。追加: 表(`[0]*N` → `List[Int]`)、段落内の局所変数、`abs`・`10 ** k`、改行なしの DISPLAY、文字列操作(連結・UNSTRING の分割・数え上げ・置換・大文字化・数字判定・INSPECT の検索/切り出し/先頭一致回数)、ファイル入出力(`_FILES` の固定長レコードの OPEN/READ/WRITE/CLOSE と FILE STATUS)、SORT/MERGE(ファイル読込・安定ソート・書き出し、INPUT/OUTPUT PROCEDURE)。CALL は Python と同じく例外で停止する。WSL(Ubuntu-24.04、Mojo 1.2)の `verify.sh` で、**hello・calc・eval・eval2・comp3・features・search1・inspect2・fileio・merge1・sort1・sort2 の12本が Python 出力と完全一致**(Mojo の警告・エラーなし)。**TODO(8本)**: sql1〜sql7(Python 側の `SQL_HANDLER`=SQL 処理系を Mojo に写す必要があり未着手)、misc(Python 自身が CALL "GREET" で NotImplementedError になり正常出力が無いため、一致を確認できない。変換自体はでき、Mojo も同じ例外で止まる)。数値の読み取り `_num` は Python の `int()` のうち符号と数字だけを扱う(`1_0` 等は未対応)。
**English**: Extended `tools/cobol2mojo/cobol_py2mojo.py` with tables (`[0]*N` to `List[Int]`), paragraph locals, `abs` and `10 ** k`, DISPLAY without newline, string operations (concatenation, UNSTRING split, count, replace, upper, numeric test, INSPECT find/slice/leading count), file I/O (fixed-length records from `_FILES`: OPEN/READ/WRITE/CLOSE with FILE STATUS) and SORT/MERGE (read files, stable sort, write, INPUT/OUTPUT PROCEDURE). CALL raises, as in Python. With `verify.sh` in WSL (Ubuntu-24.04, Mojo 1.2), **12 programs match the Python output exactly: hello, calc, eval, eval2, comp3, features, search1, inspect2, fileio, merge1, sort1, sort2** (no Mojo warnings or errors). **TODO (8)**: sql1 to sql7 (the Python `SQL_HANDLER`, an SQL engine, must be mirrored in Mojo; not started) and misc (Python itself raises NotImplementedError at CALL "GREET", so there is no normal output to compare; it converts, and Mojo stops with the same error). `_num` handles only sign and digits of Python's `int()` (no `1_0` etc.).

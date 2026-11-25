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

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

#!/usr/bin/env bash
# WSL (Ubuntu-24.04, /root/mojo-venv) で COBOL_to の Python 出力と変換後 Mojo の出力を比べる。
# usage: bash verify.sh <COBOL_to out dir (has programs/)> <mojo out dir> prog...
source /root/mojo-venv/bin/activate
py=$1; mj=$2; shift 2
here=$(cd "$(dirname "$0")" && pwd)
cd "$py"
cp "$here/c2m_external.py" ./c2m_external.py
for p in "$@"; do
  python3 -c "import programs.$p as m; g={}; exec(open('c2m_external.py', encoding='utf-8').read(), g); g['register'](m.EXTERNAL); print(m.run(), end='')" > /tmp/c2m_py_$p.txt
  mojo "$mj/$p.mojo" > /tmp/c2m_mj_$p.txt 2>/tmp/c2m_err_$p.txt
  if cmp -s /tmp/c2m_py_$p.txt /tmp/c2m_mj_$p.txt && [ ! -s /tmp/c2m_err_$p.txt ]; then echo "MATCH $p"; else echo "DIFF $p"; fi
done

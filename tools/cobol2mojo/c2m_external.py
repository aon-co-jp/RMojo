# CALL の登録先(比較試験用)。verify.sh が Python 側と Mojo 側の両方で register(EXTERNAL) を呼ぶ。
# Registry for CALL targets used by verify.sh on both the Python and the Mojo side.


def _greet(n):
    print("GREET " + str(n), flush=True)
    return 0


def register(external):
    external["GREET"] = _greet

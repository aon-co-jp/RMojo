# CALL の登録先(純 Mojo 版)。c2m_external.py の GREET と同じ動作。
# Pure-Mojo registry for CALL targets; same behaviour as GREET in c2m_external.py.
def c2m_ext_GREET(n: Int) raises -> Int:
    print("GREET " + String(n))
    return 0

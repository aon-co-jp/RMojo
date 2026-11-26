# 純 Mojo の SQL_HANDLER の試作(まだ変換器には組み込んでいない)。Python の sqlite3 を使わず、C の libsqlite3 を FFI で直接呼ぶ。
# Prototype of a pure-Mojo SQL_HANDLER (not wired into the converter yet): calls the C libsqlite3 over FFI, no Python.
# 実行: mojo tools/cobol2mojo/c2m_sql.mojo (WSL、Mojo 1.2)。COBOL_to の sql1 と同じ文の結果(COUNT=3, BOB 20.25, 0件=100相当, ROW 1/2, 構文エラー=-1)になる。
# Run: mojo tools/cobol2mojo/c2m_sql.mojo (WSL, Mojo 1.2).
from std.ffi import OwnedDLHandle
from std.memory import Pointer
from std.os import getenv

comptime SQL_NULL = "\x00NULL"


struct SqlRt(Movable):
    var lib: OwnedDLHandle
    var db: Int
    var rows: List[List[String]]

    def __init__(out self) raises:
        self.lib = OwnedDLHandle("libsqlite3.so.0")
        self.db = 0
        self.rows = List[List[String]]()

    def _open(mut self) raises:
        if self.db != 0:
            return
        var slot = List[Int](length=1, fill=0)
        var p = String(":memory:")
        var envp = getenv("COBOL_SQL_DB")
        if envp != "":
            p = envp
        var rc = self.lib.call["sqlite3_open", Int32](p.unsafe_ptr(), slot.unsafe_ptr())
        self.db = slot[0]
        if rc != 0:
            raise Error("sqlite3_open failed")

    def handler(mut self, sql: String, params: List[String]) raises -> Int:
        self._open()
        self.rows = List[List[String]]()
        var head = String("")
        for part in sql.split():
            head = String(part).upper()
            break
        if head == "COMMIT" or head == "ROLLBACK":
            if Int(self.lib.call["sqlite3_get_autocommit", Int32](self.db)) != 0:
                return 0
        var slot = List[Int](length=1, fill=0)
        var s = sql
        var rc = self.lib.call["sqlite3_prepare_v2", Int32](self.db, s.unsafe_ptr(), Int32(-1), slot.unsafe_ptr(), Int(0))
        var stmt = slot[0]
        if rc != 0:
            return -1
        var i = 0
        for prm in params:
            i += 1
            var pc = prm
            if pc == SQL_NULL:
                _ = self.lib.call["sqlite3_bind_null", Int32](stmt, Int32(i))
            else:
                _ = self.lib.call["sqlite3_bind_text", Int32](stmt, Int32(i), pc.unsafe_ptr(), Int32(-1), Int(-1))
        var ncol = Int(self.lib.call["sqlite3_column_count", Int32](stmt))
        while True:
            var r = Int(self.lib.call["sqlite3_step", Int32](stmt))
            if r == 100:
                var row = List[String]()
                for c in range(ncol):
                    var ty = Int(self.lib.call["sqlite3_column_type", Int32](stmt, Int32(c)))
                    if ty == 5:
                        row.append(String(SQL_NULL))
                    else:
                        var tp = self.lib.call["sqlite3_column_text", Int](stmt, Int32(c))
                        var cp = Pointer[UInt8, MutAnyOrigin](unsafe_from_address=tp)
                        row.append(String(unsafe_from_utf8_ptr=cp))
                self.rows.append(row^)
            elif r == 101:
                break
            else:
                _ = self.lib.call["sqlite3_finalize", Int32](stmt)
                return -1
        _ = self.lib.call["sqlite3_finalize", Int32](stmt)
        return 0


def main() raises:
    var rt = SqlRt()
    print("create", rt.handler("CREATE TABLE T (ID INTEGER, NAME TEXT, AMT REAL)", List[String]()))
    var d1 = List[String]()
    d1.append("1")
    d1.append("ALICE")
    d1.append("10.5")
    _ = rt.handler("INSERT INTO T (ID, NAME, AMT) VALUES (?, ?, ?)", d1)
    var d2 = List[String]()
    d2.append("2")
    d2.append("BOB")
    d2.append("20.25")
    _ = rt.handler("INSERT INTO T (ID, NAME, AMT) VALUES (?, ?, ?)", d2)
    var d3 = List[String]()
    d3.append("3")
    d3.append("CY")
    d3.append("5")
    _ = rt.handler("INSERT INTO T (ID, NAME, AMT) VALUES (?, ?, ?)", d3)
    var c = rt.handler("SELECT COUNT(*) FROM T", List[String]())
    print("count", c, rt.rows[0][0])
    var ps2 = List[String]()
    ps2.append("2")
    var one = rt.handler("SELECT NAME, AMT FROM T WHERE ID = ?", ps2)
    print(one, rt.rows[0][0], rt.rows[0][1])
    var ps3 = List[String]()
    ps3.append("9")
    var none = rt.handler("SELECT NAME FROM T WHERE ID = ?", ps3)
    print("norows", none, len(rt.rows))
    var ps4 = List[String]()
    ps4.append("6")
    _ = rt.handler("SELECT ID, NAME FROM T WHERE AMT > ? ORDER BY ID", ps4)
    for i in range(len(rt.rows)):
        print("ROW", rt.rows[i][0], rt.rows[i][1])
    print("bad", rt.handler("SELEC nope", List[String]()))

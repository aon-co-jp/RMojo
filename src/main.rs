fn main() {
    let args: Vec<String> = std::env::args().collect();
    rmojo::update::on_startup(&args); // 起動時の自動バージョンアップ(24時間に1回、切り離して確認。RMOJO_NO_UPDATE=1 で停止)
    match args.get(1).map(String::as_str) {
        Some(rmojo::update::WORKER_ARG) => {
            if let Err(e) = rmojo::update::run_worker() {
                if let Some(h) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
                    let _ = std::fs::write(std::path::Path::new(&h).join(".rmojo").join("last_error"), e);
                }
            }
        }
        Some("update") if rmojo::update::disabled() => println!("rmojo: 自動更新は無効です(環境変数 RMOJO_NO_UPDATE または CI が設定されています)"),
        Some("update") => match rmojo::update::run_worker() {
            Ok(Some(v)) => println!("rmojo: {v} に更新しました"),
            Ok(None) => println!("rmojo: 最新です({})", rmojo::update::CURRENT),
            Err(e) => {
                eprintln!("rmojo: 更新できません: {e}");
                std::process::exit(1);
            }
        },
        Some("--version") | Some("-V") => println!("rmojo {} (Rust, no C++)", env!("CARGO_PKG_VERSION")),
        Some("cpu") => print!("{}", open_cpu::inventory().summary()),
        Some("run") if args.len() > 2 => {
            let src = match std::fs::read_to_string(&args[2]) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("rmojo: cannot read {}: {e}", args[2]);
                    std::process::exit(2);
                }
            };
            let src = if args[2].ends_with(".mojo") || args[2].ends_with(".🔥") {
                rmojo::set_copy_mode(true);
                match rmojo::mojo_syntax::to_python(&src) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("{e}");
                        std::process::exit(2);
                    }
                }
            } else {
                src
            };
            // run on a big stack so deep (but bounded) recursion is safe
            let h = std::thread::Builder::new().stack_size(1 << 28).spawn(move || rmojo::run_source(&src));
            match h.map(|h| h.join()) {
                Ok(Ok(Ok(()))) => {}
                Ok(Ok(Err(e))) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
                _ => {
                    eprintln!("rmojo: internal error");
                    std::process::exit(70);
                }
            }
        }
        _ => {
            eprintln!("usage: rmojo run <file.mojo|file.py> | rmojo --version");
            std::process::exit(2);
        }
    }
}

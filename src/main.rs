use clap::Parser;
use winrarkey::{cli::Cli, run};

fn main() {
    // 入口层只负责解析参数与转交执行
    let cli = Cli::parse();
    if let Err(err) = run(cli) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

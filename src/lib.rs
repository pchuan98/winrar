pub mod cli;
pub mod crypto;

use std::fs;
use std::path::Path;

use cli::Cli;
use crypto::{format_license, generate_register_info};

pub fn run(cli: Cli) -> Result<(), String> {
    // 输出文件名固定为 rarreg.key
    let output = Path::new("rarreg.key");
    let mut rng = rand::rng();

    // 先生成内部注册数据 再格式化成最终授权文本
    let info = generate_register_info(&cli.user, &cli.license_name, &mut rng);
    let content = format_license(&info);

    fs::write(output, &content).map_err(|e| e.to_string())?;

    println!("written: {}", output.display());
    println!("uid: {}", info.uid);
    println!("data0: {}", info.items[0]);
    println!("checksum: {:010}", info.checksum);
    println!();
    println!("使用方式");
    println!("1. 将当前目录生成的 rarreg.key 放到 WinRAR 程序目录或 %APPDATA%\\WinRAR");
    println!("2. 也可以把 rarreg.key 拖到 WinRAR 窗口中尝试导入");
    println!("3. 常用命令示例");
    println!("   winrarkey.exe --user \"{}\"", cli.user);
    println!(
        "   winrarkey.exe --user \"{}\" --license-name \"{}\"",
        cli.user, cli.license_name
    );
    Ok(())
}

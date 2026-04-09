pub mod archive;
pub mod cli;
pub mod crypto;

use std::fs;

use cli::{Cli, Mode};
use crypto::{format_license, generate_register_info};

pub fn run(cli: Cli) -> Result<(), String> {
    // 统一在这里决定默认输出路径
    let output = cli.output();
    let mut rng = rand::rng();

    // 先生成内部注册数据 再格式化成最终授权文本
    let info = generate_register_info(&cli.user, &cli.license_type, &mut rng);
    let content = format_license(&info);

    match cli.mode {
        // 直接落地明文授权文件
        Mode::Rarreg => fs::write(&output, &content).map_err(|e| e.to_string())?,
        // 打包成可导入的 rarkey.rar
        Mode::Rarkey => archive::create_rarkey_rar(&output, &content)?,
    }

    println!("written: {}", output.display());
    println!("uid: {}", info.uid);
    println!("data0: {}", info.items[0]);
    println!("checksum: {:010}", info.checksum);
    Ok(())
}

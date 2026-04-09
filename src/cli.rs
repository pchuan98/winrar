use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum Mode {
    /// 直接生成 rarreg.key 文本文件
    Rarreg,
    /// 生成包含 rarreg.key 的 rarkey.rar
    Rarkey,
}

// 用 clap derive 维护命令行参数定义
#[derive(Debug, Parser)]
#[command(
    name = "winrarkey",
    version,
    about = "Generate WinRAR license files from username and mode."
)]
pub struct Cli {
    /// 用户名
    #[arg(short, long)]
    pub user: String,

    /// 输出模式：rarreg 或 rarkey
    #[arg(short, long, value_enum)]
    pub mode: Mode,

    /// License type
    #[arg(short = 't', long = "type", default_value = "Single PC usage license")]
    pub license_type: String,

    /// 输出路径
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

impl Cli {
    // 如果用户未显式传入输出路径 就按模式给默认文件名
    pub fn output(&self) -> PathBuf {
        self.output.clone().unwrap_or_else(|| match self.mode {
            Mode::Rarreg => PathBuf::from("rarreg.key"),
            Mode::Rarkey => PathBuf::from("rarkey.rar"),
        })
    }
}

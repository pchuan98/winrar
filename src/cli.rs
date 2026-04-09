use clap::Parser;

// 用 clap derive 维护命令行参数定义
#[derive(Debug, Parser)]
#[command(
    name = "winrarkey",
    version,
    about = "Generate WinRAR rarreg.key from username."
)]
pub struct Cli {
    /// 用户名
    #[arg(short, long)]
    pub user: String,

    /// 授权名称
    #[arg(
        short = 'l',
        long = "license-name",
        default_value = "Single PC usage license"
    )]
    pub license_name: String,
}

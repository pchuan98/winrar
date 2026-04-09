use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_workdir() -> PathBuf {
    // 为临时 rar 打包过程创建独立目录 避免污染当前目录
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = env::temp_dir().join(format!("winrarkey-{ts}"));
    let _ = fs::create_dir_all(&dir);
    dir
}

fn rar_tools() -> Vec<PathBuf> {
    // 按常见优先级搜索 RAR 可执行文件
    let mut tools = vec![PathBuf::from("rar"), PathBuf::from("WinRAR")];

    for env_name in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(base) = env::var_os(env_name) {
            tools.push(PathBuf::from(base.clone()).join("WinRAR").join("Rar.exe"));
            tools.push(PathBuf::from(base).join("WinRAR").join("WinRAR.exe"));
        }
    }

    tools
}

pub fn create_rarkey_rar(output: &Path, content: &str) -> Result<(), String> {
    // rarkey.rar 本质是包含 rarreg.key 的 RAR 包
    let workdir = temp_workdir();
    let inner = workdir.join("rarreg.key");
    fs::write(&inner, content).map_err(|e| e.to_string())?;

    for tool in rar_tools() {
        let status = Command::new(&tool)
            .args(["a", "-ep", "-idq"])
            .arg(output)
            .arg(&inner)
            .current_dir(&workdir)
            .status();

        if let Ok(status) = status
            && status.success()
        {
            let _ = fs::remove_dir_all(&workdir);
            return Ok(());
        }
    }

    // 没找到 RAR 工具时 回退写出 key 方便继续手工处理
    let fallback = output.with_extension("key");
    let _ = fs::write(&fallback, content);
    let _ = fs::remove_dir_all(&workdir);

    Err(format!(
        "没有找到可用的 RAR/WinRAR 命令，已回退写出 `{}`。",
        fallback.display()
    ))
}

# RAR.exe 注册功能分析报告

## 1. 分析目标

对 `RAR.exe` 的“注册功能”进行静态分析，确认其注册实现方式、关键路径、涉及的文件/注册表项，以及能否判断其大致校验模型。

本报告仅基于本地静态分析结果整理，不包含动态调试、补丁制作、注册码生成或绕过实现。

## 2. 分析环境

- 工作目录：`C:\Users\haeer\Desktop\fefe`
- 目标文件：`RAR.exe`
- 时间：2026-04-08
- 系统环境：Windows / PowerShell

## 3. 使用的工具

### 3.1 系统与命令行工具

- `PowerShell`
  - 用于查看目录、确认环境、执行辅助脚本
- `rg`
  - 用于直接在二进制文件中搜索字符串

### 3.2 Python 辅助分析

- `python`
  - 编写了多个一次性脚本，用于：
  - 解析 PE 头
  - 读取导入表
  - 定位 Unicode/ASCII 字符串
  - 建立字符串到代码位置的交叉引用
  - 反汇编指定函数附近的机器码

### 3.3 临时安装的 Python 库

- `pefile`
  - 用于解析 PE 结构、节表、RVA/文件偏移转换、导入表
- `capstone`
  - 用于对 `.text` 段进行反汇编，定位字符串引用和关键调用

## 4. 分析过程

### 4.1 确认样本与环境

先检查工作目录内容，确认目标文件存在，并确认本地可用工具。

发现：

- 目标主程序为 `RAR.exe`
- 同目录包含大量 WinRAR 相关组件，如 `Rar.exe`、`UnRAR.exe`、`RarExt.dll`、`Default.SFX`

这说明该题目样本整体上与 WinRAR 体系高度接近。

### 4.2 提取注册相关字符串

使用 `rg -a` 在 `RAR.exe` 中检索以下关键词：

- `register`
- `registration`
- `license`
- `serial`
- `key`
- `trial`
- `evaluation`
- `buy`

得到的关键信息包括：

- `rarreg.key`
- `rarreg.txt`
- `rarkey`
- `IDS_REGFAILED`
- `IDS_REGCORRECT`
- `IDS_REGISTEREDTO`
- `IDS_REGKEYWARNING`
- `IDS_NEWREGKEYTITLE`
- `IDS_EVALCOPY`
- `IDS_EVALDAYSLEFT`

这一步已经明确表明：

- 程序存在“已注册/注册失败/试用版”这几类状态
- 注册介质明显与 `rarreg.key` 有关
- 不像传统“用户名 + 序列号”输入框模式，更像“授权文件导入”模式

### 4.3 解析 PE 结构与导入表

用 `pefile` 解析 `RAR.exe`，确认：

- 文件为 `PE32+`，即 64 位程序
- 存在 `.text`、`.rdata`、`.data`、`.rsrc` 等标准节
- `ADVAPI32.dll` 采用 delay import

导入表中与本题最相关的 API 包括：

- `RegOpenKeyExW`
- `RegQueryValueExW`
- `RegSetValueExW`
- `RegDeleteValueW`
- `RegGetValueW`
- `CreateFileW`
- `ReadFile`
- `CopyFileW`

另外还看到：

- `CryptAcquireContextW`
- `CryptGenRandom`
- `CryptReleaseContext`

但没有发现：

- `CryptVerifySignature`
- `WinVerifyTrust`
- `BCryptVerifySignature`

这说明：

- 程序确实会访问注册表和文件系统
- 如果存在授权校验，更可能是程序内部自定义逻辑，而不是直接调用系统验签 API

### 4.4 定位关键字符串引用

对以下字符串进行了精确定位：

- `rarkey`
- `rarreg.key`
- `rarreg.txt`

定位结果：

- `rarkey`：`RVA 0x1f8580`
- `rarreg.key`：`RVA 0x1f8590`
- `rarreg.txt`：`RVA 0x1f85a8`

随后用 `capstone` 扫描 `.text` 段，查找引用这些字符串的代码位置。

得到的关键交叉引用包括：

- `rarkey`
  - `0x1400945fe`
  - `0x14019f82c`
  - `0x14019f953`
  - `0x1401a1457`
- `rarreg.key`
  - `0x140094667`
  - `0x14019911e`
  - `0x1401991f2`
  - `0x1401a38dc`
  - `0x1401a3aec`
  - `0x1401ade9b`
- `rarreg.txt`
  - `0x1400946a1`

### 4.5 确认“注册文件识别”逻辑

`0x140094584` 附近的代码最关键。

从反汇编结果可见：

- 先比较 `rarkey`
- 若不匹配，再比较 `rarreg.key`
- 若仍不匹配，再比较 `rarreg.txt`

对应关键点：

- `0x1400945fe`：引用 `rarkey`
- `0x140094667`：引用 `rarreg.key`
- `0x1400946a1`：引用 `rarreg.txt`

结论：

程序中存在一个专门的“注册相关文件名识别函数”，它明确把 `rarreg.key` / `rarreg.txt` 视为注册授权文件。

### 4.6 确认“注册表中转值”逻辑

`0x14019f800` 附近代码显示：

- 构造了 `rarkey`
- 调用内部注册表读取函数
- 传入的根键值先是 `0xffffffff80000002`，即 `HKEY_LOCAL_MACHINE`
- 另一条路径会走 `HKEY_CURRENT_USER`

结合进一步追踪的包装函数 `0x14012a3fc`、`0x14012a2f4`，底层最终落到：

- `RegOpenKeyExW`
- `RegQueryValueExW`

结论：

程序会从注册表查询一个名为 `rarkey` 的值，且同时支持从 `HKLM` 与 `HKCU` 路径读取。

这更像是：

- 外部导入流程写入了一个临时路径/状态值
- 程序启动后读取它，再完成后续导入或转换

### 4.7 确认“规范化为 rarreg.key”逻辑

`0x140199100` 附近能看到标准文件名构造和复制操作：

- `0x14019911e`：构造 `rarreg.key`
- `0x1401991f2`：再次构造 `rarreg.key`
- `0x1401991af`：`CopyFileW`
- `0x14019922f`：`CopyFileW`

从代码行为上看：

- 程序会把某个来源文件复制到标准目标名 `rarreg.key`
- 失败和成功路径都围绕这个标准文件名组织

结论：

`rarreg.key` 是程序认可的正式授权文件名，导入流程最终会把外部输入规范化为这个文件名。

### 4.8 确认“导入后清理注册表值”逻辑

`0x140129c78` 附近代码显示：

- 打开某个注册表键
- 对目标值名执行 `RegDeleteValueW`

它调用链中和 `rarkey`、注册表包装函数是连通的。

结论：

`rarkey` 很可能不是永久授权正文，而是导入时使用的中转注册表值。程序处理完成后会清掉它。

### 4.9 发现授权块头标记

在程序中发现一个固定标记：

- `Rar$RK`

该字符串位于：

- `RVA 0x20ad80`

并在：

- `0x1401a3a15`

处被代码引用。

这说明：

- 程序内部存在一种与注册/授权相关的结构化数据块
- `rarreg.key` 并不是简单的纯文本用户名或序列号，而更像带头标识的授权记录/授权块

## 5. 关键函数与作用推断

### 5.1 `0x140094584`

作用推断：

- 识别输入对象是否与注册相关
- 明确接受：
  - `rarkey`
  - `rarreg.key`
  - `rarreg.txt`

### 5.2 `0x14019f800`

作用推断：

- 读取注册表中的 `rarkey`
- 支持 `HKLM` / `HKCU`

### 5.3 `0x140199100`

作用推断：

- 构造标准目标名 `rarreg.key`
- 复制授权文件到标准位置
- 伴随界面与状态更新逻辑

### 5.4 `0x140129c78`

作用推断：

- 删除注册表中的授权中转值
- 与 `RegDeleteValueW` 直接关联

### 5.5 `0x1401a3a15`

作用推断：

- 使用 `Rar$RK` 固定头
- 涉及授权数据块的构造、处理或输出

## 6. 最终结论

### 6.1 注册机制类型

`RAR.exe` 的注册功能实现，结论上属于：

- **授权文件注册**

而不是：

- 普通序列号输入注册
- 在线激活注册

### 6.2 注册介质

可确定的注册相关介质包括：

- 正式授权文件：`rarreg.key`
- 兼容文件名：`rarreg.txt`
- 注册表中转值：`rarkey`

### 6.3 注册大致流程

推断流程如下：

1. 程序识别或接收注册文件
2. 若存在 `rarkey` 注册表值，则从注册表取得路径或中转信息
3. 将输入授权文件规范化为 `rarreg.key`
4. 对授权文件内容做内部格式/合法性检查
5. 成功则进入“Registered to”状态
6. 失败则进入“Reg failed / Evaluation copy / Days left”分支
7. 导入完成后清理 `rarkey`

### 6.4 关于校验方式

当前未发现系统验签 API 的直接使用，因此可作如下判断：

- 授权校验并非依赖 Windows 标准验签接口
- 更可能是程序内部自定义校验逻辑
- `Rar$RK` 说明授权文件内容具有固定结构

## 7. 当前已得到的结果

已确认：

- 该程序存在完整的注册/试用逻辑
- 注册核心文件名是 `rarreg.key`
- `rarreg.txt` 也被接受
- 程序通过 `rarkey` 注册表值参与导入流程
- 程序会把授权文件复制为标准名 `rarreg.key`
- 导入后会删除 `rarkey`
- 存在授权结构头 `Rar$RK`

## 7.1 新增交叉验证与修正

基于后续补充验证，可进一步修正为：

- 最终落地的 `rarreg.key` 很可能是**文本格式授权文件**
- 其外层文本头为 `RAR registration data`
- 文件中包含：
  - 用户名行
  - 授权类型行
  - `UID=...`
  - 多行十六进制授权数据

这说明前文发现的 `Rar$RK` 更可能属于：

- 导入链路中的内部标记
- 中转数据块名
- 或 `rarkey.rar` / 注册表导入流程使用的内部结构头

而**不是**最终 `rarreg.key` 文件本身的外层明文头。

## 8. 仍未完全展开的部分

当前未完成的部分主要是：

- `rarreg.key` 的精确文件结构定义
- 授权内容字段的详细语义
- 内部“合法/非法授权文件”判定函数的完整伪代码
- “Registered to”文本最终从哪个字段提取

这些内容如果继续分析，需要沿着：

- `rarreg.key` 实际文件读取路径
- `ReadFile` 之后的数据处理函数
- `Rar$RK` 使用点
- `IDS_REGCORRECT` / `IDS_REGISTEREDTO` 的调用路径

继续向下追踪。

## 9. 总结

本次分析已经足以确认：

- `RAR.exe` 的注册功能是典型的“授权文件注册”设计
- 核心授权文件为 `rarreg.key`
- 程序通过注册表值 `rarkey` 参与导入和过渡
- 校验逻辑内置在程序内部，而不是直接依赖系统验签 API

如果后续继续做更细致的逆向，重点应转向：

- `rarreg.key` 的内容解析
- `Rar$RK` 授权块格式
- 最终注册成功分支的字段提取逻辑

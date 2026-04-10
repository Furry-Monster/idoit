# idoit

`idoit` 是一个 AI 驱动的命令行简化工具。  
你用自然语言描述目标，它会生成可执行的 Shell 命令，并在执行前让你确认。

## 功能概览

- 自然语言转命令：`idoit <你的需求>`
- 失败修复：`idoit --fix`
- 命令解释：`idoit --explain '<command>'`
- 结果细化：`idoit --refine '<补充约束>'`
- 宏保存：`idoit --macro <name> <text...>`，在任意需求里用 `@name` 内联展开（定义写在 `macros.toml`）
- 重新执行上次生成命令：`idoit --last`
- 交互式 setup：`idoit setup`
- Shell 初始化脚本：`idoit init bash|zsh|fish`

## 安装与运行

前置要求：

- Rust（建议 stable）
- 可用的 AI 提供商密钥（OpenAI / Anthropic / Gemini）或本地 Ollama

在项目目录执行：

```bash
cargo build
cargo run -- --help
```

如果你希望安装到本机命令：

```bash
cargo install --path .
idoit --help
```

## 快速开始

1) 运行 setup：

```bash
idoit setup
```

2) 直接用自然语言生成命令：

```bash
idoit find files containing "TODO" in src
```

3) 修复上一条失败命令：

```bash
idoit --fix
```

4) 解释一条复杂命令：

```bash
idoit --explain 'find . -name "*.log" -mtime +7 -delete'
```

## 常用参数

- `-f`, `--fix`：修复最近失败命令
- `-l`, `--learn`：附带教学解释
- `-a`, `--anyway`：即使缺少工具也继续（仍会确认）
- `-d`, `--dry-run`：只显示命令，不执行
- `-y`, `--yes`：跳过执行确认
- `-p`, `--provider`：临时指定提供商（`openai|anthropic|gemini|ollama`）
- `--config`：查看当前配置
- `-e`, `--explain`：解释命令
- `-r`, `--refine`：细化上次建议
- `--last`：重新执行上次生成命令
- `--macro <name>`：保存宏（`@name` 展开）

## 配置说明

默认配置路径：

`~/.config/idoit/config.toml`

宏定义路径：

`~/.config/idoit/macros.toml`

常见环境变量：

- `OPENAI_API_KEY`
- `ANTHROPIC_API_KEY`
- `GEMINI_API_KEY`

若使用 Ollama，请确保本地服务可用（默认 `http://localhost:11434`）。

## Shell 集成

可以生成对应 Shell 的初始化脚本：

```bash
idoit init bash
idoit init zsh
idoit init fish
```

脚本中还包含便捷别名（如 `ido`, `ifix`, `ilearn`, `iexplain`）。

## 说明

`idoit` 会在执行前进行确认（除非你显式使用 `--yes` 或开启自动确认）。  
执行 AI 生成命令前，请先检查命令内容，尤其是在生产环境中。

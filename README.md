# Hero-RS - 英雄联盟连招脚本

这是一个用Rust编写的英雄联盟连招辅助工具，实现了以下功能：

- 监听键盘和鼠标输入并在控制台显示
- 自动检测预设的连招序列并执行
- 支持全局脚本和英雄特定脚本
- 支持多种触发方式（单键、键序列、组合键）
- 支持按键屏蔽功能
- 灵活的操作延迟设置
- 可通过配置文件自定义连招

## 功能特点

- 全局脚本和英雄特定脚本分离
- 每个按键操作支持独立的延迟配置
- 多种连招触发方式：
  - 单键触发（例如按A键触发QWE连招）
  - 键序列触发（例如按下E后150毫秒内按R触发亚索EQR连招）
  - 修饰键组合触发（例如按住Alt同时按Q键）
- 支持屏蔽原始按键输入
- 实时监听键盘和鼠标事件
- 硬件级模拟鼠标键盘操作
- 命令行交互界面
- 通过Ctrl+C优雅退出

## 环境要求

- Rust 环境（1.56.0及以上）

## 安装和运行

1. 克隆仓库
   ```
   git clone https://github.com/yourusername/hero-rs.git
   cd hero-rs
   ```

2. 编译并运行
   ```
   cargo run
   ```

## 使用方法

1. 运行程序后，它会在后台监听键盘和鼠标事件
2. 使用命令行界面切换英雄：`champion Yasuo`
3. 当您按下的按键序列匹配预设连招时，程序会自动执行相应操作
4. 您可以通过编辑配置文件来自定义连招
5. 按Ctrl+C退出程序

### 可用命令

- `champion <英雄名称>` - 切换当前使用的英雄
- `help` - 显示帮助信息
- `exit` 或 `quit` - 退出程序

## 配置文件

配置文件位于：
- Windows: `%APPDATA%\hero-rs\config.toml`
- macOS: `~/Library/Application Support/hero-rs/config.toml`
- Linux: `~/.config/hero-rs/config.toml`

配置文件格式示例：

```toml
[general]
history_size = 20
history_timeout_ms = 2000
default_delay_ms = 50

# 全局连招配置（所有英雄通用）
[[global_combos]]
name = "QWE连招"
sequence = [
    { key = { Keyboard = "KeyQ" }, delay_after_ms = 50 },
    { key = { Keyboard = "KeyW" }, delay_before_ms = 20, delay_after_ms = 50 },
    { key = { Keyboard = "KeyE" }, delay_before_ms = 20 }
]
trigger = { SingleKey = "KeyA" }
block_original_input = false
active = true

# 特定英雄配置
[champion_specific.Yasuo]
name = "Yasuo"
[[champion_specific.Yasuo.combos]]
name = "亚索EQR双风"
sequence = [
    { key = { Keyboard = "KeyE" }, delay_after_ms = 50 },
    { key = { Keyboard = "KeyQ" }, delay_after_ms = 50 },
    { key = { Keyboard = "KeyR" } }
]
trigger = { KeySequence = { keys = ["KeyE", "KeyR"], timeout_ms = 150 } }
block_original_input = true
active = true

[champion_specific.Tryndamere]
name = "Tryndamere"
[[champion_specific.Tryndamere.combos]]
name = "蛮王连招E+W+Q"
sequence = [
    { key = { Keyboard = "KeyE" }, delay_after_ms = 100 },
    { key = { Keyboard = "KeyW" }, delay_after_ms = 100 },
    { key = { Keyboard = "KeyQ" } }
]
trigger = { SingleKey = "KeyZ" }
block_original_input = false
active = true
```

### 配置项说明

#### 通用设置

- `general`: 全局配置
  - `history_size`: 按键历史记录的最大长度
  - `history_timeout_ms`: 按键历史记录的超时时间（毫秒）
  - `default_delay_ms`: 默认延迟时间（毫秒）

#### 连招配置

- `global_combos`: 全局连招配置（数组）
- `champion_specific`: 英雄特定配置（字典）

#### 连招属性

- `name`: 连招名称
- `sequence`: 按键序列数组
  - `key`: 按键类型（Keyboard或Mouse）
  - `delay_before_ms`: 按键前的延迟（可选）
  - `delay_after_ms`: 按键后的延迟（可选）
- `trigger`: 触发条件
  - `SingleKey`: 单键触发
  - `KeySequence`: 按键序列触发，包含键数组和超时时间
  - `KeyModifier`: 修饰键组合，包含修饰键和主键
  - `Manual`: 手动触发（预留）
- `block_original_input`: 是否屏蔽原始输入
- `active`: 是否启用该连招

## 注意事项

1. 按键屏蔽功能仅在grab模式下工作（需要管理员/root权限）
2. 不同系统上的按键名称可能有差异
3. 部分游戏可能有反作弊机制，请谨慎使用

## 开发计划

- [x] 监听键盘和鼠标事件
- [x] 实现连招序列识别
- [x] 硬件级模拟键盘鼠标操作
- [x] 配置文件支持
- [x] 全局和英雄特定脚本
- [x] 多种触发方式支持
- [x] 灵活的延迟配置
- [x] 按键屏蔽功能
- [ ] 图形用户界面
- [ ] 连招录制功能
- [ ] 低延迟优化
- [ ] 更多游戏特定功能支持

## 法律声明

本工具仅用于学习和研究目的，请勿在实际游戏中使用以避免违反游戏规则。作者不对使用本工具可能导致的任何后果负责。 # hero-rs

//! Simplified Chinese dictionary for [`crate::i18n::tr`].
//!
//! Keys are the exact English source strings; a missing key falls back to
//! English. Keep entries sorted alphabetically so merges stay mechanical.

pub fn translate(text: &str) -> Option<&'static str> {
    Some(match text {
        // ── App menus ──
        "About Warp" => "关于 Warp",
        "Bring All to Front" => "全部置前",
        "Check for Updates..." => "检查更新...",
        "Close" => "关闭",
        "Close Window" => "关闭窗口",
        "Compact Mode" => "紧凑模式",
        "Copy" => "复制",
        "Copy on Select within the Terminal" => "在终端中选择即复制",
        "Create anonymous user" => "创建匿名用户",
        "Cut" => "剪切",
        "Debug" => "调试",
        "Edit" => "编辑",
        "Export Default Settings as CSV to home dir" => "将默认设置导出为 CSV 到主目录",
        "File" => "文件",
        "GitHub Issues..." => "GitHub Issues...",
        "Help" => "帮助",
        "Hide" => "隐藏",
        "Hide Others" => "隐藏其他",
        "Hide Warpified SSH Blocks" => "隐藏 Warpified SSH 块",
        "Join our Slack community..." => "加入 Slack 社区...",
        "Launch Configurations" => "启动配置",
        "Log out" => "登出",
        "Manually Toggle Network Status" => "手动切换网络状态",
        "Minimize" => "最小化",
        "New Agent Tab" => "新建智能体标签页",
        "New Terminal Tab" => "新建终端标签页",
        "New Window" => "新建窗口",
        "Open Recent" => "最近打开",
        "Paste" => "粘贴",
        "Preferences" => "偏好设置",
        "Privacy Policy..." => "隐私政策...",
        "Redo" => "重做",
        "Reopen closed session" => "重新打开已关闭的会话",
        "Save New..." => "另存为...",
        "Select All" => "全选",
        "Send Feedback..." => "发送反馈...",
        "Set Warp as Default Terminal" => "将 Warp 设为默认终端",
        "Show All" => "显示全部",
        "Show In-band Command Blocks" => "显示带内命令块",
        "Show Initialization Block" => "显示初始化块",
        "Show Warpified SSH Blocks" => "显示 Warpified SSH 块",
        "Synchronize Inputs" => "同步输入",
        "Toggle Focus Reporting" => "切换焦点上报",
        "Toggle Mouse Reporting" => "切换鼠标上报",
        "Toggle Scroll Reporting" => "切换滚动上报",
        "Undo" => "撤销",
        "Use Warp's Prompt" => "使用 Warp 提示符",
        "View" => "查看",
        "Warp Documentation..." => "Warp 文档...",

        // ── Shared menu / settings labels ──
        "Blocks" => "命令块",
        "Tab" => "标签页",
        "Warp" => "Warp",
        "Warp Drive" => "Warp Drive",
        "Window" => "窗口",
        "Zoom" => "缩放",

        // ── Settings sidebar and page titles ──
        "About" => "关于",
        "Account" => "账户",
        "Appearance" => "外观",
        "Billing and usage" => "账单与用量",
        "Cloud platform" => "云平台",
        "Code" => "代码",
        "Agents" => "智能体",
        "Editor and Code Review" => "编辑器与代码审查",
        "Environments" => "环境",
        "Features" => "功能",
        "Indexing and projects" => "索引与项目",
        "Keyboard shortcuts" => "键盘快捷键",
        "Knowledge" => "知识库",
        "MCP servers" => "MCP 服务器",
        "Privacy" => "隐私",
        "Profiles" => "配置档案",
        "Referrals" => "推荐计划",
        "Scripting" => "脚本",
        "Search" => "搜索",
        "Settings" => "设置",
        "Shared blocks" => "共享命令块",
        "Teams" => "团队",
        "Third party CLI agents" => "第三方 CLI 智能体",
        "Warp Agent" => "Warp 智能体",
        "API keys" => "API 密钥",

        // ── Settings appearance page: categories ──
        "Cursor" => "光标",
        "Full-screen Apps" => "全屏应用",
        "Icon" => "图标",
        "Input" => "输入",
        "Language" => "语言",
        "Panes" => "窗格",
        "Tabs" => "标签页",
        "Text" => "文本",
        "Themes" => "主题",
        "Tools panel" => "工具面板",

        // ── Settings appearance page: widget labels ──
        "Adjusts the default zoom level across all windows" => "调整所有窗口的默认缩放级别",
        "Agent conversations" => "智能体会话",
        "Agent font" => "智能体字体",
        "Bar" => "竖线",
        "Blinking cursor" => "光标闪烁",
        "Block" => "块",
        "Columns" => "列",
        "Compact mode" => "紧凑模式",
        "Cursor type" => "光标样式",
        "Customize your app icon" => "自定义应用图标",
        "Dim inactive panes" => "弱化非活动窗格",
        "Enforce minimum contrast" => "强制最低对比度",
        "Fallback font" => "回退字体",
        "Focus follows mouse" => "焦点跟随鼠标",
        "Global search" => "全局搜索",
        "Header toolbar layout" => "顶栏工具栏布局",
        "Hide search bar in vertical tab layout" => "在竖排标签页布局中隐藏搜索栏",
        "Input position" => "输入位置",
        "Input type" => "输入类型",
        "Open new windows with custom size" => "以自定义尺寸打开新窗口",
        "Preserve active tab color for new tabs" => "为新标签页保留活动标签页颜色",
        "Project explorer" => "项目浏览器",
        "Rows" => "行",
        "Show block dividers" => "显示块分隔线",
        "Show Jump to Bottom of Block button" => "显示“跳到块底部”按钮",
        "Show ligatures in terminal" => "在终端中显示连字",
        "Show tab indicators" => "显示标签页指示器",
        "Show the tab bar" => "显示标签栏",
        "Show vertical tabs panel in restored windows" => "在恢复的窗口中显示竖排标签页面板",
        "Show Warp in Dock" => "在 Dock 中显示 Warp",
        "Sync with OS" => "跟随系统",
        "Tab close button position" => "标签页关闭按钮位置",
        "Terminal font" => "终端字体",
        "Tools panel visibility is consistent across tabs" => "工具面板在各标签页间保持可见性一致",
        "Underline" => "下划线",
        "Use custom padding in alt-screen" => "在全屏应用中使用自定义内边距",
        "Use latest user prompt as conversation title in tab names" => {
            "在标签页名称中使用最新用户输入作为会话标题"
        }
        "Use thin strokes" => "使用细笔画",
        "Use vertical tab layout" => "使用竖排标签页布局",
        "Window backdrop" => "窗口背景材质",

        // ── Settings appearance page: prompts and hints ──
        "Create your own custom theme" => "创建你自己的自定义主题",
        "Backdrop is not visible at opacity 100%" => "不透明度为 100% 时看不到背景材质",
        "Changing the app icon requires the app to be bundled." => {
            "更改应用图标需要应用以打包方式运行。"
        }

        // ── Common dropdown option labels ──
        "Always" => "总是",
        "Bold" => "粗体",
        "Current theme" => "当前主题",
        "Dark" => "深色",
        "Left" => "左侧",
        "Light" => "浅色",
        "Never" => "从不",
        "Normal" => "常规",
        "On high-DPI displays" => "在高 DPI 显示器上",
        "On low-DPI displays" => "在低 DPI 显示器上",
        "Only for named colors" => "仅限命名颜色",
        "Only on hover" => "仅悬停时",
        "Right" => "右侧",
        "When windowed" => "窗口化时",
        "Shell (PS1)" => "Shell (PS1)",

        // ── Settings pages: Features / general toggles ──
        "1 teammate" => "1 名成员",
        "Agent thinking display" => "智能体思考过程显示",
        "Asynchronous find" => "异步查找",
        "Autocomplete quotes, parentheses, and brackets" => "自动补全引号和括号",
        "Auto-handoff before sleep" => "睡眠前自动交接",
        "Automatically open links in desktop app whenever possible." => {
            "尽可能在桌面应用中打开链接。"
        }
        "Auto open code review panel" => "自动打开代码审查面板",
        "Auto reload" => "自动重载",
        "Auto save" => "自动保存",
        "Auto show/hide Rich Input based on agent status" => "根据智能体状态自动显示/隐藏富输入框",
        "Base model" => "基础模型",
        "Call MCP servers" => "调用 MCP 服务器",
        "Clipboard access (OSC 52)" => "剪贴板访问 (OSC 52)",
        "Cloud handoff" => "云端交接",
        "Cloud Handoff" => "云端交接",
        "Code editor line numbers:" => "代码编辑器行号：",
        "Computer use in Cloud Agents" => "云智能体使用计算机",
        "Confirm before closing shared session" => "关闭共享会话前确认",
        "Context window (tokens)" => "上下文窗口（token）",
        "Copy on select" => "选中即复制",
        "Ctrl+Tab behavior:" => "Ctrl+Tab 行为：",
        "Custom Routers" => "自定义路由",
        "Default long-running command submission mode" => "长时命令的默认提交模式",
        "Default prompt submission mode" => "输入内容的默认提交模式",
        "Default shell for new sessions" => "新会话的默认 Shell",
        "Double-click smart selection" => "双击智能选择",
        "Edit code and commands with Vim keybindings" => "使用 Vim 键位编辑代码和命令",
        "Enable '@' context menu in terminal mode" => "在终端模式下启用 '@' 上下文菜单",
        "Enable agent attribution" => "启用智能体归属",
        "Enabled by your organization." => "由你的组织启用。",
        "Enable Focus Reporting" => "启用焦点上报",
        "Enable Mouse Reporting" => "启用鼠标上报",
        "Enable Scroll Reporting" => "启用滚动上报",
        "Enable slash commands in terminal mode" => "在终端模式下启用斜杠命令",
        "Enables the use of Wayland" => "启用 Wayland 支持",
        "Error underlining for commands" => "命令错误下划线",
        "Expand aliases as you type" => "输入时展开别名",
        "Experimental" => "实验性",
        "Format on save (requires an active language server)" => {
            "保存时格式化（需要运行中的语言服务器）"
        }
        "General" => "通用",
        "Global file search" => "全局文件搜索",
        "Global hotkey:" => "全局热键：",
        "Honor linux selection clipboard" => "遵循 Linux 选择剪贴板",
        "Install SSH extension" => "安装 SSH 扩展",
        "Join this workspace and start collaborating on workflows, notebooks, and more." => {
            "加入此工作区，开始协作使用工作流、笔记本等功能。"
        }
        "Key for Activating Voice Input" => "激活语音输入的按键",
        "Keys" => "按键",
        "Ligatures may reduce performance" => "连字可能降低性能",
        "Lines scrolled by mouse wheel interval" => "鼠标滚轮单次滚动行数",
        "Maximum rows in a block" => "单个块的最大行数",
        "Middle-click to paste" => "中键粘贴",
        "Models" => "模型",
        "Native shell completions" => "原生 Shell 补全",
        "Network log console" => "网络日志控制台",
        "New tab placement" => "新标签页位置",
        "Notifications" => "通知",
        "Open completions menu as you type" => "输入时打开补全菜单",
        "Open links in desktop app" => "在桌面应用中打开链接",
        "Orchestration message display" => "编排消息显示",
        "Other" => "其他",
        "Outline codebase symbols for '@' context menu" => "为 '@' 上下文菜单提取代码库符号",
        "Permissions" => "权限",
        "Preferred graphics backend" => "首选图形后端",
        "Preferred layout when opening existing agent conversations" => {
            "打开已有智能体会话时的首选布局"
        }
        "Prefer rendering new windows with integrated GPU (low power)" => {
            "优先用集成显卡渲染新窗口（低功耗）"
        }
        "Preserve input focus on block selection" => "选择块时保持输入焦点",
        "Quit when all windows are closed" => "所有窗口关闭后退出",
        "Receive desktop notifications from Warp" => "接收 Warp 桌面通知",
        "Requires the Warp plugin for your coding agent" => "需要为你的编码智能体安装 Warp 插件",
        "Restore windows, tabs, and panes on startup" => "启动时恢复窗口、标签页和窗格",
        "Reuse existing SSH ControlMaster" => "复用已有的 SSH ControlMaster",
        "Right-click:" => "右键：",
        "Send crash reports" => "发送崩溃报告",
        "Session" => "会话",
        "Sets the monthly overage spending limit beyond the plan amount" => {
            "设置超出套餐金额的月度消费上限"
        }
        "Settings sync" => "设置同步",
        "Show autosuggestion ignore button" => "显示自动建议忽略按钮",
        "Show autosuggestion keybinding hint" => "显示自动建议快捷键提示",
        "Show changelog toast after updates" => "更新后显示更新日志提示",
        "Show code review button" => "显示代码审查按钮",
        "Show diff stats on code review button" => "在代码审查按钮上显示差异统计",
        "Show Global Workflows in Command Search (ctrl-r)" => "在命令搜索中显示全局工作流 (ctrl-r)",
        "Show help block in new sessions" => "在新会话中显示帮助块",
        "Show hidden files in project explorer" => "在项目浏览器中显示隐藏文件",
        "Show in-app agent notifications" => "显示应用内智能体通知",
        "Show sticky command header" => "显示固定命令头",
        "Show terminal input message line" => "显示终端输入消息行",
        "Show tooltip on click on links" => "点击链接时显示提示",
        "Show warning before quitting/logging out" => "退出/登出前显示警告",
        "Speech Language" => "语音语言",
        "Store AI conversations in the cloud" => "在云端存储 AI 会话",
        "Suggest corrected commands" => "建议纠正后的命令",
        "Supports floating point values between 1 and 20." => "支持 1 到 20 之间的浮点值。",
        "Syntax highlighting for commands" => "命令语法高亮",
        "System" => "系统",
        "Terminal" => "终端",
        "Terminal Input" => "终端输入",
        "Text Editing" => "文本编辑",
        "Usage" => "用量",
        "Usage display unit" => "用量显示单位",
        "Use Audible Bell" => "使用响铃",
        "Use IAP for staging requests" => "使用 IAP 发起预发布请求",
        "Use & to trigger handoff" => "使用 & 触发交接",
        "Use Wayland for window management" => "使用 Wayland 管理窗口",
        "Voice" => "语音",
        "Warp completions" => "Warp 补全",
        "Warp Control CLI command" => "Warp Control CLI 命令",
        "Warpify SSH Sessions" => "Warpify SSH 会话",
        "Whether the Linux primary clipboard should be supported." => {
            "是否支持 Linux 主选择剪贴板。"
        }
        "Window Opacity:" => "窗口不透明度：",
        "Workflows" => "工作流",
        "Working directory for new sessions" => "新会话的工作目录",

        // ── Common buttons ──
        "+ Add custom model" => "+ 添加自定义模型",
        "+ Add router" => "+ 添加路由",
        "Add Profile" => "添加配置档案",
        "Add repo" => "添加仓库",
        "Cancel" => "取消",
        "Cancelling…" => "取消中…",
        "Change default model" => "更改默认模型",
        "Continue with ChatGPT" => "使用 ChatGPT 继续",
        "Continue with Grok" => "使用 Grok 继续",
        "Create" => "创建",
        "Create environment" => "创建环境",
        "Delete" => "删除",
        "Delete environment" => "删除环境",
        "Disconnect" => "断开连接",
        "Index new folder" => "索引新文件夹",
        "Load more" => "加载更多",
        "Not now" => "暂不",
        "Open file" => "打开文件",
        "Refresh" => "刷新",
        "Remove" => "移除",
        "Remove endpoint" => "移除端点",
        "Save" => "保存",
        "Sign up" => "注册",

        // ── Features page: inline hints ──
        "Autohides on loss of keyboard focus" => "失去键盘焦点时自动隐藏",
        "Change keybinding" => "更改快捷键",
        "Height %" => "高度 %",
        "Keybinding" => "快捷键",
        "Not supported on Wayland. " => "Wayland 不支持。",
        "Press new keyboard shortcut" => "按下新的快捷键",
        "Tab key behavior" => "Tab 键行为",
        "Toast notifications stay visible for" => "Toast 通知显示时长为",
        "When a command takes longer than" => "当命令耗时超过",
        "Width %" => "宽度 %",
        "Window positions won't be restored on Wayland. " => "Wayland 下不会恢复窗口位置。",
        "seconds" => "秒",
        "seconds to complete" => "秒才能完成",

        // ── Warp Agent page ──
        "Agent Attribution" => "智能体归属",
        "Configure keyboard shortcuts" => "配置键盘快捷键",
        "Connect your ChatGPT plan to use OpenAI models in the Warp Agent through your OpenAI account." => {
            "连接你的 ChatGPT 套餐，通过你的 OpenAI 账号在 Warp 智能体中使用 OpenAI 模型。"
        }
        "Connect your SuperGrok plan to use Grok models in the Warp Agent through your xAI account." => {
            "连接你的 SuperGrok 套餐，通过你的 xAI 账号在 Warp 智能体中使用 Grok 模型。"
        }
        "Custom endpoints" => "自定义端点",
        "OpenAI ChatGPT plan" => "OpenAI ChatGPT 套餐",
        "Premium or SuperGrok plan" => "Premium 或 SuperGrok 套餐",
        "To use AI features, please create an account." => "使用 AI 功能请先创建账户。",
        "Use your" => "使用你的",
        "Your organization disallows AI when the active pane contains content from a remote session" => {
            "当活动窗格包含远程会话内容时，你的组织禁止使用 AI"
        }

        // ── Keybindings page ──
        "Add your own custom keybindings to existing actions below." => {
            "在下方为现有操作添加自定义快捷键。"
        }
        "Command" => "命令",
        "Keyboard shortcuts are not synced to the cloud" => "键盘快捷键不会同步到云端",
        "This shortcut conflicts with other keybinds" => "此快捷键与其他键位冲突",
        "to reference these keybindings in a side pane at anytime." => {
            "可随时在侧边栏中查看这些快捷键。"
        }
        "Use" => "使用",

        _ => return None,
    })
}

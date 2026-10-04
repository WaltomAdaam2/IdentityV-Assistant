import { memo, useCallback, useDeferredValue, useEffect, useMemo, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { bridge } from "./bridge";
import { appendLogBatch, appendNormalLogBatch, filterLogs, isWindowDiagnostic, progressLog, type LogEntry, type LogLevel } from "./logs";
import type {
  CaptureFrame,
  EngineEvent,
  EngineSnapshot,
  Settings,
  TaskItem,
  TemplateTestResult,
  WindowInfo,
} from "./types";

type Page =
  | "dashboard"
  | "tasks"
  | "runtime"
  | "debug"
  | "logs"
  | "settings"
  | "about";
type Language = Settings["app"]["language"];
interface TaskProgress {
  taskId: string;
  stage: string;
  role: "hunter" | "survivor" | "bad_child" | null;
  completedCycles: number;
  result?: string;
}
interface TaskControlEvent {
  revision: number;
  type: "task_started" | "task_paused" | "task_resumed" | "task_finished";
  taskId?: string;
}

const translations = {
  "zh-CN": {
    nav: {
      dashboard: "概览",
      tasks: "任务",
      runtime: "服务详情",
      debug: "捕捉诊断",
      logs: "运行日志",
      settings: "设置",
      about: "关于",
    },
    localConsole: "本地运行",
    localOnly: "数据仅保存在此设备",
    engine: "自动化服务",
    windowControls: {
      minimize: "最小化",
      maximize: "最大化或还原",
      close: "关闭",
    },
    closeNotice: "关闭提示",
    ready: "应用已就绪，等待自动化服务连接。",
    states: {
      stopped: "已停止",
      starting: "启动中",
      idle: "就绪",
      running: "运行中",
      paused: "已暂停",
      stopping: "停止中",
      error: "异常",
    },
    taskStates: {
      idle: "空闲",
      queued: "等待中",
      running: "执行中",
      paused: "已暂停",
    },
    levels: { debug: "调试", info: "信息", warning: "警告", error: "错误" },
    taskNames: {
      login_lobby: [
        "登录进大厅",
        "自动识别当前登录画面，选择平台并关闭弹窗，直到进入归宿",
      ],
      blackjack: ["黑杰克", "自动匹配、游玩并返回归宿"],
      hunter_match: ["自动监管者匹配", "自动进入普通匹配、游玩并返回归宿"],
      hunter_rank: ["自动监管者排位", "自动进入传统排位、游玩并返回归宿"],
      hide_seek: ["捉迷藏", "自动匹配坏孩子、完成对局并返回归宿"],
    },
    dashboard: {
      eyebrow: "自动化服务状态",
      noTask: "自动化服务尚未执行任务。",
      taskState: "任务状态：",
      start: "启动服务",
      stop: "停止",
      restart: "重启",
      process: "服务进程",
      notStarted: "未启动",
      heartbeat: "连接延迟",
      enabledTasks: "启用任务",
      version: "服务版本",
      overview: "任务概况",
      overviewHint: "当前配置中的自动任务",
      manage: "管理任务",
      quickStart: "快速开始",
      quickHint: "连接游戏并检查环境",
      steps: [
        "在桌面启动第五人格。",
        "在“设置”确认窗口标题。",
        "启动服务后，从“任务”开始执行。",
      ],
    },
    tasks: {
      title: "自动任务",
      hint: "按列表顺序依次执行已启用任务。",
      startEnabled: "启动已启用任务",
      pause: "暂停",
      resume: "恢复",
      execute: "执行",
      startTask: "启动任务",
      pauseTask: "暂停任务",
      resumeTask: "恢复任务",
      stopTask: "停止任务",
      edit: "编辑",
      closeEdit: "收起",
      infiniteLoop: "无限循环",
      cameraScale: "视角转动幅度（%）",
      editHint: "更改会自动保存，并立即应用到正在运行的服务。",
    },
    progress: {
      title: "任务进度",
      cycles: "完成循环",
      role: "角色",
      hunter: "监管者",
      survivor: "求生者",
      bad_child: "坏孩子",
      unknown: "待确认",
      results: { completed: "已完成", user_stopped: "已停止", timeout: "等待超时", window_lost: "窗口已关闭", window_recovery_timeout: "窗口恢复超时", engine_error: "执行失败" },
      stages: {
        starting: "正在启动", waiting_home: "等待归宿", home: "已到归宿",
        waiting_chaos: "等待混沌纷争", chaos: "混沌纷争",
        waiting_blackjack: "等待黑杰克", blackjack: "黑杰克",
        waiting_start_case: "等待开始案件还原", start_case: "准备匹配",
        waiting_matching: "等待匹配", matching: "匹配中",
        waiting_ready_case: "等待确认匹配", ready_case: "确认匹配",
        waiting_ready_start: "等待准备开始", ready_start: "准备开始",
        waiting_card_confirm: "等待卡牌", card_confirm: "收下卡牌",
        waiting_role: "等待对局角色", playing: "对局中",
        continue: "进入结算画面", waiting_return_lobby: "等待返回大厅",
        return_lobby: "返回大厅", cycle_complete: "本局完成",
        diary: "日记推理", rank_entry: "排位模式", traditional_rank: "传统排位",
        hunter_choice: "选择监管者", normal_matching: "普通匹配中", rank_matching: "排位匹配中",
        queue_warning: "阵营队列人数过多", result_first: "进入结算画面",
        result_second: "进入第二个结算画面", surrendering: "等待投降", surrender: "投降菜单",
        loading: "加载中", running: "运行中",
        waiting_hide_seek: "等待捉迷藏", hide_seek: "捉迷藏",
        waiting_bad_child: "等待坏孩子", bad_child: "选择坏孩子",
        waiting_start_game: "等待开始游戏", start_game: "准备匹配",
        hide_seek_matching: "捉迷藏匹配中", waiting_f: "等待进入对局",
        disguise: "寻找伪装", waiting_r_blocked: "确认伪装状态",
        moving_to_fix: "寻找固定位置", waiting_v_fixed: "等待固定完成",
        fixed: "成功固定", eliminated: "已淘汰", hide_seek_results: "进入结算画面",
        f_initial: "寻找伪装", r_blocked: "已伪装", v_ready: "寻找固定位置",
        v_fixed: "成功固定", living_no_v: "固定消失", death_f: "已淘汰",
      },
    },
    runtime: {
      snapshot: "服务状态数据",
      refreshHint: "每 5 秒自动刷新",
      refresh: "立即刷新",
      latestEvent: "最新任务事件",
      command: "调度命令",
      commandHint: "直接向本地自动化服务发送 JSON 指令",
      send: "发送命令",
      invalidJson: "请输入有效的 JSON 对象。",
      sendCommand: "发送运行时命令",
    },
    debug: {
      gameWindow: "游戏窗口",
      gameWindowHint: "检查当前桌面上的目标窗口。",
      detect: "检测窗口",
      found: "已找到",
      notFound: "未找到",
      status: "状态",
      title: "标题",
      process: "进程 / PID",
      className: "窗口类名",
      clientRect: "客户区",
      dpi: "DPI 缩放",
      windowState: "窗口状态",
      responding: "响应状态",
      responsive: "正常响应",
      unresponsive: "未响应",
      foreground: "前台",
      background: "后台",
      minimized: "已最小化",
      restored: "未最小化",
      matchReasons: "匹配依据",
      captureBackend: "捕获后端",
      configuredBackend: "配置后端",
      captureNote: "捕捉备注",
      requestId: "请求编号",
      resolution: "画面尺寸",
      filePath: "文件位置",
      errorTitle: "操作未完成",
      matchReasonLabels: { process: "进程名", title: "窗口标题", class: "窗口类名" },
      preview: "画面预览",
      previewHint: "获取最近一帧，文件保存在自动化服务的 debug 文件夹。",
      capture: "捕获画面",
      captureAlt: "最近捕获的游戏画面",
      noFrame: "尚未捕获画面",
      capturedAt: "捕获时间",
      duration: "耗时",
      fileSize: "文件大小",
      captureRegion: "捕获区域",
      windowHandle: "窗口句柄",
      template: "模板识别",
      templateHint:
        "填写 assets/templates 下的相对文件路径，结果会通过服务消息显示。",
      templatePath: "模板路径",
      templatePlaceholder: "例如：lobby/start.png",
      testTemplate: "测试模板",
      result: "结果",
      matched: "已匹配",
      unmatched: "未匹配",
      confidence: "置信度",
      input: "输入测试",
      inputHint: "输入测试默认关闭。首次操作会先启用开发者输入模式；当前：",
      enabled: "已启用",
      disabled: "未启用",
      testClick: "测试点击",
      testEsc: "测试 Esc",
      lastAction: "最近计划操作",
      detectWindow: "检测窗口",
      captureFrame: "捕获画面",
      testTemplateRequest: "测试模板",
      clickTest: "点击测试",
      keyTest: "按键测试",
    },
    logs: {
      all: "全部级别",
      filter: "筛选日志",
      clear: "清空当前会话日志",
      time: "时间",
      level: "级别",
      source: "来源",
      message: "消息",
      empty: "没有匹配的日志。",
      previous: "上一页",
      next: "下一页",
      jump: "跳转",
      page: "页",
      total: "条记录",
    },
    settings: {
      general: "常规",
      generalHint: "切换语言后，界面文本会立即更新。",
      theme: "界面主题",
      system: "跟随系统",
      light: "浅色",
      dark: "深色",
      language: "界面语言",
      launch: "启动应用时启动自动化服务",
      preventSystemSleep: "禁止电脑自动休眠",
      preventDisplaySleep: "禁止自动熄屏和闲置锁屏",
      gameWindow: "游戏窗口",
      gameWindowHint: "通过进程名、标题关键词和窗口类名定位游戏窗口。",
      windowResolution: "游戏分辨率",
      windowPosition: "窗口位置",
      windowLayoutHint: "引擎开启时应用。无限制保留当前位置；关闭引擎恢复原窗口。",
      windowPositions: { center: "居中", unrestricted: "无限制", top_left: "左上", top_right: "右上", bottom_left: "左下", bottom_right: "右下" },
      titleKeywords: "窗口标题关键词",
      processNames: "进程名",
      classNames: "窗口类名",
      loginAutomation: "登录自动化",
      loginAutomationHint: "随机偏移始终启用；每次点击前会重复确认当前画面。",
      loginPlatform: "登录平台",
      android: "Android",
      ios: "iOS",
      loginTimeout: "最长等待时间（秒）",
      screenshotDelay: "点击后等待截图（毫秒）",
      clickDelay: "识别后等待点击（毫秒）",
      positionJitter: "点击位置随机范围（像素）",
      intervalJitter: "点击间隔随机范围（毫秒）",
      durationJitter: "按住时长随机范围（毫秒）",
      developerMode: "开发者模式",
      developerModeHint: "显示服务详情与手动命令",
      developerSection: "开发者设置",
      developerSectionHint: "以下选项用于诊断和高级调整。",
      engineCapture: "服务与捕捉",
      engineCaptureHint: "服务与捕捉诊断设置。",
      heartbeat: "连接检测间隔（秒）",
      captureBackend: "捕获后端",
      auto: "自动选择",
      windowCapture: "窗口捕捉（推荐）",
      idleFps: "空闲帧率",
      memoryProtection: "内存占用保护",
      memoryProtectionHint: "系统内存超过 90% 时清理未使用缓存",
      recognition: "识别与诊断",
      threshold: "模板匹配阈值",
      verbose: "记录详细调试日志",
      screenshots: "保存调试截图",
      autosave: "更改会自动保存",
    },
    about: {
      name: "第五人格助手",
      description:
        "面向第五人格玩家的本地自动化助手，用于连接游戏窗口并执行任务。",
      version: "版本",
      author: "作者",
      license: "许可证",
      stack: "技术栈",
    },
    actionSent: (label: string) => `${label}请求已发送。`,
    actionNotice: (label: string) => `${label}请求已发送`,
    actionFailed: (label: string, error: string) => `${label}失败：${error}`,
    waitingEvent: (label: string) => `${label}请求已发送，等待服务返回结果。`,
    readFrameFailed: (error: string) => `无法读取调试画面：${error}`,
    snapshotFailed: (error: string) => `无法读取运行时快照：${error}`,
    taskEvent: (state: string) => `任务状态：${state}`,
    transitionEvent: (from: string, to: string) => `状态迁移：${from} → ${to}`,
    finishedEvent: (reason: string) => `任务结束：${reason}`,
    receivedEvent: "收到服务消息",
  },
  "en-US": {
    nav: {
      dashboard: "Overview",
      tasks: "Tasks",
      runtime: "Service details",
      debug: "Capture diagnostics",
      logs: "Service logs",
      settings: "Settings",
      about: "About",
    },
    localConsole: "Local service",
    localOnly: "Data stays on this device",
    engine: "Automation service",
    windowControls: {
      minimize: "Minimize",
      maximize: "Maximize or restore",
      close: "Close",
    },
    closeNotice: "Dismiss notification",
    ready: "The app is ready and waiting for the automation service.",
    states: {
      stopped: "Stopped",
      starting: "Starting",
      idle: "Ready",
      running: "Running",
      paused: "Paused",
      stopping: "Stopping",
      error: "Error",
    },
    taskStates: {
      idle: "Idle",
      queued: "Queued",
      running: "Running",
      paused: "Paused",
    },
    levels: {
      debug: "Debug",
      info: "Info",
      warning: "Warning",
      error: "Error",
    },
    taskNames: {
      login_lobby: [
        "Enter the home lobby",
        "Detects the current login screen, selects a platform, and dismisses popups until home is reached",
      ],
      blackjack: ["Blackjack", "Match, play and return to the home lobby"],
      hunter_match: ["Automatic Hunter Match", "Join a normal hunter match, play and return home"],
      hunter_rank: ["Automatic Hunter Rank", "Join traditional hunter rank, play and return home"],
      hide_seek: ["Hide and Seek", "Match as a Bad Child, finish the round, and return home"],
    },
    dashboard: {
      eyebrow: "AUTOMATION SERVICE",
      noTask: "The automation service is not running a task.",
      taskState: "Task status: ",
      start: "Start service",
      stop: "Stop",
      restart: "Restart",
      process: "Service process",
      notStarted: "Not started",
      heartbeat: "Connection delay",
      enabledTasks: "Enabled tasks",
      version: "Service version",
      overview: "Task overview",
      overviewHint: "Automated tasks in the current configuration",
      manage: "Manage tasks",
      quickStart: "Quick start",
      quickHint: "Connect the game and check the environment",
      steps: [
        "Launch Identity V on the desktop.",
        "Confirm the window title in Settings.",
        "Start the service, then run a task from Tasks.",
      ],
    },
    tasks: {
      title: "Automated tasks",
      hint: "The automation service runs enabled tasks one at a time.",
      startEnabled: "Start enabled tasks",
      pause: "Pause",
      resume: "Resume",
      execute: "Run",
      startTask: "Start task",
      pauseTask: "Pause task",
      resumeTask: "Resume task",
      stopTask: "Stop task",
      edit: "Edit",
      closeEdit: "Collapse",
      infiniteLoop: "Loop indefinitely",
      cameraScale: "Camera turn scale (%)",
      editHint: "Changes are saved automatically and applied to the running service.",
    },
    progress: {
      title: "Task progress",
      cycles: "Completed cycles",
      role: "Role",
      hunter: "Hunter",
      survivor: "Survivor",
      bad_child: "Bad Child",
      unknown: "Unconfirmed",
      results: { completed: "Completed", user_stopped: "Stopped", timeout: "Timed out", window_lost: "Game window lost", window_recovery_timeout: "Window recovery timed out", engine_error: "Failed" },
      stages: {
        starting: "Starting", waiting_home: "Waiting for home", home: "At home",
        waiting_chaos: "Waiting for Chaos", chaos: "Chaos mode",
        waiting_blackjack: "Waiting for Blackjack", blackjack: "Blackjack",
        waiting_start_case: "Waiting to start case", start_case: "Preparing match",
        waiting_matching: "Waiting for match", matching: "Matching",
        waiting_ready_case: "Waiting for match confirmation", ready_case: "Confirming match",
        waiting_ready_start: "Waiting for ready", ready_start: "Ready to start",
        waiting_card_confirm: "Waiting for card", card_confirm: "Taking card",
        waiting_role: "Waiting for role", playing: "Playing",
        continue: "Match ended", waiting_return_lobby: "Waiting to return",
        return_lobby: "Returning to lobby", cycle_complete: "Cycle complete",
        diary: "Diary deduction", rank_entry: "Rank mode", traditional_rank: "Traditional rank",
        hunter_choice: "Choose hunter", normal_matching: "Normal matching", rank_matching: "Rank matching",
        queue_warning: "Hunter queue is full", result_first: "First result page",
        result_second: "Second result page", surrendering: "Waiting to surrender", surrender: "Surrender menu",
        loading: "Loading", running: "Running",
        waiting_hide_seek: "Waiting for Hide and Seek", hide_seek: "Hide and Seek",
        waiting_bad_child: "Waiting for Bad Child", bad_child: "Choosing Bad Child",
        waiting_start_game: "Waiting to start game", start_game: "Starting game",
        hide_seek_matching: "Hide and Seek matching", waiting_f: "Waiting for the round",
        disguise: "Finding a disguise", waiting_r_blocked: "Confirming disguise state",
        moving_to_fix: "Moving before fixing", waiting_v_fixed: "Waiting for fixed state",
        fixed: "Fixed in place", eliminated: "Eliminated", hide_seek_results: "Round results",
        f_initial: "Finding a disguise", r_blocked: "Disguised", v_ready: "Moving before fixing",
        v_fixed: "Fixed in place", living_no_v: "Fixing lost, using skills", death_f: "Eliminated",
      },
    },
    runtime: {
      snapshot: "Service status data",
      refreshHint: "Refreshes every 5 seconds",
      refresh: "Refresh now",
      latestEvent: "Latest task event",
      command: "Scheduler command",
      commandHint: "Send a JSON instruction directly to the local automation service",
      send: "Send command",
      invalidJson: "Enter a valid JSON object.",
      sendCommand: "Send runtime command",
    },
    debug: {
      gameWindow: "Game window",
      gameWindowHint: "Inspect the target window on the current desktop.",
      detect: "Detect window",
      found: "Found",
      notFound: "Not found",
      status: "Status",
      title: "Title",
      process: "Process / PID",
      className: "Window class",
      clientRect: "Client area",
      dpi: "DPI scale",
      windowState: "Window state",
      responding: "Response status",
      responsive: "Responsive",
      unresponsive: "Not responding",
      foreground: "Foreground",
      background: "Background",
      minimized: "Minimized",
      restored: "Not minimized",
      matchReasons: "Match reasons",
      captureBackend: "Capture backend",
      configuredBackend: "Configured backend",
      captureNote: "Capture note",
      requestId: "Request ID",
      resolution: "Frame size",
      filePath: "File location",
      errorTitle: "Operation did not complete",
      matchReasonLabels: { process: "Process name", title: "Window title", class: "Window class" },
      preview: "Frame preview",
      previewHint:
        "Get the latest frame. Files are saved to the automation service's debug folder.",
      capture: "Capture frame",
      captureAlt: "Most recently captured game frame",
      noFrame: "No frame captured",
      capturedAt: "Captured at",
      duration: "Duration",
      fileSize: "File size",
      captureRegion: "Capture region",
      windowHandle: "Window handle",
      template: "Template matching",
      templateHint:
        "Enter a relative path under assets/templates. The result arrives through a service message.",
      templatePath: "Template path",
      templatePlaceholder: "For example: lobby/start.png",
      testTemplate: "Test template",
      result: "Result",
      matched: "Matched",
      unmatched: "Not matched",
      confidence: "Confidence",
      input: "Input test",
      inputHint:
        "Input tests start disabled. The first action enables developer input mode; current state: ",
      enabled: "Enabled",
      disabled: "Disabled",
      testClick: "Test click",
      testEsc: "Test Esc",
      lastAction: "Latest planned action",
      detectWindow: "Detect window",
      captureFrame: "Capture frame",
      testTemplateRequest: "Test template",
      clickTest: "Click test",
      keyTest: "Key test",
    },
    logs: {
      all: "All levels",
      filter: "Filter logs",
      clear: "Clear session logs",
      time: "Time",
      level: "Level",
      source: "Source",
      message: "Message",
      empty: "No matching logs.",
      previous: "Previous",
      next: "Next",
      jump: "Go",
      page: "Page",
      total: "entries",
    },
    settings: {
      general: "General",
      generalHint: "Changing the language updates the interface immediately.",
      theme: "Interface theme",
      system: "Use system setting",
      light: "Light",
      dark: "Dark",
      language: "Interface language",
      launch: "Start the automation service when the app opens",
      preventSystemSleep: "Prevent computer sleep",
      preventDisplaySleep: "Prevent display sleep and idle lock",
      gameWindow: "Game window",
      gameWindowHint:
        "Find the game window by process name, title keywords, and window class.",
      windowResolution: "Game resolution",
      windowPosition: "Window position",
      windowLayoutHint: "Applied while the engine runs. Unrestricted keeps the current position; shutdown restores the original window.",
      windowPositions: { center: "Center", unrestricted: "Unrestricted", top_left: "Top left", top_right: "Top right", bottom_left: "Bottom left", bottom_right: "Bottom right" },
      titleKeywords: "Window title keywords",
      processNames: "Process names",
      classNames: "Window classes",
      loginAutomation: "Login automation",
      loginAutomationHint:
        "Random variation is always enabled. The current screen is confirmed again before every click.",
      loginPlatform: "Login platform",
      android: "Android",
      ios: "iOS",
      loginTimeout: "Maximum wait time (seconds)",
      screenshotDelay: "Wait before screenshot after click (ms)",
      clickDelay: "Wait before click after detection (ms)",
      positionJitter: "Click position variation (px)",
      intervalJitter: "Click interval variation (ms)",
      durationJitter: "Click hold variation (ms)",
      developerMode: "Developer mode",
      developerModeHint: "Show service details and manual commands",
      developerSection: "Developer settings",
      developerSectionHint: "These options are intended for diagnostics and advanced tuning.",
      engineCapture: "Service and capture",
      engineCaptureHint: "Service and capture diagnostic settings.",
      heartbeat: "Connection check interval (seconds)",
      captureBackend: "Capture backend",
      auto: "Automatic",
      windowCapture: "Window capture (recommended)",
      idleFps: "Idle frame rate",
      memoryProtection: "Memory protection",
      memoryProtectionHint: "Release unused caches when system memory usage exceeds 90%",
      recognition: "Recognition and diagnostics",
      threshold: "Template match threshold",
      verbose: "Record verbose debug logs",
      screenshots: "Save debug screenshots",
      autosave: "Changes are saved automatically",
    },
    about: {
      name: "Identity V Assistant",
      description:
        "A local automation assistant for Identity V players that connects to the game window and runs tasks.",
      version: "Version",
      author: "Author",
      license: "License",
      stack: "Stack",
    },
    actionSent: (label: string) => `${label} request sent.`,
    actionNotice: (label: string) => `${label} request sent`,
    actionFailed: (label: string, error: string) => `${label} failed: ${error}`,
    waitingEvent: (label: string) =>
      `${label} request sent; waiting for the service response.`,
    readFrameFailed: (error: string) =>
      `Could not read the debug frame: ${error}`,
    snapshotFailed: (error: string) =>
      `Could not read the runtime snapshot: ${error}`,
    taskEvent: (state: string) => `Task state: ${state}`,
    transitionEvent: (from: string, to: string) =>
      `State transition: ${from} → ${to}`,
    finishedEvent: (reason: string) => `Task finished: ${reason}`,
    receivedEvent: "Service message received",
  },
} as const;

const nav: Array<{ id: Page; icon: string }> = [
  { id: "dashboard", icon: "⌂" },
  { id: "tasks", icon: "✓" },
  { id: "runtime", icon: "⌘" },
  { id: "debug", icon: "⌕" },
  { id: "logs", icon: "≡" },
  { id: "settings", icon: "⚙" },
  { id: "about", icon: "i" },
];
const initialTasks: TaskItem[] = [
  {
    id: "login_lobby",
    name: "login_lobby",
    description: "",
    enabled: true,
    status: "idle",
  },
  {
    id: "blackjack",
    name: "blackjack",
    description: "",
    enabled: false,
    status: "idle",
  },
  {
    id: "hunter_match",
    name: "hunter_match",
    description: "",
    enabled: false,
    status: "idle",
  },
  {
    id: "hunter_rank",
    name: "hunter_rank",
    description: "",
    enabled: false,
    status: "idle",
  },
  {
    id: "hide_seek",
    name: "hide_seek",
    description: "",
    enabled: false,
    status: "idle",
  },
];
const defaults: Settings = {
  tasks: { enabled: ["login_lobby"] },
  app: {
    language: "zh-CN",
    theme: "light",
    launch_engine_on_start: false,
    developer_mode: false,
    prevent_system_sleep: false,
    prevent_display_sleep: false,
  },
  engine: {
    memory_protection: false,
    heartbeat_interval_seconds: 2,
  },
  game_window: {
    resolution: "1600x900",
    position: "top_right",
    process_names: ["dwrg.exe", "IdentityV.exe"],
    title_contains: ["第五人格", "Identity V"],
    class_names: [],
  },
  capture: { backend: "wgc", idle_fps: 3, active_fps: 10 },
  vision: { template_threshold: 0.85, state_confirmation_count: 2 },
  automation: {
    login_platform: "android",
    login_timeout_seconds: 300,
    screenshot_after_click_ms: 500,
    click_after_detection_ms: 500,
    click_position_jitter_px: 14,
    click_interval_jitter_ms: 120,
    click_duration_jitter_ms: 15,
    blackjack_infinite_loop: false,
    hunter_match_infinite_loop: false,
    hunter_rank_infinite_loop: false,
    hide_seek_infinite_loop: false,
    camera_turn_scale_percent: 100,
  },
  debug: { save_debug_screenshots: false, verbose_logs: false },
};
function now(language: Language) {
  return new Date().toLocaleTimeString(language, { hour12: false });
}
function asMessage(value: unknown) {
  return value instanceof Error ? value.message : String(value);
}
function engineState(snapshot: EngineSnapshot) {
  return snapshot.status ?? "stopped";
}
function stateLabel(state: string, language: Language) {
  return (
    translations[language].states[
      state as keyof (typeof translations)["zh-CN"]["states"]
    ] ?? state
  );
}
function statusLabel(status: TaskItem["status"], language: Language) {
  return translations[language].taskStates[status];
}
function taskCopy(task: TaskItem, language: Language) {
  return (
    translations[language].taskNames[
      task.id as keyof (typeof translations)["zh-CN"]["taskNames"]
    ] ?? [task.name, task.description]
  );
}

function WindowTitleBar({ language }: { language: Language }) {
  const labels = translations[language].windowControls;
  const appWindow = getCurrentWindow();
  const runWindowAction = (action: () => Promise<unknown>) => {
    void action().catch(() => undefined);
  };
  return (
    <header className="window-titlebar">
      <div className="window-title" data-tauri-drag-region>
        <span className="window-title-mark" data-tauri-drag-region>
          V
        </span>
        <span data-tauri-drag-region>Identity V Assistant</span>
      </div>
      <div className="window-drag-space" data-tauri-drag-region />
      <div className="window-controls">
        <button
          className="window-control"
          type="button"
          aria-label={labels.minimize}
          title={labels.minimize}
          onClick={() => runWindowAction(() => appWindow.minimize())}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="M3 8.5h10" />
          </svg>
        </button>
        <button
          className="window-control"
          type="button"
          aria-label={labels.maximize}
          title={labels.maximize}
          onClick={() => runWindowAction(() => appWindow.toggleMaximize())}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <rect x="3.5" y="3.5" width="9" height="9" rx="1" />
          </svg>
        </button>
        <button
          className="window-control window-close"
          type="button"
          aria-label={labels.close}
          title={labels.close}
          onClick={() => runWindowAction(() => appWindow.close())}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="m4 4 8 8m0-8-8 8" />
          </svg>
        </button>
      </div>
    </header>
  );
}

export default function App() {
  const [page, setPage] = useState<Page>("dashboard");
  const [pagePhase, setPagePhase] = useState<"enter" | "leave">("enter");
  const pageTransitionTimer = useRef<number | null>(null);
  const workspaceRef = useRef<HTMLElement | null>(null);
  const [snapshot, setSnapshot] = useState<EngineSnapshot>({
    state: "stopped",
    queue_size: 0,
  });
  const engineStatusRef = useRef("stopped");
  engineStatusRef.current = engineState(snapshot);
  const [tasks, setTasks] = useState(initialTasks);
  const enabledTaskIds = tasks.filter((task) => task.enabled).map((task) => task.id).join(",");
  const requestedTaskQueueRef = useRef<string[]>([]);
  const activeTaskIdRef = useRef<string | null>(null);
  const progressStagesRef = useRef(new Map<string, string>());
  const [taskControlEvent, setTaskControlEvent] = useState<TaskControlEvent>({
    revision: 0,
    type: "task_finished",
  });
  const [progress, setProgress] = useState<TaskProgress | null>(null);
  const [settings, setSettings] = useState<Settings>(defaults);
  const [settingsLoaded, setSettingsLoaded] = useState(false);
  const language = settings.app.language;
  const languageRef = useRef<Language>(language);
  languageRef.current = language;
  const t = translations[language];
  const [logs, setLogs] = useState<LogEntry[]>(() => [
    {
      id: 1,
      at: now(defaults.app.language),
      level: "info",
      message: translations[defaults.app.language].ready,
      source: "ui",
      important: true,
    },
  ]);
  const [normalLogs, setNormalLogs] = useState(logs);
  const pendingLogsRef = useRef<LogEntry[]>([]);
  const logFlushTimer = useRef<number | null>(null);
  const nextLogId = useRef(2);
  const [notice, setNotice] = useState<string | null>(null);
  const [windowInfo, setWindowInfo] = useState<WindowInfo | null>(null);
  const [frame, setFrame] = useState<CaptureFrame | null>(null);
  const [diagnosticError, setDiagnosticError] = useState<EngineEvent | null>(
    null,
  );
  const frameLoadSequence = useRef(0);
  const [templateMatch, setTemplateMatch] = useState<TemplateTestResult | null>(
    null,
  );
  const [lastPlannedAction, setLastPlannedAction] =
    useState<EngineEvent | null>(null);
  const [developerInputEnabled, setDeveloperInputEnabled] = useState(false);
  const [runtimeEvent, setRuntimeEvent] = useState<EngineEvent | null>(null);
  const appendLog = useCallback(
    (level: LogLevel, message: string, source = "engine", important = false, persist = true, detailedOnly = false) => {
      if (persist)
        void bridge.appendUiLog({ type: "ui_log", timestamp: new Date().toISOString(), level, source, message }).catch(console.error);
      pendingLogsRef.current.push({
        id: nextLogId.current++,
        at: now(languageRef.current),
        level,
        message,
        source,
        important,
        detailedOnly,
      });
      if (logFlushTimer.current === null)
        logFlushTimer.current = window.setTimeout(() => {
          const pending = pendingLogsRef.current;
          pendingLogsRef.current = [];
          logFlushTimer.current = null;
          setLogs((current) => appendLogBatch(current, pending));
          // Reuse the normal view unless a visible entry is added or evicted.
          setNormalLogs((current) => appendNormalLogBatch(current, pending));
        }, 100);
    },
    [],
  );
  const clearLogs = useCallback(() => {
    if (logFlushTimer.current !== null) window.clearTimeout(logFlushTimer.current);
    logFlushTimer.current = null;
    pendingLogsRef.current = [];
    setLogs([]);
    setNormalLogs([]);
  }, []);
  useEffect(() => () => {
    if (logFlushTimer.current !== null) window.clearTimeout(logFlushTimer.current);
    logFlushTimer.current = null;
  }, []);
  const updateFrame = useCallback(
    (nextFrame: CaptureFrame) => {
      const sequence = ++frameLoadSequence.current;
      setFrame(nextFrame);
      if (nextFrame.path)
        void bridge
          .readDebugFrame(nextFrame.path)
          .then((imageUrl) => {
            if (sequence === frameLoadSequence.current)
              setFrame({ ...nextFrame, imageUrl });
          })
          .catch((error) =>
            appendLog(
              "warning",
              translations[languageRef.current].readFrameFailed(
                asMessage(error),
              ),
              "debug",
            ),
          );
    },
    [appendLog],
  );
  const run = useCallback(
    async (label: string, action: () => Promise<void>) => {
      const current = translations[languageRef.current];
      try {
        await action();
        appendLog("info", current.actionSent(label), "ui");
        setNotice(current.actionNotice(label));
        return true;
      } catch (error) {
        const message = current.actionFailed(label, asMessage(error));
        appendLog("error", message, "ui");
        setNotice(message);
        return false;
      }
    },
    [appendLog],
  );
  const sendEngineCommand = useCallback(
    async (label: string, command: Record<string, unknown>) => {
      const previousQueue = requestedTaskQueueRef.current;
      const type = command.type;
      if (type === "start_task" && typeof command.task === "string")
        requestedTaskQueueRef.current = [command.task];
      else if (type === "start_tasks" && Array.isArray(command.tasks))
        requestedTaskQueueRef.current = command.tasks.filter(
          (task): task is string => typeof task === "string",
        );
      const sent = await run(label, () => bridge.sendEngineCommand(command));
      if (!sent && (type === "start_task" || type === "start_tasks"))
        requestedTaskQueueRef.current = previousQueue;
      return sent;
    },
    [run],
  );
  const refreshSnapshot = useCallback(async () => {
    try {
      setSnapshot(await bridge.getEngineSnapshot());
    } catch (error) {
      appendLog(
        "debug",
        translations[languageRef.current].snapshotFailed(asMessage(error)),
        "ui",
      );
    }
  }, [appendLog]);
  useEffect(() => {
    void refreshSnapshot();
    void bridge
      .loadSettings()
      .then((loaded) => {
        const enabled = new Set(
          (loaded.tasks?.enabled ?? defaults.tasks.enabled).map((id) =>
            id === "home_book" ? "blackjack" : id,
          ),
        );
        setTasks(initialTasks.map((task) => ({ ...task, enabled: enabled.has(task.id) })));
        setSettings({
          ...defaults,
          ...loaded,
          tasks: { enabled: initialTasks.filter((task) => enabled.has(task.id)).map((task) => task.id) },
          app: { ...defaults.app, ...loaded.app },
          engine: { ...defaults.engine, ...loaded.engine },
          game_window: { ...defaults.game_window, ...loaded.game_window },
          capture: { ...defaults.capture, ...loaded.capture },
          vision: { ...defaults.vision, ...loaded.vision },
          automation: { ...defaults.automation, ...loaded.automation },
          debug: { ...defaults.debug, ...loaded.debug },
        });
        setSettingsLoaded(true);
      })
      .catch(() => undefined);
    let dispose: (() => void) | undefined;
    let disposed = false;
    void bridge
      .onEngineEvent((event: EngineEvent) => {
        const current = translations[languageRef.current];
        if (event.type === "task_started" && event.task_id)
          progressStagesRef.current.delete(event.task_id);
        const stageMessage = progressLog(event, current.progress.stages, progressStagesRef.current);
        if (event.source === "engine.lifecycle" && event.engine_status) {
          setSnapshot((previous) => ({ ...previous, status: event.engine_status }));
          if (event.engine_status !== "starting" && event.engine_status !== "stopping")
            void refreshSnapshot();
        }
        const level: LogLevel =
          event.type === "error"
            ? "error"
            : event.level &&
                ["debug", "info", "warning", "error"].includes(event.level)
              ? event.level
              : "info";
        const eventMessage =
          stageMessage ??
          (event.type === "ready" ? `${current.engine} ${current.states.idle}` : undefined) ??
          (event.type === "task_finished" ? current.finishedEvent(current.progress.results[event.reason as keyof typeof current.progress.results] ?? event.reason ?? "?") : undefined) ??
          event.message ??
          (event.type === "task_state"
            ? current.taskEvent(event.state ?? "?")
            : event.type === "state_transition"
              ? current.transitionEvent(event.from ?? "?", event.to ?? "?")
              : event.type === "task_paused"
                ? current.taskEvent(current.taskStates.paused)
                : event.type === "task_resumed"
                  ? current.taskEvent(current.taskStates.running)
                  : (event.type ?? current.receivedEvent));
        const important =
          event.data?.important === true ||
          stageMessage !== null ||
          event.type === "ready" ||
          event.type === "task_paused" ||
          event.type === "task_resumed" ||
          event.type === "task_finished";
        appendLog(level, eventMessage, event.source ?? event.task_id ?? event.type ?? "engine", important, false, isWindowDiagnostic(event));
        if (event.type === "heartbeat") {
          setSnapshot((current) =>
            current.status === (event.engine_status ?? current.status) &&
            current.taskStatus === (event.task_status ?? current.taskStatus)
              ? current
              : {
                  ...current,
                  status: event.engine_status ?? current.status,
                  taskStatus: event.task_status ?? current.taskStatus,
                },
          );
          if (event.task_status === "paused" || event.task_status === "running")
            setTasks((current) => {
              if (!current.some((task) => (task.status === "running" || task.status === "paused") && task.status !== event.task_status))
                return current;
              return current.map((task) =>
                task.status === "running" || task.status === "paused"
                  ? { ...task, status: event.task_status as TaskItem["status"] }
                  : task,
              );
            });
        }
        if (event.type === "window_diagnostics") {
          setWindowInfo(event);
          if (event.found) setDiagnosticError(null);
        }
        if (
          (event.type === "heartbeat" || event.type === "window_health") &&
          typeof event.found === "boolean"
        )
          setWindowInfo(event);
        if (event.type === "debug_frame")
          updateFrame({
            path: event.path,
            width: event.width,
            height: event.height,
            state: event.state,
            source: event.source,
            captured_at: event.captured_at,
            capture_ms: event.capture_ms,
            file_size: event.file_size,
            region: event.region,
            hwnd: event.hwnd,
            request_id: event.request_id,
          });
        if (event.type === "debug_frame") setDiagnosticError(null);
        if (
          event.type === "error" &&
          [
            "WINDOW_NOT_FOUND",
            "WINDOW_INVALID",
            "CAPTURE_FAILED",
            "TEMPLATE_NOT_FOUND",
          ].includes(event.code ?? "")
        ) {
          setDiagnosticError(event);
          setNotice(event.message ?? event.code ?? current.receivedEvent);
        }
        if (event.type === "template_match")
          setTemplateMatch({
            template: event.template,
            found: event.found,
            confidence: event.confidence,
            bbox: event.bbox,
            threshold: event.threshold,
          });
        if (event.type === "planned_action") {
          setLastPlannedAction(event);
          if (event.action === "input_test_enabled")
            setDeveloperInputEnabled(Boolean(event.enabled));
        }
        if (
          event.type === "task_started" ||
          event.type === "task_paused" ||
          event.type === "task_resumed" ||
          event.type === "task_finished"
        )
          setTaskControlEvent((previous) => ({
            revision: previous.revision + 1,
            type: event.type as TaskControlEvent["type"],
            taskId: event.task_id,
          }));
        if (
          event.type === "task_state" ||
          event.type === "state_transition" ||
          event.type === "task_paused" ||
          event.type === "task_resumed" ||
          event.type === "task_finished"
        )
          setRuntimeEvent(event);
        if (event.type === "task_state")
          setTasks((current) =>
            current.map((task) =>
              task.status === "running" || task.status === "paused"
                ? { ...task, status: event.state === "paused" ? "paused" : "running" }
                : task,
            ),
          );
        if (event.type === "task_paused") {
          setTasks((current) =>
            current.map((task) =>
              task.id === event.task_id ||
              (!event.task_id && (task.status === "running" || task.status === "paused"))
                ? { ...task, status: "paused" }
                : task,
            ),
          );
          setSnapshot((current) => ({
            ...current,
            status: "paused",
            taskStatus: "paused",
            lastError: event.message ?? event.reason ?? current.lastError,
          }));
          setNotice(
            event.message ??
            (event.reason === "user_paused"
              ? current.taskEvent(current.taskStates.paused)
              : event.reason ?? current.receivedEvent),
          );
        }
        if (event.type === "task_resumed") {
          setTasks((current) =>
            current.map((task) =>
              task.id === event.task_id ||
              (!event.task_id && task.status === "paused")
                ? { ...task, status: "running" }
                : task,
            ),
          );
          setSnapshot((current) => ({
            ...current,
            status: "running",
            taskStatus: "running",
            lastError: null,
          }));
        }
        if (event.type === "task_started" && event.task_id) {
          activeTaskIdRef.current = event.task_id;
          if (!requestedTaskQueueRef.current.includes(event.task_id))
            requestedTaskQueueRef.current = [event.task_id];
          setProgress({ taskId: event.task_id, stage: "running", role: null, completedCycles: 0 });
        }
        if (event.type === "task_progress" && event.task_id) {
          const nextProgress: TaskProgress = {
            taskId: event.task_id,
            stage: event.stage ?? "running",
            role: event.role ?? null,
            completedCycles: event.completed_cycles ?? 0,
          };
          setProgress((previous) =>
            previous && previous.taskId === nextProgress.taskId &&
            previous.stage === nextProgress.stage && previous.role === nextProgress.role &&
            previous.completedCycles === nextProgress.completedCycles
              ? previous : nextProgress,
          );
        }
        if (event.type === "task_started" && event.task_id)
          setTasks((current) =>
            {
              const activeIndex = requestedTaskQueueRef.current.indexOf(event.task_id ?? "");
              const queuedIds = new Set(
                activeIndex >= 0
                  ? requestedTaskQueueRef.current.slice(activeIndex + 1)
                  : [],
              );
              return current.map((task) => ({
                ...task,
                status: task.id === event.task_id
                  ? "running"
                  : queuedIds.has(task.id)
                    ? "queued"
                    : "idle",
              }));
            },
          );
        if (event.type === "task_finished") {
          const finishedTaskId = activeTaskIdRef.current;
          const finishedIndex = finishedTaskId
            ? requestedTaskQueueRef.current.indexOf(finishedTaskId)
            : -1;
          requestedTaskQueueRef.current = event.reason === "completed" && finishedIndex >= 0
            ? requestedTaskQueueRef.current.slice(finishedIndex + 1)
            : [];
          activeTaskIdRef.current = null;
          setProgress((current) => current && { ...current, result: event.reason ?? "finished" });
          setTasks((current) =>
            {
              const queuedIds = new Set(requestedTaskQueueRef.current);
              return current.map((task) => ({
                ...task,
                status: queuedIds.has(task.id) ? "queued" : "idle",
              }));
            },
          );
          setSnapshot((current) => ({ ...current, taskStatus: "idle" }));
        }
        if (event.type === "error" && !event.recoverable)
          setSnapshot((current) =>
            current.taskStatus === "running" || current.taskStatus === "paused"
              ? { ...current, status: "paused", taskStatus: "paused", lastError: event.message ?? current.lastError }
              : { ...current, status: "error", lastError: event.message ?? current.lastError },
          );
      })
      .then((unlisten) => {
        if (disposed) void unlisten();
        else dispose = unlisten;
      })
      .catch(() => undefined);
    const timer = window.setInterval(() => void refreshSnapshot(), 5_000);
    return () => {
      disposed = true;
      window.clearInterval(timer);
      dispose?.();
    };
  }, [appendLog, refreshSnapshot, updateFrame]);
  useEffect(() => {
    document.documentElement.dataset.theme = settings.app.theme;
  }, [settings.app.theme]);
  useEffect(() => {
    if (!settingsLoaded) return;
    void bridge
      .setPowerPrevention(settings.app.prevent_system_sleep, settings.app.prevent_display_sleep)
      .catch((error) => {
        appendLog("error", asMessage(error), "power");
      });
  }, [
    appendLog,
    settingsLoaded,
    settings.app.prevent_system_sleep,
    settings.app.prevent_display_sleep,
  ]);
  useEffect(() => {
    if (!settingsLoaded) return;
    const timer = window.setTimeout(() => {
      void (async () => {
        try {
          const nextSettings = { ...settings, tasks: { enabled: enabledTaskIds ? enabledTaskIds.split(",") : [] } };
          await bridge.saveSettings(nextSettings);
          if (
            !["stopped", "starting", "stopping", "error"].includes(
              engineStatusRef.current,
            )
          ) {
            await bridge.sendEngineCommand({
              type: "reload_settings",
              settings: nextSettings,
            });
          }
        } catch (error) {
          const message = translations[languageRef.current].actionFailed(
            translations[languageRef.current].nav.settings,
            asMessage(error),
          );
          appendLog("error", message, "settings");
          setNotice(message);
        }
      })();
    }, 250);
    return () => window.clearTimeout(timer);
  }, [appendLog, settings, settingsLoaded, enabledTaskIds]);
  useEffect(() => {
    document.title = t.about.name;
  }, [t.about.name]);
  useEffect(
    () => () => {
      if (pageTransitionTimer.current !== null)
        window.clearTimeout(pageTransitionTimer.current);
    },
    [],
  );
  const navigate = useCallback(
    (nextPage: Page) => {
      if (nextPage === page || pagePhase === "leave") return;
      if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
        setPage(nextPage);
        setPagePhase("enter");
        workspaceRef.current?.scrollTo({ top: 0 });
        return;
      }
      workspaceRef.current?.scrollTo({ top: 0, behavior: "smooth" });
      setPagePhase("leave");
      pageTransitionTimer.current = window.setTimeout(() => {
        setPage(nextPage);
        setPagePhase("enter");
        pageTransitionTimer.current = null;
        window.requestAnimationFrame(() =>
          workspaceRef.current?.scrollTo({ top: 0 }),
        );
      }, 160);
    },
    [page, pagePhase],
  );
  useEffect(() => {
    if (
      !settings.app.developer_mode &&
      page === "runtime"
    ) {
      setPage("settings");
      setPagePhase("enter");
      workspaceRef.current?.scrollTo({ top: 0 });
    }
  }, [page, settings.app.developer_mode]);
  const actions = {
    start: () =>
      run(t.dashboard.start, async () => {
        await bridge.startEngine();
        await refreshSnapshot();
      }),
    stop: () =>
      run(t.dashboard.stop, async () => {
        await bridge.stopEngine();
        await refreshSnapshot();
      }),
    restart: () =>
      run(t.dashboard.restart, async () => {
        await bridge.restartEngine();
        await refreshSnapshot();
      }),
  };
  const content = {
    dashboard: (
      <Dashboard
        language={language}
        snapshot={snapshot}
        tasks={tasks}
        onStart={actions.start}
        onStop={actions.stop}
        onRestart={actions.restart}
        onNavigate={navigate}
      />
    ),
    tasks: (
      <Tasks
        language={language}
        tasks={tasks}
        onChange={setTasks}
        settings={settings}
        setSettings={setSettings}
        taskControlEvent={taskControlEvent}
        onCommand={sendEngineCommand}
      />
    ),
    runtime: (
      <Runtime
        language={language}
        snapshot={snapshot}
        runtimeEvent={runtimeEvent}
        onRefresh={refreshSnapshot}
        onCommand={sendEngineCommand}
      />
    ),
    debug: (
      <Debug
        language={language}
        windowInfo={windowInfo}
        frame={frame}
        diagnosticError={diagnosticError}
        templateMatch={templateMatch}
        lastPlannedAction={lastPlannedAction}
        developerInputEnabled={developerInputEnabled}
        appendLog={appendLog}
        setNotice={setNotice}
      />
    ),
    logs: (
      <Logs language={language} entries={settings.debug.verbose_logs ? logs : normalLogs} verbose={settings.debug.verbose_logs} onClear={clearLogs} />
    ),
    settings: (
      <SettingsPage
        language={language}
        settings={settings}
        setSettings={setSettings}
      />
    ),
    about: <About language={language} />,
  }[page];
  const visibleNav = nav.filter(
    (item) =>
      settings.app.developer_mode ||
      item.id !== "runtime",
  );
  return (
    <>
      <WindowTitleBar language={language} />
      <main className="shell">
        <aside className="sidebar">
          <div className="brand sidebar-card" title={t.about.name}>
            <span className="brand-mark">V</span>
            <span>{t.about.name}</span>
          </div>
          <nav>
            {visibleNav.map((item) => (
              <button
                key={item.id}
                className={page === item.id ? "nav-item active" : "nav-item"}
                onClick={() => navigate(item.id)}
              >
                <span>{item.icon}</span>
                {t.nav[item.id]}
              </button>
            ))}
          </nav>
          <div className="sidebar-bottom">
            {progress && <div className="progress-card sidebar-card" title={`${taskCopy(initialTasks.find((task) => task.id === progress.taskId) ?? initialTasks[0], language)[0]} · ${progress.stage}`}>
              <strong>{t.progress.title} · {taskCopy(initialTasks.find((task) => task.id === progress.taskId) ?? initialTasks[0], language)[0]}</strong>
              <small className="progress-stage" key={`${progress.taskId}:${progress.stage}:${progress.result ?? ""}`} aria-live="polite">{progress.result ? (t.progress.results[progress.result as keyof typeof t.progress.results] ?? progress.result) : (t.progress.stages[progress.stage as keyof typeof t.progress.stages] ?? progress.stage)}</small>
              <small>{t.progress.cycles}: {progress.completedCycles} · {t.progress.role}: {progress.role ? t.progress[progress.role] : t.progress.unknown}</small>
            </div>}
            <div className="sidebar-footer" title={`${t.localConsole}\n${t.localOnly}\nv0.1.0`}>
              <span className={`local-status-dot ${engineState(snapshot)}`} />
              <span className="local-status-copy">
                <strong>{t.localConsole}</strong>
                <small>{t.localOnly}</small>
              </span>
              <b>v0.1.0</b>
            </div>
          </div>
        </aside>
        <section className="workspace" ref={workspaceRef}>
          <header className="topbar">
            <div>
              <h1>{t.nav[page]}</h1>
              <p>IdentityV Assistant</p>
            </div>
            <div className="engine-status">
              <span className={`dot ${engineState(snapshot)}`} />
              {t.engine} {stateLabel(engineState(snapshot), language)}
            </div>
          </header>
          {notice && (
            <div className="notice">
              <span>{notice}</span>
              <button aria-label={t.closeNotice} onClick={() => setNotice(null)}>
                ×
              </button>
            </div>
          )}
          <div className={`page-content page-${page} page-${pagePhase}`}>
            {content}
          </div>
        </section>
      </main>
    </>
  );
}

function Dashboard({
  language,
  snapshot,
  tasks,
  onStart,
  onStop,
  onRestart,
  onNavigate,
}: {
  language: Language;
  snapshot: EngineSnapshot;
  tasks: TaskItem[];
  onStart: () => void;
  onStop: () => void;
  onRestart: () => void;
  onNavigate: (page: Page) => void;
}) {
  const t = translations[language].dashboard;
  const state = engineState(snapshot);
  return (
    <>
      <div className="hero-card">
        <div>
          <span className="eyebrow">{t.eyebrow}</span>
          <h2>{stateLabel(state, language)}</h2>
          <p>
            {snapshot.taskStatus && snapshot.taskStatus !== "idle"
              ? `${t.taskState}${statusLabel(snapshot.taskStatus as TaskItem["status"], language)}`
              : t.noTask}
          </p>
        </div>
        <div className="button-row">
          <button
            className="primary"
            disabled={state === "running" || state === "starting"}
            onClick={onStart}
          >
            {t.start}
          </button>
          <button
            disabled={state === "stopped" || state === "stopping"}
            onClick={onStop}
          >
            {t.stop}
          </button>
          <button onClick={onRestart}>{t.restart}</button>
        </div>
      </div>
      <div className="stat-grid">
        <Stat
          label={t.process}
          value={snapshot.pid ? `#${snapshot.pid}` : t.notStarted}
        />
        <Stat
          label={t.heartbeat}
          value={
            snapshot.heartbeatAgeSeconds == null
              ? "—"
              : `${snapshot.heartbeatAgeSeconds.toFixed(1)} s`
          }
        />
        <Stat
          label={t.enabledTasks}
          value={String(tasks.filter((task) => task.enabled).length)}
        />
        <Stat label={t.version} value={snapshot.engineVersion ?? "—"} />
      </div>
      <div className="two-column">
        <section className="panel">
          <div className="panel-title">
            <div>
              <h3>{t.overview}</h3>
              <p>{t.overviewHint}</p>
            </div>
            <button className="link-button" onClick={() => onNavigate("tasks")}>
              {t.manage}
            </button>
          </div>
          <div className="task-summary">
            {tasks.map((task) => (
              <div key={task.id} className="summary-row">
                <span className={`task-indicator ${task.status}`} />
                <span>{taskCopy(task, language)[0]}</span>
                <small>{statusLabel(task.status, language)}</small>
              </div>
            ))}
          </div>
        </section>
        <section className="panel">
          <div className="panel-title">
            <div>
              <h3>{t.quickStart}</h3>
              <p>{t.quickHint}</p>
            </div>
          </div>
          <ol className="steps">
            {t.steps.map((step) => (
              <li key={step}>{step}</li>
            ))}
          </ol>
        </section>
      </div>
    </>
  );
}
function Stat({ label, value }: { label: string; value: string }) {
  return (
    <section className="stat">
      <span>{label}</span>
      <strong>{value}</strong>
    </section>
  );
}
function Tasks({
  language,
  tasks,
  onChange,
  settings,
  setSettings,
  taskControlEvent,
  onCommand,
}: {
  language: Language;
  tasks: TaskItem[];
  onChange: (value: TaskItem[] | ((current: TaskItem[]) => TaskItem[])) => void;
  settings: Settings;
  setSettings: (value: Settings) => void;
  taskControlEvent: TaskControlEvent;
  onCommand: (label: string, command: Record<string, unknown>) => Promise<boolean>;
}) {
  const t = translations[language].tasks;
  const labels = translations[language].settings;
  const [editingTask, setEditingTask] = useState<string | null>(null);
  const [pendingCommand, setPendingCommand] = useState<{
    type: "start" | "pause" | "resume" | "stop";
    afterRevision: number;
  } | null>(null);
  const updateAutomation = (patch: Partial<Settings["automation"]>) =>
    setSettings({ ...settings, automation: { ...settings.automation, ...patch } });
  const timingFields = [
    ["screenshot_after_click_ms", labels.screenshotDelay, 0, 10000],
    ["click_after_detection_ms", labels.clickDelay, 0, 10000],
    ["click_position_jitter_px", labels.positionJitter, 1, 50],
    ["click_interval_jitter_ms", labels.intervalJitter, 1, 2000],
    ["click_duration_jitter_ms", labels.durationJitter, 1, 2000],
  ] as const;
  const update = (id: string, patch: Partial<TaskItem>) =>
    onChange((current) =>
      current.map((task) => (task.id === id ? { ...task, ...patch } : task)),
    );
  const enabledTasks = tasks.filter((task) => task.enabled);
  const hasActiveTask = tasks.some((task) => task.status !== "idle");
  const commandPending = pendingCommand !== null;
  useEffect(() => {
    if (!pendingCommand || taskControlEvent.revision <= pendingCommand.afterRevision)
      return;
    if (
      (pendingCommand.type === "start" && ["task_started", "task_paused", "task_finished"].includes(taskControlEvent.type)) ||
      (pendingCommand.type === "pause" && ["task_paused", "task_finished"].includes(taskControlEvent.type)) ||
      (pendingCommand.type === "resume" && ["task_resumed", "task_paused", "task_finished"].includes(taskControlEvent.type)) ||
      (pendingCommand.type === "stop" && taskControlEvent.type === "task_finished")
    )
      setPendingCommand(null);
  }, [pendingCommand, taskControlEvent]);
  const startFirst = async () => {
    if (!enabledTasks.length || hasActiveTask || commandPending) return;
    setPendingCommand({ type: "start", afterRevision: taskControlEvent.revision });
    try {
      const sent = await onCommand(t.startEnabled, {
        type: "start_tasks",
        tasks: enabledTasks.map((task) => task.id),
      });
      if (!sent) setPendingCommand(null);
    } catch {
      setPendingCommand(null);
    }
  };
  return (
    <>
      <div className="section-heading">
        <div>
          <h2>{t.title}</h2>
          <p>{t.hint}</p>
        </div>
        <button
          className="primary"
          disabled={!enabledTasks.length || hasActiveTask || commandPending}
          onClick={() => void startFirst()}
        >
          {t.startEnabled}
        </button>
      </div>
      <div className="task-list">
        {tasks.map((task) => {
          const loopKey = task.id === "blackjack"
            ? "blackjack_infinite_loop"
            : task.id === "hunter_rank"
              ? "hunter_rank_infinite_loop"
              : task.id === "hide_seek"
                ? "hide_seek_infinite_loop"
                : "hunter_match_infinite_loop";
          const action: "start" | "pause" | "resume" =
            task.status === "running"
              ? "pause"
              : task.status === "paused"
                ? "resume"
                : "start";
          const label =
            action === "pause"
              ? t.pauseTask
              : action === "resume"
                ? t.resumeTask
                : t.startTask;
          const [name, description] = taskCopy(task, language);
          return (
            <div className="task-block" key={task.id}>
            <article className="task-card">
              <label className="switch">
                <input
                  type="checkbox"
                  checked={task.enabled}
                  onChange={(event) =>
                    update(task.id, { enabled: event.target.checked })
                  }
                />
                <span />
              </label>
              <div className="task-copy">
                <h3>{name}</h3>
                <p>{description}</p>
              </div>
              <button
                disabled={commandPending || (action === "start" && hasActiveTask)}
                onClick={() => void (async () => {
                  setPendingCommand({ type: action, afterRevision: taskControlEvent.revision });
                  try {
                    const sent = await onCommand(
                      label,
                      action === "pause"
                        ? { type: "pause_task" }
                        : action === "resume"
                          ? { type: "resume_task" }
                          : { type: "start_task", task: task.id },
                    );
                    if (!sent) setPendingCommand(null);
                  } catch {
                    setPendingCommand(null);
                  }
                })()}
              >
                {action === "pause"
                  ? t.pause
                  : action === "resume"
                    ? t.resume
                    : t.execute}
              </button>
              <button className="task-edit-button" onClick={() => setEditingTask(editingTask === task.id ? null : task.id)}>
                {editingTask === task.id ? t.closeEdit : t.edit}
              </button>
              <button
                className="task-stop-button"
                disabled={task.status === "idle" || commandPending}
                onClick={() => void (async () => {
                  setPendingCommand({ type: "stop", afterRevision: taskControlEvent.revision });
                  try {
                    const sent = await onCommand(t.stopTask, { type: "stop_task" });
                    if (!sent) setPendingCommand(null);
                  } catch {
                    setPendingCommand(null);
                  }
                })()}
              >
                {translations[language].dashboard.stop}
              </button>
              <span key={task.status} className={`tag task-status ${task.status}`}>{statusLabel(task.status, language)}</span>
            </article>
            {editingTask === task.id && <section className="task-editor panel">
              <h3>{name} · {t.edit}</h3>
              {task.id === "login_lobby" ? <>
                <label>{labels.loginPlatform}
                  <select value={settings.automation.login_platform} onChange={(event) => updateAutomation({ login_platform: event.target.value as Settings["automation"]["login_platform"] })}>
                    <option value="android">{labels.android}</option>
                    <option value="ios">{labels.ios}</option>
                  </select>
                </label>
                <label>{labels.loginTimeout}
                  <input type="number" min="30" max="900" value={settings.automation.login_timeout_seconds} onChange={(event) => updateAutomation({ login_timeout_seconds: Math.max(30, Math.min(900, Number(event.target.value))) })} />
                </label>
                {timingFields.map(([key, label, min, max]) => <label key={key}>{label}
                  <input type="number" min={min} max={max} value={settings.automation[key]} onChange={(event) => updateAutomation({ [key]: Math.max(min, Math.min(max, Number(event.target.value))) })} />
                </label>)}
              </> : <>
                <label className="check-label"><input type="checkbox" checked={settings.automation[loopKey]} onChange={(event) => updateAutomation({ [loopKey]: event.target.checked })} />{t.infiniteLoop}</label>
                {task.id === "blackjack" && <label>{t.cameraScale}
                  <input type="number" min="20" max="300" value={settings.automation.camera_turn_scale_percent} onChange={(event) => updateAutomation({ camera_turn_scale_percent: Math.max(20, Math.min(300, Number(event.target.value))) })} />
                </label>}
                {timingFields.map(([key, label, min, max]) => <label key={key}>{label}
                  <input type="number" min={min} max={max} value={settings.automation[key]} onChange={(event) => updateAutomation({ [key]: Math.max(min, Math.min(max, Number(event.target.value))) })} />
                </label>)}
              </>}
              <p>{t.editHint}</p>
            </section>}
            </div>
          );
        })}
      </div>
    </>
  );
}
function Runtime({
  language,
  snapshot,
  runtimeEvent,
  onRefresh,
  onCommand,
}: {
  language: Language;
  snapshot: EngineSnapshot;
  runtimeEvent: EngineEvent | null;
  onRefresh: () => Promise<void>;
  onCommand: (label: string, command: Record<string, unknown>) => void;
}) {
  const t = translations[language].runtime;
  const [command, setCommand] = useState('{ "type": "ping" }');
  const [error, setError] = useState<string | null>(null);
  const send = () => {
    try {
      const parsed = JSON.parse(command) as Record<string, unknown>;
      setError(null);
      onCommand(t.sendCommand, parsed);
    } catch {
      setError(t.invalidJson);
    }
  };
  return (
    <div className="runtime-grid">
      <section className="panel">
        <div className="panel-title">
          <div>
            <h2>{t.snapshot}</h2>
            <p>{t.refreshHint}</p>
          </div>
          <button onClick={() => void onRefresh()}>{t.refresh}</button>
        </div>
        <pre className="code-block">{JSON.stringify(snapshot, null, 2)}</pre>
        {runtimeEvent && (
          <>
            <h3 className="event-title">{t.latestEvent}</h3>
            <pre className="code-block">
              {JSON.stringify(runtimeEvent, null, 2)}
            </pre>
          </>
        )}
      </section>
      <section className="panel">
        <div className="panel-title">
          <div>
            <h2>{t.command}</h2>
            <p>{t.commandHint}</p>
          </div>
        </div>
        <textarea
          className="command-input"
          value={command}
          onChange={(event) => setCommand(event.target.value)}
          spellCheck={false}
        />
        {error && <p className="form-error">{error}</p>}
        <button className="primary" onClick={send}>
          {t.send}
        </button>
      </section>
    </div>
  );
}
function Debug({
  language,
  windowInfo,
  frame,
  diagnosticError,
  templateMatch,
  lastPlannedAction,
  developerInputEnabled,
  appendLog,
  setNotice,
}: {
  language: Language;
  windowInfo: WindowInfo | null;
  frame: CaptureFrame | null;
  diagnosticError: EngineEvent | null;
  templateMatch: TemplateTestResult | null;
  lastPlannedAction: EngineEvent | null;
  developerInputEnabled: boolean;
  appendLog: (level: LogLevel, message: string, source?: string) => void;
  setNotice: (value: string) => void;
}) {
  const t = translations[language].debug;
  const [templateName, setTemplateName] = useState("test/mock_template.pgm");
  const [busy, setBusy] = useState<string | null>(null);
  const request = async (label: string, action: () => Promise<unknown>) => {
    setBusy(label);
    try {
      await action();
      appendLog("info", translations[language].waitingEvent(label), "debug");
      setNotice(translations[language].actionNotice(label));
    } catch (error) {
      const message = translations[language].actionFailed(
        label,
        asMessage(error),
      );
      appendLog("error", message, "debug");
      setNotice(message);
    } finally {
      setBusy(null);
    }
  };
  const sendInput = async (label: string, action: Record<string, unknown>) =>
    request(label, async () => {
      if (!developerInputEnabled)
        await bridge.inputTest({ type: "set_enabled", enabled: true });
      await bridge.inputTest(action);
    });
  const dash = "—";
  const matchReasons = windowInfo?.match_reasons?.map(
    (reason) =>
      t.matchReasonLabels[
        reason as keyof typeof t.matchReasonLabels
      ] ?? reason,
  );
  return (
    <div className="debug-grid">
      {diagnosticError && (
        <section className="diagnostic-error" role="alert">
          <strong>{t.errorTitle}</strong>
          <span>{diagnosticError.message ?? diagnosticError.code}</span>
          {diagnosticError.request_id && (
            <small>
              {t.requestId}: {diagnosticError.request_id}
            </small>
          )}
        </section>
      )}
      <section className="panel">
        <h2>{t.gameWindow}</h2>
        <p>{t.gameWindowHint}</p>
        <button
          className="primary"
          disabled={Boolean(busy)}
          onClick={() => void request(t.detectWindow, bridge.detectWindow)}
        >
          {t.detect}
        </button>
        {windowInfo && (
          <dl className="key-values">
            <dt>{t.status}</dt>
            <dd>{windowInfo.found ? t.found : t.notFound}</dd>
            <dt>{t.title}</dt>
            <dd>{windowInfo.title || dash}</dd>
            <dt>{t.process}</dt>
            <dd>
              {[
                windowInfo.process_name,
                windowInfo.process_id && `#${windowInfo.process_id}`,
              ]
                .filter(Boolean)
                .join(" ") || dash}
            </dd>
            <dt>{t.className}</dt>
            <dd>{windowInfo.class_name || dash}</dd>
            <dt>{t.clientRect}</dt>
            <dd>
              {windowInfo.client_rect
                ? `(${windowInfo.client_rect.left ?? "?"}, ${windowInfo.client_rect.top ?? "?"}) · ${windowInfo.client_rect.width ?? "?"} × ${windowInfo.client_rect.height ?? "?"}`
                : dash}
            </dd>
            <dt>{t.dpi}</dt>
            <dd>
              {windowInfo.dpi_scale == null ? dash : `${windowInfo.dpi_scale}×`}
            </dd>
            <dt>{t.windowState}</dt>
            <dd>
              {windowInfo.is_foreground == null &&
              windowInfo.is_minimized == null
                ? dash
                : `${windowInfo.is_foreground ? t.foreground : t.background} · ${windowInfo.is_minimized ? t.minimized : t.restored}`}
            </dd>
            <dt>{t.responding}</dt>
            <dd>
              {windowInfo.is_responding == null
                ? dash
                : windowInfo.is_responding
                  ? t.responsive
                  : t.unresponsive}
            </dd>
            <dt>{t.matchReasons}</dt>
            <dd>
              {matchReasons?.length
                ? matchReasons.join(language === "zh-CN" ? "、" : ", ")
                : dash}
            </dd>
            <dt>{t.captureBackend}</dt>
            <dd>{windowInfo.capture_backend ?? dash}</dd>
            <dt>{t.configuredBackend}</dt>
            <dd>{windowInfo.capture_backend_configured ?? dash}</dd>
            <dt>{t.captureNote}</dt>
            <dd>{windowInfo.capture_last_error ?? dash}</dd>
            <dt>{t.requestId}</dt>
            <dd>{windowInfo.request_id ?? dash}</dd>
          </dl>
        )}
      </section>
      <section className="panel">
        <h2>{t.preview}</h2>
        <p>{t.previewHint}</p>
        <button
          disabled={Boolean(busy)}
          onClick={() => void request(t.captureFrame, bridge.captureFrame)}
        >
          {t.capture}
        </button>
        <div className="preview">
          {frame?.imageUrl ? (
            <img src={frame.imageUrl} alt={t.captureAlt} />
          ) : frame ? (
            <span>
              {frame.width} × {frame.height}
              <br />
              {frame.source ?? "unknown"}
              <br />
              <small>{frame.path}</small>
            </span>
          ) : (
            <span>{t.noFrame}</span>
          )}
        </div>
        {frame && (
          <dl className="key-values capture-details">
            <dt>{t.resolution}</dt>
            <dd>
              {frame.width ?? "?"} × {frame.height ?? "?"}
            </dd>
            <dt>{t.captureBackend}</dt>
            <dd>{frame.source ?? dash}</dd>
            <dt>{t.filePath}</dt>
            <dd>{frame.path ?? dash}</dd>
            <dt>{t.capturedAt}</dt>
            <dd>{frame.captured_at ?? dash}</dd>
            <dt>{t.duration}</dt>
            <dd>
              {frame.capture_ms == null
                ? dash
                : `${frame.capture_ms.toFixed(1)} ms`}
            </dd>
            <dt>{t.fileSize}</dt>
            <dd>
              {frame.file_size == null
                ? dash
                : `${(frame.file_size / 1024).toFixed(1)} KiB`}
            </dd>
            <dt>{t.captureRegion}</dt>
            <dd>
              {frame.region
                ? `(${frame.region.left ?? "?"}, ${frame.region.top ?? "?"}) · ${frame.region.width ?? "?"} × ${frame.region.height ?? "?"}`
                : dash}
            </dd>
            <dt>{t.windowHandle}</dt>
            <dd>
              {frame.hwnd == null ? dash : `0x${frame.hwnd.toString(16)}`}
            </dd>
            <dt>{t.requestId}</dt>
            <dd>{frame.request_id ?? dash}</dd>
          </dl>
        )}
      </section>
      <section className="panel">
        <h2>{t.template}</h2>
        <p>{t.templateHint}</p>
        <label>
          {t.templatePath}
          <input
            value={templateName}
            onChange={(event) => setTemplateName(event.target.value)}
            placeholder={t.templatePlaceholder}
          />
        </label>
        <button
          disabled={Boolean(busy) || !templateName.trim()}
          onClick={() =>
            void request(t.testTemplateRequest, () =>
              bridge.testTemplate(templateName.trim()),
            )
          }
        >
          {t.testTemplate}
        </button>
        {templateMatch && (
          <dl className="key-values">
            <dt>{t.template}</dt>
            <dd>{templateMatch.template ?? dash}</dd>
            <dt>{t.result}</dt>
            <dd>{templateMatch.found ? t.matched : t.unmatched}</dd>
            <dt>{t.confidence}</dt>
            <dd>{templateMatch.confidence?.toFixed(3) ?? dash}</dd>
          </dl>
        )}
      </section>
      <section className="panel">
        <h2>{t.input}</h2>
        <p>
          {t.inputHint}
          {developerInputEnabled ? t.enabled : t.disabled}.
        </p>
        <div className="button-row">
          <button
            disabled={Boolean(busy)}
            onClick={() =>
              void sendInput(t.clickTest, { type: "click", x: 100, y: 100 })
            }
          >
            {t.testClick}
          </button>
          <button
            disabled={Boolean(busy)}
            onClick={() =>
              void sendInput(t.keyTest, { type: "press", key: "esc" })
            }
          >
            {t.testEsc}
          </button>
        </div>
        {lastPlannedAction && (
          <dl className="key-values">
            <dt>{t.lastAction}</dt>
            <dd>{lastPlannedAction.action ?? dash}</dd>
          </dl>
        )}
      </section>
    </div>
  );
}
const Logs = memo(function Logs({
  language,
  entries,
  verbose,
  onClear,
}: {
  language: Language;
  entries: LogEntry[];
  verbose: boolean;
  onClear: () => void;
}) {
  const t = translations[language].logs;
  const [level, setLevel] = useState<"all" | LogLevel>("all");
  const [query, setQuery] = useState("");
  const deferredQuery = useDeferredValue(query);
  const [page, setPage] = useState(1);
  const [jump, setJump] = useState("");
  const shown = useMemo(
    () => filterLogs(entries, level, deferredQuery, verbose),
    [entries, level, deferredQuery, verbose],
  );
  const pageCount = Math.max(1, Math.min(99, Math.ceil(shown.length / 1000)));
  const currentPage = Math.min(page, pageCount);
  const visible = shown.slice((currentPage - 1) * 1000, currentPage * 1000);
  const pages = [...new Set([1, currentPage - 1, currentPage, currentPage + 1, pageCount])]
    .filter((value) => value >= 1 && value <= pageCount).sort((a, b) => a - b);
  return (
    <section className="panel logs-panel">
      <div className="log-toolbar">
        <div className="filters">
          <select
            value={level}
            onChange={(event) => {
              setLevel(event.target.value as "all" | LogLevel);
              setPage(1);
            }}
          >
            <option value="all">{t.all}</option>
            {(["debug", "info", "warning", "error"] as const).map((item) => (
              <option key={item} value={item}>
                {translations[language].levels[item]}
              </option>
            ))}
          </select>
          <input
            placeholder={t.filter}
            value={query}
            onChange={(event) => { setQuery(event.target.value); setPage(1); }}
          />
        </div>
        <button onClick={() => { onClear(); setPage(1); }}>{t.clear}</button>
      </div>
      <div className="log-table">
        <div className="log-head">
          <span>{t.time}</span>
          <span>{t.level}</span>
          <span>{t.source}</span>
          <span>{t.message}</span>
        </div>
        {visible.length ? (
          visible.map((entry) => (
            <div className="log-row" key={entry.id}>
              <time>{entry.at}</time>
              <span>
                <b className={`level ${entry.level}`}>
                  {translations[language].levels[entry.level]}
                </b>
              </span>
              <span>{entry.source}</span>
              <span>{entry.message}</span>
            </div>
          ))
        ) : (
          <p className="empty">{t.empty}</p>
        )}
      </div>
      <div className="log-pagination">
        <span>{shown.length} {t.total}</span>
        <button disabled={currentPage === 1} onClick={() => setPage(currentPage - 1)}>{t.previous}</button>
        {pages.map((value, index) => (
          <span key={value}>
            {index > 0 && value - pages[index - 1] > 1 && <span className="page-gap">…</span>}
            <button className={value === currentPage ? "active" : ""} onClick={() => setPage(value)}>{value}</button>
          </span>
        ))}
        <button disabled={currentPage === pageCount} onClick={() => setPage(currentPage + 1)}>{t.next}</button>
        {pageCount > 5 && <form onSubmit={(event) => {
          event.preventDefault();
          const target = Number(jump);
          if (Number.isInteger(target) && target >= 1 && target <= pageCount) setPage(target);
          setJump("");
        }}><input aria-label={t.page} type="number" min="1" max={pageCount} value={jump} onChange={(event) => setJump(event.target.value)} /><button type="submit">{t.jump}</button></form>}
      </div>
    </section>
  );
});
function SettingsPage({
  language,
  settings,
  setSettings,
}: {
  language: Language;
  settings: Settings;
  setSettings: (value: Settings) => void;
}) {
  const t = translations[language].settings;
  const updateApp = (patch: Partial<Settings["app"]>) =>
    setSettings({ ...settings, app: { ...settings.app, ...patch } });
  const updateEngine = (patch: Partial<Settings["engine"]>) =>
    setSettings({ ...settings, engine: { ...settings.engine, ...patch } });
  const updateWindow = (patch: Partial<Settings["game_window"]>) =>
    setSettings({
      ...settings,
      game_window: { ...settings.game_window, ...patch },
    });
  const updateCapture = (patch: Partial<Settings["capture"]>) =>
    setSettings({ ...settings, capture: { ...settings.capture, ...patch } });
  const updateVision = (patch: Partial<Settings["vision"]>) =>
    setSettings({ ...settings, vision: { ...settings.vision, ...patch } });
  const updateDebug = (patch: Partial<Settings["debug"]>) =>
    setSettings({ ...settings, debug: { ...settings.debug, ...patch } });
  const keywords = (value: string) =>
    value
      .split(/[,，]/)
      .map((item) => item.trim())
      .filter(Boolean);
  return (
    <section className="settings-form">
      <section className="panel">
        <h2>{t.general}</h2>
        <p>{t.generalHint}</p>
        <label>
          {t.theme}
          <select
            value={settings.app.theme}
            onChange={(event) =>
              updateApp({
                theme: event.target.value as Settings["app"]["theme"],
              })
            }
          >
            <option value="system">{t.system}</option>
            <option value="light">{t.light}</option>
            <option value="dark">{t.dark}</option>
          </select>
        </label>
        <label>
          {t.language}
          <select
            value={settings.app.language}
            onChange={(event) =>
              updateApp({ language: event.target.value as Language })
            }
          >
            <option value="zh-CN">简体中文</option>
            <option value="en-US">English</option>
          </select>
        </label>
        <label className="check-label">
          <input
            type="checkbox"
            checked={settings.app.launch_engine_on_start}
            onChange={(event) =>
              updateApp({ launch_engine_on_start: event.target.checked })
            }
          />
          {t.launch}
        </label>
        <label className="check-label">
          <input
            type="checkbox"
            checked={settings.app.prevent_system_sleep}
            onChange={(event) =>
              updateApp({ prevent_system_sleep: event.target.checked })
            }
          />
          {t.preventSystemSleep}
        </label>
        <label className="check-label">
          <input
            type="checkbox"
            checked={settings.app.prevent_display_sleep}
            onChange={(event) =>
              updateApp({ prevent_display_sleep: event.target.checked })
            }
          />
          {t.preventDisplaySleep}
        </label>
        <label className="check-label developer-toggle">
          <input
            type="checkbox"
            checked={settings.app.developer_mode}
            onChange={(event) =>
              updateApp({ developer_mode: event.target.checked })
            }
          />
          <span>
            {t.developerMode}
            <small>{t.developerModeHint}</small>
          </span>
        </label>
      </section>
      <section className="panel developer-panel">
        <h2>{t.gameWindow}</h2>
        <p>{t.gameWindowHint}</p>
        <label>
          {t.windowResolution}
          <select value={settings.game_window.resolution} onChange={(event) => updateWindow({ resolution: event.target.value as Settings["game_window"]["resolution"] })}>
            {["2560x1440", "1920x1080", "1600x900", "1280x720"].map((value) => <option key={value} value={value}>{value.replace("x", " × ")}</option>)}
          </select>
        </label>
        <label>
          {t.windowPosition}
          <select value={settings.game_window.position} onChange={(event) => updateWindow({ position: event.target.value as Settings["game_window"]["position"] })}>
            {Object.entries(t.windowPositions).map(([value, label]) => <option key={value} value={value}>{label}</option>)}
          </select>
        </label>
        <p>{t.windowLayoutHint}</p>
        <label>
          {t.titleKeywords}
          <input
            value={settings.game_window.title_contains.join("，")}
            onChange={(event) =>
              updateWindow({ title_contains: keywords(event.target.value) })
            }
            placeholder="第五人格，Identity V"
          />
        </label>
        <label>
          {t.processNames}
          <input
            value={settings.game_window.process_names.join("，")}
            onChange={(event) =>
              updateWindow({ process_names: keywords(event.target.value) })
            }
            placeholder="dwrg.exe，IdentityV.exe"
          />
        </label>
        <label>
          {t.classNames}
          <input
            value={settings.game_window.class_names.join("，")}
            onChange={(event) =>
              updateWindow({ class_names: keywords(event.target.value) })
            }
          />
        </label>
      </section>
      <section className="panel developer-panel">
        <h2>{t.engineCapture}</h2>
        <p>{t.engineCaptureHint}</p>
        <label>
          {t.heartbeat}
          <input
            type="number"
            min="0.5"
            step="0.5"
            value={settings.engine.heartbeat_interval_seconds}
            onChange={(event) =>
              updateEngine({
                heartbeat_interval_seconds: Number(event.target.value),
              })
            }
          />
        </label>
        <label>
          {t.captureBackend}
          <select
            value={settings.capture.backend}
            onChange={(event) => updateCapture({ backend: event.target.value })}
          >
            <option value="auto">{t.auto}</option>
            <option value="wgc">{t.windowCapture}</option>
            <option value="dxcam">DXcam</option>
            <option value="mss">MSS</option>
          </select>
        </label>
        <label>
          {t.idleFps}
          <input
            type="number"
            min="1"
            max="60"
            value={settings.capture.idle_fps}
            onChange={(event) =>
              updateCapture({ idle_fps: Number(event.target.value) })
            }
          />
        </label>
        <label className="check-label">
          <input
            type="checkbox"
            checked={settings.engine.memory_protection}
            onChange={(event) =>
              updateEngine({ memory_protection: event.target.checked })
            }
          />
          <span>
            {t.memoryProtection}
            <small>{t.memoryProtectionHint}</small>
          </span>
        </label>
      </section>
      <section className="panel developer-panel">
        <h2>{t.recognition}</h2>
        <label>
          {t.threshold}
          <input
            type="number"
            min="0"
            max="1"
            step="0.01"
            value={settings.vision.template_threshold}
            onChange={(event) =>
              updateVision({ template_threshold: Number(event.target.value) })
            }
          />
        </label>
        <label className="check-label">
          <input
            type="checkbox"
            checked={settings.debug.verbose_logs}
            onChange={(event) =>
              updateDebug({ verbose_logs: event.target.checked })
            }
          />
          {t.verbose}
        </label>
        <label className="check-label">
          <input
            type="checkbox"
            checked={settings.debug.save_debug_screenshots}
            onChange={(event) =>
              updateDebug({ save_debug_screenshots: event.target.checked })
            }
          />
          {t.screenshots}
        </label>
      </section>
      <div className="save-bar">
        <span>{t.autosave}</span>
      </div>
    </section>
  );
}
function About({ language }: { language: Language }) {
  const t = translations[language].about;
  return (
    <section className="panel about">
      <div className="about-intro">
        <span className="large-mark">V</span>
        <span className="eyebrow">IDENTITYV ASSISTANT</span>
        <h2>{t.name}</h2>
        <p>{t.description}</p>
      </div>
      <div className="about-details">
        <div>
          <span>{t.version}</span>
          <strong>0.1.0</strong>
        </div>
        <div>
          <span>{t.author}</span>
          <strong>WaltomAdaam2</strong>
        </div>
        <div>
          <span>{t.license}</span>
          <strong>AGPL-3.0-or-later</strong>
        </div>
        <div>
          <span>{t.stack}</span>
          <strong>React · TypeScript · Tauri</strong>
        </div>
      </div>
      <a
        className="repository-link"
        href="https://github.com/WaltomAdaam2/IdentityV-Assistant"
        target="_blank"
        rel="noreferrer"
      >
        GitHub · WaltomAdaam2/IdentityV-Assistant{" "}
        <span aria-hidden="true">↗</span>
      </a>
    </section>
  );
}

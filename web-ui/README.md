# Bee Admin Web UI

Bee Agent 系统的管理后台前端界面，基于 React + Vite + Tailwind CSS 构建。

## 功能特性

### Dashboard 首页
- 系统概览统计（活跃 Agent、运行中任务、请求量、错误告警）
- 最近活动动态
- 快速统计指标（响应时间、成功率、Token 使用量）

### Agent 管理
- Agent 列表展示（卡片式布局）
- 状态管理（运行中、空闲、错误）
- 技能和工具配置展示
- 性能指标（今日任务数、成功率）
- 快速操作（启动/停止、配置）

### 任务/Workflow 管理
- 看板视图（待执行、执行中、已完成、错误）
- 列表视图切换
- 执行进度可视化
- 步骤状态追踪
- 错误详情展示

### 监控日志
- 实时性能指标（请求数、成功率、延迟、错误数）
- 请求趋势图表
- 模型使用分布
- 错误日志筛选
- 审计日志追踪

## 技术栈

- **React 19** - UI 框架
- **Vite 8** - 构建工具
- **TypeScript** - 类型安全
- **Tailwind CSS 4** - 样式系统
- **Framer Motion** - 动画效果
- **Recharts** - 数据可视化
- **Lucide React** - 图标库
- **React Router** - 路由管理

## 快速开始

### 安装依赖

```bash
cd web-ui
npm install
```

### 开发模式

```bash
npm run dev
```

访问终端里显示的本地地址（默认是 `http://127.0.0.1:3000`，端口被占用时会自动递增）。

### bee-web、bee-admin 与 bee-gateway

- **bee-web**（默认 `http://127.0.0.1:8080`）提供对话、静态页与完整 **REST** `/api/*`（与历史行为一致）。
- **bee-admin**（默认 `http://127.0.0.1:8081`，环境变量 `BEE_ADMIN_PORT`）仅提供 **管理类** `/api/*`（metrics、tasks、assistants、审计等，无 `/api/chat` 与静态首页）。管理后台可只连 bee-admin。
- **bee-gateway**（默认 `ws://127.0.0.1:9000`，环境变量 `GATEWAY_BIND`）是 **WebSocket** 中枢，**没有**与 web-ui 当前页面对齐的一套 `/api` 路由，因此不能把 `/api` 代理整体改成 gateway 端口，否则请求会失败。

若希望浏览器只连 gateway 的 WebSocket，可在 `web-ui/.env` 里设置（并复制 `.env.example`）：

```bash
VITE_DEV_API_PROXY_TARGET=http://127.0.0.1:8080
VITE_DEV_WS_PROXY_TARGET=ws://127.0.0.1:9000
```

生产环境建议在 **反向代理**（Caddy / Nginx）上分别转发：`/api` → bee-web，`/ws` 或网关路径 → bee-gateway。

### 生产构建

```bash
npm run build
```

### 前端自检（推荐）

```bash
npm run verify
```

`verify` 会执行一次完整构建，能提前发现 Tailwind/Vite 配置回退、类型错误和构建问题。

## Tailwind v4 注意事项

本项目使用 Tailwind CSS v4 的 `@theme` / `@utility` 指令，必须启用 Vite 插件
`@tailwindcss/vite`（已在 `vite.config.ts` 配置）。

如果缺少这个插件，页面会出现样式错乱，并在构建时出现 `Unknown at rule: @theme`
等警告。

### 预览构建结果

```bash
npm run preview
```

## 项目结构

```
web-ui/
├── src/
│   ├── components/
│   │   ├── Layout.tsx          # 主布局（侧边栏 + 顶部导航）
│   │   └── ui/
│   │       ├── Badge.tsx       # 徽章组件
│   │       ├── Button.tsx      # 按钮组件
│   │       └── Card.tsx        # 卡片组件
│   ├── lib/
│   │   └── utils.ts            # 工具函数
│   ├── pages/
│   │   ├── Dashboard.tsx       # 首页仪表盘
│   │   ├── Agents.tsx          # Agent 管理页面
│   │   ├── Workflows.tsx       # 任务/Workflow 页面
│   │   └── Monitoring.tsx      # 监控日志页面
│   ├── App.tsx                 # 路由配置
│   ├── main.tsx                # 入口文件
│   └── index.css               # 全局样式
├── index.html
├── package.json
├── tsconfig.json
└── vite.config.ts
```

## API 集成

Vite 配置中已设置代理，将 API 请求转发到后端服务：

- `/api` → `http://127.0.0.1:8080`
- `/ws` → `ws://127.0.0.1:8080`

## 设计系统

### 颜色方案

- **Primary** - 蓝色系 (#3b82f6) - 主要操作和状态
- **Success** - 绿色系 (#22c55e) - 成功状态
- **Warning** - 黄色系 (#f59e0b) - 警告提示
- **Error** - 红色系 (#ef4444) - 错误状态
- **Surface** - 中性灰色系 - 背景和文本

### 组件规范

所有 UI 组件都支持：
- 深色模式自动适配
- 统一的圆角和阴影设计
- 平滑过渡动画
- 无障碍访问支持

## 后续开发

- [ ] 系统设置页面
- [ ] Agent 创建/编辑对话框
- [ ] Workflow 编辑器
- [ ] 用户权限管理
- [ ] 与后端 API 完整集成
- [ ] 单元测试覆盖

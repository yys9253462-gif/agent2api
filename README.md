# Agent2API（二次开发分支）

把多家 AI 桌面客户端的登录态转成一个本地 OpenAI 兼容网关。本仓库 fork 自 [aimod-cc/agent2api](https://github.com/aimod-cc/agent2api)（当前基线 3.0.8），在其上做二次开发。

> 本项目以非官方客户端形态复用账号登录态，可能违反上游服务条款，账号有被风控或封禁的风险。上游 LICENSE 在 MIT 之上追加了禁止商用、禁止二次分发牟利、禁止批量账号运营等限制，二次开发同样受其约束，详见 [LICENSE](./LICENSE)。

## 它做什么

客户端把 `base_url` 填成 `http://127.0.0.1:3065/v1`，网关按模型和优先级挑一个已登录账号，把请求改写成该平台的上游协议并转发，再把响应转回 OpenAI 格式。

- 对外协议：`/v1/chat/completions`、`/v1/responses`、`/v1/messages`（含 `count_tokens`）、`/v1/models`
- 账号：多账号优先级队列、429 自动换号与冷却、Token 自动续期、余额查询、签到与领取
- 出网：每账号独立代理（含订阅内核与代理池）、出站请求头按官方客户端对齐
- 管理：网关 Key 分权、管理员登录与人机验证、请求报表、定时任务
- 部署：Windows 桌面端（Tauri，托盘常驻），或 Docker（面板与网关同端口）

已接入的平台：WorkBuddy、小浣熊、CatPaw、AutoClaw、Qoder、Cline、Accio、ZCode、CodeArts、Trae、Loomy、KukuAI、MonkeyCode、Command Code、Antigravity、OfficeAce、OrcaTerm，另支持自定义提供商。各平台能力差异见上游 README。

## 目录结构

```
desktop-tauri/
├─ src-tauri/
│  ├─ src/                 桌面端外壳：窗口、托盘、登录窗口、自更新、与网关的桥接
│  └─ server/              网关本体（独立 crate，桌面端内嵌，Docker 版单独编译）
│     └─ src/
│        ├─ bin/agent2api-server.rs   Docker / 无界面入口
│        └─ server/
│           ├─ api/        HTTP 路由：/v1 转发（chat.rs、pipeline.rs）与面板 /api/*
│           ├─ core/
│           │  ├─ providers/<平台>/   各平台适配：登录、续期、模型目录、余额、签到
│           │  ├─ protocol/           OpenAI / Anthropic / Responses 与各上游格式互转
│           │  ├─ upstream/           转发主循环、换号重试、流式聚合、取消
│           │  ├─ account_store/      账号存储
│           │  ├─ routing.rs          模型路由与优先级队列
│           │  └─ proxies.rs 等       出网代理、订阅内核、代理池
│           ├─ config/     配置解析与持久化
│           └─ db/         SQLite（请求统计、日志、记录）
├─ ui/                     面板前端（原生 JS，桌面端与 Docker 共用）
├─ ui-islands/             局部 React 组件（Vite 构建）
├─ ui-kit/                 共享 UI 组件库
└─ i18n/                   界面多语言抽取与构建脚本
docs/                      多语言说明
scripts/                   发版脚本、models.dev 模型元数据同步
points-service.js 等       逆向参考：Loomy 客户端原始代码，不参与构建
Dockerfile / docker-compose.yml
agent.md                   上游的分支、版本号与发版约定
```

## 本地开发

环境要求：

- Rust >= 1.77 与 Tauri 2 工具链（Windows 需 WebView2）
- Node.js >= 18.17（只用于构建脚本和前端，运行时不依赖）

```bash
npm run tauri:install
```

```bash
npm run tauri:dev
```

```bash
npm run tauri:build
```

`tauri:dev` 使用 `src-tauri/tauri.dev.conf.json`，支持热重载。安装包输出到 `target/release/bundle/nsis/`（cargo 的 `target-dir` 在 `src-tauri/.cargo/config.toml` 里指到了项目根）。

只调网关不要桌面壳时，用 Docker 从源码构建：

```bash
docker compose up -d --build
```

打开 `http://localhost:3065`，首次注册管理员，在「网关 Key」页建 Key 后即可调用 `/v1`。数据在 `./data`。可用环境变量见 [.env.example](./.env.example) 与 [docker-compose.yml](./docker-compose.yml)，常用的有：

| 变量 | 说明 |
| --- | --- |
| `AGENT2API_ADMIN_USER` / `AGENT2API_ADMIN_PASSWORD` | 预置管理员，不填走面板注册 |
| `AGENT2API_PANEL_PORT` | 面板单独监听的端口，主端口只留 `/v1` |
| `AGENT2API_HOST` / `AGENT2API_PROXY_PORT` | 监听地址与端口，默认 `0.0.0.0:3065` |
| `AGENT2API_ALLOW_NO_KEY` | 置 `1` 时未配 Key 也放行 `/v1`，仅限内网 |
| `AGENT2API_CAPTCHA_ENABLED` | 登录页人机验证，默认 `1` |
| `AGENT2API_SELF_UPDATE` | 面板内一键更新，默认 `1` |
| `WORKBUDDY_UPDATE_REPO` / `WORKBUDDY_UPDATE_API_BASE` | 更新检查源，默认跟上游 `aimod-cc/agent2api`；二开发版时应改成本仓库 |

联通性检查：

```bash
curl http://127.0.0.1:3065/health
```

```bash
curl http://127.0.0.1:3065/v1/models -H "Authorization: Bearer <网关 Key>"
```

## 常见改动入口

- 新增一个平台：在 `core/providers/` 下仿照现有目录（如 `cline/`）实现登录、续期与请求改写，再在 `providers/mod.rs`、`router.rs`、`catalog.rs` 注册；面板侧在 `ui/preset-providers.js` 与 `ui/providers.js` 加卡片。
- 改协议转换：`core/protocol/`，入站在 `anthropic.rs`、`responses.rs`，出站在各 `*_outbound.rs`。
- 改换号与重试：`core/upstream/provider_loop/`、`rotate.rs`，以及 `core/degrade.rs`、`core/limiter.rs`。
- 加面板接口：`api/` 下新建文件并在 `api/mod.rs` 挂路由。
- 改界面文案：改完中文源后用 `desktop-tauri/i18n/` 的脚本重新抽取各语言。

## 与上游同步

`.github/workflows/sync.yml` 每 6 小时执行一次 `gh repo sync` 拉上游 `main`。二次开发的改动如果直接提交到 `main`，同步可能失败或产生冲突，建议：

- 自己的改动放在独立分支，定期从 `main` 合并上游更新；或者
- 关掉该工作流，改为手动合并上游。

发版约定（五处版本号、tag 触发构建、Release 脚本）沿用 [agent.md](./agent.md)。如果要发自己的版本，记得同时改 `package.json` 的 `repository`、Docker 镜像名和更新检查源，避免用户被引导到上游版本。

## 凭证安全

账号凭证以明文存放在本机配置目录（默认 `~/.agent2api/`，Docker 版在 `/data`），导出文件同样是明文。不要提交到仓库或分享出去。

## 许可

MIT 加附加使用声明，两者冲突时以更严格的一方为准，完整内容见 [LICENSE](./LICENSE)。原作者 [aimod-cc](https://github.com/aimod-cc)。

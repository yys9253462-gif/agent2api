# Agent2API · 多提供商本地网关

**简体中文** | [English](./README.en.md)

把多家 AI 桌面客户端的登录态包装成本地 **OpenAI 兼容 API 网关**，统一暴露一个 `base_url`，附带多提供商账号管理、模型管理（启停 / 删除 / 映射）、出站指纹脱敏、出网代理与请求报表，并提供一个开箱即用的 Tauri 桌面端。任何支持自定义 `base_url` 的 OpenAI 客户端都能以 `http://127.0.0.1:3065/v1` 为端点调用这几家的模型额度——不需要 API Key，不需要改客户端源码。

各平台的反代能力一览（✓ 支持 · ✗ 不支持 · — 无此概念或不适用）：

| 平台 | LLM 请求 | Token 自动续期 | 模型列表（远程刷新） | 余额查询 | 签到 | 领取类 |
| --- | :--: | :--: | :--: | :--: | :--: | :--: |
| WorkBuddy 国内版 | ✓ | ✓ | ✓ 远程 + 静态兜底 | ✓ | ✓ 每日签到 | — |
| WorkBuddy 国际版 | ✓ | ✓ | ✓ 远程 + 静态兜底 | ✓ | ✗ 无签到活动 | — |
| 小浣熊 | ✓ | ✓ | ✓ 远程 + 静态兜底 | ✓ | ✓ 桌面登录积分 | — |
| CatPaw | ✓ | ✗ 无刷新机制 | ✓ 远程 + 静态兜底 | ✓ | ✗ | — |
| AutoClaw（国内版 / 国际版） | ✓ | ✓ | ✓ 远程 + 静态兜底 | ✓ | ✓ 每日签到 | — |
| Qoder | ✓ | ✓ | ✓ 远程（按地区）+ 静态兜底 | ✓ | ✓ 仅中国版 | — |
| Cline（Free / Pass） | ✓ | ✓ | ✓ 远程 + 静态兜底 | ✓ | — | — |
| Accio（国际版 / 国内版） | ✓ | ✓ | ✓ 远程 + 静态兜底 | ✓ 用量百分比 | — | — |
| ZCode（国内版 / 国际版） | ✓ | ✗ | ✗ 静态表 | ✓ 套餐余额 | — | ✓ 限时套餐（手动） |
| CodeArts | ✓ | ✓ 一次性轮换 | ✓ 远程（三源合并） | ✓ 两份账 | — | ✓ 每日福利（手动） |
| Trae | ✓ | ✓ 一次一换 | ✓ 仅远程 | ✓ 两份账 | — | — |
| Loomy（讯飞） | ✓ | ✗ 无续期接口 | ✓ 仅远程 | ✓ 两份积分账 | ✓ 每日赠送积分刷新 | — |
| 自定义提供商 | ✓ Chat 透传 / Responses / Anthropic | — | ✓ 手动登记 + 服务端拉取 | — | — | — |

三条对话协议入口（`/v1/chat/completions`、`/v1/responses`、`/v1/messages`，另含 `/v1/messages/count_tokens`）与 `/v1/models` 对所有平台一视同仁，差异只在各家上游能不能做到表里那些事；模型映射、全局优先级队列、429 降级、出网代理、出站指纹脱敏与请求报表同样对全平台通用。

> **本项目仅供学习与交流使用。** 它通过本地反向代理复用你自己账号的登录态，这种「以非官方客户端形态转发」的方式可能不符合上游服务的用户协议，使用风险（含账号被风控、封禁）由使用者自行承担；禁止用于商业用途或绕过计费。详见[使用声明](#使用声明)与 [LICENSE](./LICENSE)。
>
> 本项目是个人用途的本地代理工具，与腾讯（WorkBuddy）、美团（CatPaw）、商汤（小浣熊）、智谱（AutoClaw/autoglm）、阿里巴巴（Qoder / Accio）、华为云（CodeArts）、字节跳动（Trae）、科大讯飞（Loomy）、Cline 及其官方产品均无关；所有接口形态来自对各家桌面端通信的观察，上游随时可能调整。

---

## 目录

- [快速开始](#快速开始)
- [Docker 部署](#docker-部署)
- [界面预览](#界面预览)
- [项目结构](#项目结构)
- [开发与构建](#开发与构建)
- [使用声明](#使用声明)
- [许可](#许可)

---

## 快速开始

从 Releases 下载安装包（NSIS，简体中文，默认装到 `C:\Program Files\Agent2API`，安装时需要管理员授权），安装后启动即可，**无需安装 Node 或任何其它运行时**。

1. 首次启动即在应用进程内启动本机网关（端口 3065）并打开主窗口；若检测到旧版本的数据目录或数据文件，会弹窗提示迁移，按指引操作即可。
2. 点「账号」页的「添加账号」，选提供商（WorkBuddy / 小浣熊 / CatPaw / AutoClaw 国内版 / AutoClaw 国际版 / Qoder / Cline / Accio 国际版 / Accio 国内版 / ZCode 国内版 / ZCode 国际版 / CodeArts / Trae / Loomy），再按该家支持的方式完成登录或填写凭证：网页登录、手机验证码、粘贴凭证，或导入本机桌面端登录态（导入不落 token，客户端重新登录后网关自动跟上；CodeArts 与 Trae 只有网页登录与粘贴凭证两种，Loomy 只有手机验证码与粘贴 session 两种）。
3. 把 OpenAI 客户端的 `base_url` 填成 `http://127.0.0.1:3065/v1`，`api_key` 随便填（例如 `sk-local`，未启用鉴权时服务端不校验）。

关闭窗口默认只是最小化到托盘，网关继续在后台转发；要彻底退出请在托盘图标上右键选「退出」。

账号排在同一条**全局队列**里，按优先级从小到大逐个尝试，跳过已禁用、余额不足（账号设置里配置了「跳过」且低于阈值）、不提供该模型或对该模型处于限额冷却期的账号；某家对某模型触发 429 时降级到下一个候选，全部不可用才把最后一个真实错误透传出来。

### 验证

网关起来后，用 curl 确认联通性（`model` 填 `GET /v1/models` 里任一实际存在的名字）：

```bash
curl http://127.0.0.1:3065/health
curl http://127.0.0.1:3065/v1/models

curl http://127.0.0.1:3065/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{"model":"deepseek-v4.1-flash","messages":[{"role":"user","content":"你好"}],"stream":true}'
```

客户端接入示例（Python SDK）：

```python
from openai import OpenAI

client = OpenAI(base_url="http://127.0.0.1:3065/v1", api_key="sk-local")
resp = client.chat.completions.create(
    model="deepseek-v4.1-flash",     # 名单里有哪家就是哪家，见 GET /v1/models
    messages=[{"role": "user", "content": "你好"}],
)
print(resp.choices[0].message.content)
```

**浏览器里的页面**（自建 Web UI、单文件前端应用等）用 `fetch` 直连这个端点时，会因为跨源预检被拒而报「无法连接 API」：网关面默认**不应答 CORS**，预检请求（OPTIONS）会落到 API Key 校验上得到 401（跨源预检按规范不携带 `Authorization` 头），请求根本发不出去。两种解法：① 在设置页「安全 → 网关跨域访问」里打开它，网关随即按面板的同一口径应答（预检放行、响应带 `Access-Control-Allow-*`，来源 `*`，立即生效）—— 注意网关是真正转发上游、消耗额度的那一面，开着 `*` 又没配 API Key 时任何网页都能借本机网关打上游，建议同时配置「网关 Key」；② 让页面与网关同源 —— 用一个本地静态服务同时托管页面并把 `/v1` 反代到 `127.0.0.1:3065`，这样连跨域都不存在，不需要放开任何东西。

### 局域网访问

默认网关只监听 `127.0.0.1`，只有本机能用。在「设置 → 通用 → 局域网访问」开启后，网关改听所有网卡，同一局域网内的设备把 API 地址指向本机 IP（界面上会给出完整地址，如 `http://192.168.1.5:3065/v1`）即可共用这套账号。出于安全考虑，开启前必须注册一个面板管理员：管理接口从此要求管理员会话或网关 Key，一把启用的 Key 都没有时转发接口也会拒绝服务（开启流程会自动补一把「默认」Key）。还可以选择把网页管理面板一并开放给局域网（其他设备的浏览器打开本机 IP 即可管理，需管理员登录）；默认不开放，桌面端的面板只由本程序自己出。改动随应用重启生效。

---

## Docker 部署

```bash
docker run -d --name agent2api --restart unless-stopped \
  -p 3065:3065 -v ./data:/data \
  aimodcc/agent2api:latest
```

浏览器打开 `http://<主机>:3065`，首次进入会引导**注册管理员账号**（后续登录用它）；登录后在「网关 Key」页创建一把 API Key 给客户端用 —— `http://<主机>:3065/v1` 即 OpenAI 兼容端点，未建 Key 前拒绝转发，建第一把后自动恢复。所有状态（SQLite 库 / 配置 / 日志）都落在 `./data` 一个卷里。

compose 用户（`docker-compose.yml` 全文就这么多；amd64 / arm64 都有镜像）：

```yaml
services:
  agent2api:
    image: aimodcc/agent2api:latest
    container_name: agent2api
    restart: unless-stopped
    ports:
      - "3065:3065"
    volumes:
      - ./data:/data
```

环境变量（都可选，不需要预置任何东西）：

| 变量 | 说明 |
| --- | --- |
| `AGENT2API_ADMIN_USER` + `AGENT2API_ADMIN_PASSWORD` | 预置管理员账号密码（密码明文填，启动时自动转哈希）；不填走面板注册 |
| `AGENT2API_PANEL_PORT` | 面板分端口：设后面板（界面 + `/api/*`）单独监听该端口，公网只映射主端口即可把管理面留在内网（面板端口绑回环，写 `127.0.0.1:3066:3066`） |
| `AGENT2API_HOST` / `AGENT2API_PROXY_PORT` | 监听地址（默认 `0.0.0.0`）/ 端口（默认 `3065`） |
| `AGENT2API_ALLOW_NO_KEY` | 置 `1` 关闭 fail-closed（未配 Key 也放行 `/v1`，仅限纯内网） |
| `AGENT2API_CAPTCHA_ENABLED` | 登录页人机验证组件环境变量：默认为 `1` 开启，`0` 为关闭 |

从源码构建：克隆本仓库后 `docker compose up -d --build`（镜像里只有网关与面板，不含 Rust 工具链）。

**网页端功能差异**（都源于「没有本机桌面客户端」）：网页登录（WorkBuddy / Qoder / Cline）、手机验证码、粘贴凭证完全可用；AutoClaw / CatPaw / Accio / CodeArts / Trae 网页登录的回调打本机端口，远程面板请改用粘贴凭证；小浣熊网页登录与「导入本机桌面端登录态」不可用（用填写凭证；Loomy / CodeArts / Trae 本来也没有桌面端登录态可导入）。

---

---

## 界面预览

### 账号

所有提供商的账号排在同一条**全局队列**里（左起第二列就是优先级），可逐条启停；限额行显示按模型维度的冷却状态与恢复时间，有效期由「凭据维护」后台任务刷新，余额按**每账号自己的间隔**自动查询（账号设置「查询设置」里配，默认开启、每 1 分钟）；余额不足默认按阈值 1 跳过，也可改禁用或不处理。

![账号管理页：全局队列、按模型限额冷却、有效期与余额](./assets/screenshots/accounts.png)

添加账号时先选提供商，再按该家支持的方式登录。同一家可同时保存多个版本的账号（例如 WorkBuddy 的国内版 / 国际版），转发时按模型自动选路：

![添加账号：选择提供商与版本，然后走网页登录](./assets/screenshots/add-account.png)

### 报表

统计概览给出总请求数、成功率、Token 总量与 Top 模型，右侧按账号和提供商分别排序；再往下是模型用量与提供商用量两张环形图，以及固定 365 天的活跃热力图：

![报表概览：统计卡片、Top 账号 / 提供商、模型与提供商用量环形图](./assets/screenshots/report-overview.png)

再往下是趋势区：近 24 小时的**缓存命中率**（左轴，折线）与 **Token 消耗**（右轴，面积）叠在同一张图上，便于判断命中率下滑是流量结构变化还是缓存失效；底部是按天 Token 柱状图，范围随顶部时间窗切换：

![报表趋势：缓存命中率与 Token 消耗双轴趋势、按天 Token 柱状图](./assets/screenshots/report-trends.png)

### 定时任务

后台任务在「定时任务」页统一管理：开关、执行间隔、上次执行结果与下次触发时间都在这里，也可以绕过间隔手动「立即执行」一次。任务清单本身保存在 `~/.agent2api/config.json` 的 `scheduledTasks` 字段，改动立即生效，不需要重启程序。

![定时任务页：自动签到、凭证维护、模型目录刷新等后台任务的开关与间隔](./assets/screenshots/scheduled-tasks.png)

---

## 项目结构

网关与桌面端都在 `desktop-tauri/`：后端是 `src-tauri/` 下的 Rust 进程内 HTTP 服务器，前端是 `ui/` 下的原生 HTML/CSS/JS。

```
agent2api/
├─ desktop-tauri/
│  ├─ src-tauri/
│  │  ├─ server/                 网关本体 crate（agent2api-server，独立编译：
│  │  │                          桌面端与 headless 二进制共用；src/server/ 下
│  │  │                          的实现与 bin/agent2api-server.rs 无 GUI 依赖）
│  │  │  ├─ mod.rs               服务组装：ServerState、启动、停机、启动迁移
│  │  │  ├─ http.rs              路由表、CORS、API Key 中间件、body 限制、headless 静态托管
│  │  │  ├─ config.rs / logging.rs / logs_store.rs / errors.rs
│  │  │  ├─ config_migration.rs  1.x 配置目录迁移（~/.workbuddy-proxy → ~/.agent2api，启动第一步）
│  │  │  ├─ request_stats.rs + request_stats/   统计的时钟窗口、写入、聚合与裁剪
│  │  │  ├─ core/
│  │  │  │  ├─ providers/        ★ 多提供商层（本改造的核心）
│  │  │  │  │  ├─ mod.rs        ProviderKind（内置各家；Cline 按额度池拆两池、Accio 与
│  │  │  │  │  │                ZCode 按地区拆两地）+ PROVIDERS 注册表 + id 互查
│  │  │  │  │  ├─ adapter.rs    ProviderAdapter trait + adapter_for + implemented_kinds
│  │  │  │  │  ├─ router.rs     模型名 → 候选 provider 集合（聚合目录）
│  │  │  │  │  ├─ catalog.rs    聚合模型目录（清单合并 / 同名去重 / 可用性判定）
│  │  │  │  │  ├─ catalog_cache.rs  各家远程清单的持久化缓存（重启后读回，不再回落到内置清单）
│  │  │  │  │  ├─ refresh_flight.rs  凭证刷新的单飞去重
│  │  │  │  │  ├─ workbuddy.rs  WorkBuddy 适配器（头集合 / system 注入 / 6004 / 11128）
│  │  │  │  │  ├─ raccoon/      小浣熊：mod / models / credentials / jwt / oauth / balance
│  │  │  │  │  ├─ catpaw/       CatPaw：adapter（is_stateful）/ conversation（轮次状态机）/
│  │  │  │  │  │                turn_executor / prepare / decision（轮次判定）/ fingerprint /
│  │  │  │  │  │                registry/（会话注册表：表与句柄 / 账号身份 / 作废）/
│  │  │  │  │  │                messages / blocks / tools / openai（翻译层）/
│  │  │  │  │  │                upstream_http / image_compress / models / credentials / balance
│  │  │  │  │  ├─ autoclaw/     智谱 autoglm（国内版 + 国际版两家）：region（两地域名与身份）/
│  │  │  │  │  │                adapter / credentials / refresh / crypto / models /
│  │  │  │  │  │                balance / login（手机验证码，仅国内版）/
│  │  │  │  │  │                oauth（Zai / Google 网页登录，仅国际版）/ checkin（每日签到任务）
│  │  │  │  │  ├─ qoder/        Qoder：adapter / endpoints（两站地址）/ oauth（设备授权）/
│  │  │  │  │  │                auth / cosy（COSY 签名与体编码）/ protocol（信封解码）/
│  │  │  │  │  │                chat（会话式转发）/ stream / machine（PKCE 与机器标识）/
│  │  │  │  │  │                credentials / refresh / models / balance
│  │  │  │  │  ├─ cline/        Cline：adapter（Bearer + 产品面头）/ credentials（workos: 前缀
│  │  │  │  │  │                + 桌面端登录态 + 姓名解析）/ login（WorkOS 设备授权）/
│  │  │  │  │  │                refresh（单飞续期）/ models（两额度池 + 默认映射种子）/
│  │  │  │  │  │                balance（credit 余额，微 credit ÷1e6）
│  │  │  │  │  ├─ accio/        Accio（国际版 + 国内版两家）：endpoints（两地区与端点）/
│  │  │  │  │  │                credentials / auth / refresh（单飞续期）/
│  │  │  │  │  │                oauth（PKCE 网页登录 + loopback 回调）/
│  │  │  │  │  │                models（静态兜底 + /api/llm/config/v2）/
│  │  │  │  │  │                protocol（OpenAI ↔ ADK 的 Gemini 风格信封）/
│  │  │  │  │  │                chat（会话式转发）/ stream（ADK SSE 解包）/ balance
│  │  │  │  │  ├─ codearts/     CodeArts（华为云码道）：signer（华为云 SDK-HMAC-SHA256，
│  │  │  │  │  │                与参考实现逐字节对账）/ credentials / dpop（ES256 DPoP proof）/
│  │  │  │  │  │                oauth（PKCE 网页登录 + loopback 回调）/ refresh（单飞续期）/
│  │  │  │  │  │                session（chat-session 心跳与每账号并发准入）/ chat（会话式转发）/
│  │  │  │  │  │                stream_fault（HTTP 200 的流内错误信封）/ redact（错误体脱敏）/
│  │  │  │  │  │                models（agent / builtin / 福利网关三源合并）/
│  │  │  │  │  │                balance（订阅统计 + 福利网关两份账）/
│  │  │  │  │  │                welfare（每日福利领取：幂等键先落盘、回读二次确认）
│  │  │  │  │  ├─ trae/         Trae（字节 AI IDE SOLO 通道）：credentials / device（设备密钥对）/
│  │  │  │  │                   login + oauth（PKCE 网页登录 + 换证候选）/ callback_server
│  │  │  │  │                   （本机随机端口回调与噪音过滤）/ refresh（单飞续期）/
│  │  │  │  │                   payload（SOLO 信封白名单重建）/ headers（SOLO 头集合）/
│  │  │  │  │                   stream（SSE→chunk 翻译）/ forward（有状态转发）/
│  │  │  │  │                   errors（错误分类与死配置名单）/ models（get_detail_param 目录）/
│  │  │  │  │                   usage（权益包 + 套餐 quota 两份账）/ profile（身份解析）
│  │  │  │  │  └─ loomy/        Loomy（讯飞）：login（手机验证码）/ credentials（session 14 天、无续期接口）/
│  │  │  │  │                   sign（复刻客户端 HMAC-SHA1 签名头）/ endpoints / client（集成网关）/
│  │  │  │  │                   models（/api/v1/models 远程目录，上游无内置兜底清单）/
│  │  │  │  │                   balance（永久积分 + 每日赠送两份账）/ checkin（每日首次登录刷新赠送积分）
│  │  │  │  ├─ upstream/        转发编排：全局账号队列循环（provider_loop）+ 发送体处理
│  │  │  │  │                    （payload）+ SSE 透传/聚合 + usage 旁路提取
│  │  │  │  ├─ account_store/   账号存储（全局优先级、限额冷却、各家添加与导入）
│  │  │  │  ├─ models/          模型目录底层（workbuddy 内置清单 + /v3/config 刷新）
│  │  │  │  ├─ model_rules.rs   模型管理规则（禁用 / 隐藏 / 映射 alias）
│  │  │  │  ├─ api_keys.rs      网关 Key 列表（多把 Key，任一启用即通过）
│  │  │  │  ├─ auth.rs / auth_http.rs / login.rs   会话、出网传输、无头登录
│  │  │  │  │                    （login/ 下是各家特有的登录流程：catpaw 的
│  │  │  │  │                    loopback 回调、qoder 的设备授权）
│  │  │  │  ├─ routing.rs / billing/   账号选路（全局优先级 + 限额冷却）/ 积分签到运营
│  │  │  │  ├─ proxies.rs / clash.rs / egress.rs   出网代理与按出口缓存 Client
│  │  │  │  ├─ sanitize.rs      出站指纹脱敏（硬编码规则集：表头剥离 + 模板句最小改写）
│  │  │  │  ├─ prompt.rs        网关自有系统提示词（透传 / 替换 / 追加三模式）
│  │  │  │  ├─ degrade.rs       内容拦截降级状态机（撞审核误报后到次日 00:00 用中性提示词）
│  │  │  │  ├─ credential_maintenance.rs  已过期 / 临期凭证的批量刷新
│  │  │  │  ├─ usage_query.rs     余额 / 积分查询（跨账号并发 + 定时那一轮的快照）
│  │  │  │  ├─ scheduled_tasks.rs  间隔型定时任务注册表与调度循环（开关 / 间隔 /
│  │  │  │  │                      上次结果；配置在 config.json 的 scheduledTasks）
│  │  │  │  └─ account_transfer.rs + account_transfer/ / auto_checkin.rs / update/
│  │  │  │                       导入导出（含身份归一）/ 定时签到 / 软件更新
│  │  │  └─ api/                 各路由 handler（health/session/accounts/accounts_usage/
│  │  │                          chat/models/keys/model_manage/stats/logs/billing/
│  │  │                          sanitize/prompt/auto-checkin/scheduled-tasks/update/…）
│  │  ├─ lib.rs                 应用入口（配置目录迁移 → 设置 → 托盘 → 主窗口 → 启动后端）
│  │  ├─ backend.rs              进程内服务器生命周期
│  │  ├─ legacy_install.rs       旧「当前用户」安装的清理（目录 / 快捷方式 / 卸载项 / 自启；仅 release）
│  │  ├─ gateway.rs              壳侧访问管理 API 的 HTTP 客户端
│  │  ├─ login.rs / commands.rs  登录窗口与轮询、暴露给前端的 invoke 命令
│  │  ├─ login_profile.rs        每次网页登录独享一个临时 WebView2 数据目录（登完即删）
│  │  ├─ bridge.rs               注入 window.workbuddyDesktop 的桥接脚本
│  │  └─ update.rs / settings.rs / state.rs / tray.rs
│  ├─ ui/                        前端（原生 HTML/CSS/JS，无框架）
│  └─ src-tauri/tauri.conf.json  打包配置（NSIS）
├─ build/make-icon.mjs           生成应用图标源图
├─ assets/screenshots/           README 配图（界面截图）
├─ Dockerfile / .dockerignore    headless 镜像（多阶段构建，只含网关与面板）
├─ docker-compose.yml / .env.example   部署（单容器：面板 + 网关同端口）
└─ package.json                  构建脚本入口（tauri:dev / tauri:build / build:icon）
```

---

## 开发与构建

### 环境要求

- Rust >= 1.77 与 Tauri 2 工具链（编译桌面端本体；Windows 上还需 WebView2 运行时）
- Node.js >= 18.17（仅用来执行 `npm run tauri:*` 与 `build/make-icon.mjs` 这些前端构建脚本，桌面端运行时不依赖 Node，也不会打包任何 Node 产物）

### 常用脚本

```bash
npm run tauri:install      # 安装桌面端依赖（等价 npm --prefix desktop-tauri install）
npm run tauri:dev          # 开发模式调起桌面端（自动热重载）
npm run tauri:build        # 构建桌面端安装包

npm run build:icon         # 生成图标源图（改图标设计后执行，再跑 tauri icon）
```

根项目本身没有运行期依赖，`package.json` 只提供上面这些快捷脚本入口。打包产物为 `target/release/bundle/nsis/Agent2API_<版本>_x64-setup.exe`（当前约 3.0 MB；`src-tauri/.cargo/config.toml` 把 cargo 的 `target-dir` 指到了项目根的 `target/`）。

---

## 使用声明

### 仅供学习与交流

本项目是一个用于学习 HTTP 反向代理、SSE 流式透传、多上游协议适配与桌面端打包（Tauri）等技术主题的实践项目，**仅供个人学习与研究使用**。它不是官方产品，与腾讯公司及 WorkBuddy / CodeBuddy、美团及 CatPaw、商汤及小浣熊、智谱及 AutoClaw / autoglm、阿里巴巴及 Qoder / Accio、华为云及 CodeArts、字节跳动及 Trae、科大讯飞及 Loomy 均无任何关联，未获得其授权、认可或赞助。

### 关于反向代理行为

本项目实现的是本地反向代理：在你自己的机器上复用你自己账号的登录态，把请求转发到官方上游网关。它不破解、不绕过任何付费或权限校验，使用的额度始终来自你自己账号本就拥有的配额。但需要明确的是，这种「以非官方客户端形态复用登录态」的转发方式，**可能不符合上游服务的用户协议或使用条款**；是否使用、由此产生的一切后果（包括但不限于账号被限流、被风控、被冻结或封禁），均由使用者自行承担。

### 禁止用途

禁止将本项目用于任何商业用途、二次分发牟利、批量账号运营、绕过上游计费或配额限制、或其他违反当地法律法规的活动。如需在生产环境或商业场景中调用相关模型服务，请使用官方渠道与官方 API。

### 凭证与数据风险

本项目会把账号凭证（`accessToken` / `refreshToken` 等）以**明文**形式保存在本机配置目录（默认 `~/.agent2api/`）中，导出功能生成的文件同样包含明文凭证。请自行妥善保管，切勿提交到公开仓库、上传到网盘或分享给他人。因凭证泄露造成的损失由使用者自行承担。

### 无担保与权利通知

本项目按「现状」提供，作者不对其可用性、稳定性、安全性或对特定用途的适用性作任何承诺。上游接口随时可能变更，本项目可能随时失效而不再维护。完整条款见 [LICENSE](./LICENSE)。本项目中的接口形态、协议字段等信息来自对各官方客户端公开网络通信的观察与整理，相关商标与服务的权利归其各自所有者；若权利方认为本项目存在不当之处，请联系作者，将及时调整或删除。

---

## 许可

本项目基于 [MIT License](./LICENSE)，可自由使用、修改与分发，须保留版权声明。

需要留意的是：LICENSE 正文之后附有一份**使用声明**，其中第 3 条在 MIT 之上**追加了限制**（禁止商业用途、禁止二次分发牟利、禁止批量账号运营）。因此本项目**不是**纯粹的 MIT 项目——**MIT 条款与使用声明共同构成完整的授权与使用约定**，两者对同一行为给出不同结论时以更严格的一方为准。这也是 `Cargo.toml` 用 `license-file` 指向 LICENSE、而不声明 SPDX `"MIT"` 的原因。

---

## Star History

<a href="https://star-history.com/#aimod-cc/agent2api&Date">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=aimod-cc/agent2api&type=Date&theme=dark" />
    <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=aimod-cc/agent2api&type=Date" />
    <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=aimod-cc/agent2api&type=Date" />
  </picture>
</a>


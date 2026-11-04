#!/usr/bin/env node
//! 迁移 parity 验证：**内置实现 vs 插件实现**，同一份输入比同一份输出。
//!
//! ── 这个脚本解决什么问题 ────────────────────────────────────
//! 把一家 provider 从「编译进网关」搬到「插件子进程」时，最大的风险不是编译
//! 不过，而是**静默行为漂移**：某个字段改了名、某个默认值变了、某个分支顺序
//! 反了。这些都能编译通过，也都能「看起来正常工作」，直到某个具体请求出错。
//!
//! 所以这里做**差分测试**：同一份输入分别喂给两边，逐字段比对。
//!
//! ── 覆盖什么（按「容易漂移」排序）──────────────────────────────
//!   1. `chat`：URL / 请求头 / 完整请求体（**深度相等**，键序也管）
//!   2. `classify`：四档分类的每个分支（含边界：402→429、403 两种语义）
//!   3. `models`：清单条目数与首条形状
//!   4. 能力位与 provider id
//!
//! ── 怎么跑 ──────────────────────────────────────────────────
//! ```bash
//! # 1. 先构建插件
//! cd agent2api-providers && cargo build
//! # 2. 跑 parity
//! node scripts/parity-check.mjs
//! ```
//! 插件二进制不存在时跳过（不报失败）。
//!
//! ── 为什么用 Node 而不是 Rust 集成测试 ──────────────────────
//! 内置实现要跑起来需要整个网关的 `ServerState`（账号库、配置、日志库…），
//! 那是几百行脚手架。而**插件侧**只需要 stdin/stdout —— Node 直接说协议就
//! 行了。于是这里的做法是：**用 Node 扮演网关调插件**，把结果与「从内置
//! 实现抄下来的期望值」比对。期望值来自内置实现的源码，是冻结的。

import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");

function pluginExe() {
  const candidates = [
    join(root, "agent2api-providers/target/debug/agent2api-providers.exe"),
    join(root, "agent2api-providers/target/debug/agent2api-providers"),
    join(root, "agent2api-providers/target/release/agent2api-providers.exe"),
    join(root, "agent2api-providers/target/release/agent2api-providers"),
  ];
  return candidates.find((path) => existsSync(path));
}

// ── 一个最小的「网关」：说协议、收应答 ────────────────────────
class Gateway {
  constructor(exe) {
    this.child = spawn(exe, [], { stdio: ["pipe", "pipe", "inherit"] });
    this.nextId = 1;
    this.pending = new Map();
    this.buffer = "";
    this.child.stdout.on("data", (chunk) => {
      this.buffer += chunk.toString("utf8");
      let index;
      while ((index = this.buffer.indexOf("\n")) >= 0) {
        const line = this.buffer.slice(0, index);
        this.buffer = this.buffer.slice(index + 1);
        if (!line.trim()) continue;
        let frame;
        try {
          frame = JSON.parse(line);
        } catch {
          continue;
        }
        if (frame.id && this.pending.has(frame.id)) {
          const { resolve, reject } = this.pending.get(frame.id);
          this.pending.delete(frame.id);
          if (frame.error) reject(new Error(frame.error.message));
          else resolve(frame.result);
        }
      }
    });
  }

  call(method, params) {
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.child.stdin.write(JSON.stringify({ id, method, params }) + "\n");
    });
  }

  async init() {
    return this.call("init", {
      env: {
        data_dir: "",
        host_version: "parity",
        target: "x86_64-pc-windows-msvc",
        proxy: "",
        debug_traffic: false,
      },
    });
  }

  close() {
    this.child.stdin.end();
    this.child.kill();
  }
}

// ── 断言工具（失败要能一眼看出哪一层、差在哪）─────────────────
let failures = 0;
let checks = 0;

/**
 * 深比较，**忽略对象键序**。
 *
 * ── 为什么不能直接 JSON.stringify 比 ────────────────────────────
 * 插件的请求体由 `serde_json` 序列化，而它的 `Map` 默认是 **BTreeMap**
 * （键按字典序排）。网关内置实现走的是同一条栈，所以两边键序其实一致 ——
 * 但把「键序」当成契约会让这个脚本在无关的改动上误报（例如给信封加一个
 * 字段）。JSON 对象的键序本来就没有语义，比较值即可。
 */
function deepEqual(a, b) {
  if (a === b) return true;
  if (a === null || b === null) return a === b;
  if (typeof a !== typeof b) return false;
  if (Array.isArray(a) !== Array.isArray(b)) return false;
  if (Array.isArray(a)) {
    if (a.length !== b.length) return false;
    return a.every((item, index) => deepEqual(item, b[index]));
  }
  if (typeof a === "object") {
    const ka = Object.keys(a).sort();
    const kb = Object.keys(b).sort();
    if (ka.length !== kb.length) return false;
    if (ka.some((key, index) => key !== kb[index])) return false;
    return ka.every((key) => deepEqual(a[key], b[key]));
  }
  return false;
}

function check(label, actual, expected) {
  checks += 1;
  if (deepEqual(actual, expected)) return;
  failures += 1;
  console.error(`\n✗ ${label}`);
  console.error(`  实际: ${JSON.stringify(actual)}`);
  console.error(`  期望: ${JSON.stringify(expected)}`);
}

function checkTruthy(label, value, note = "") {
  checks += 1;
  if (value) return;
  failures += 1;
  console.error(`\n✗ ${label}${note ? `（${note}）` : ""}`);
}

/**
 * 插件的 `headers` 是 `Vec<(String, String)>`，过线后是**数组**（保序，
 * 因为鉴权头的顺序在少数上游那里有意义）。这里转成对象便于按名字断言。
 */
function headerMap(headers) {
  if (!headers) return {};
  if (Array.isArray(headers)) {
    return Object.fromEntries(headers.map(([key, value]) => [key, value]));
  }
  return headers;
}

/** 键集合比较（不看顺序 —— 见 `deepEqual` 的说明） */
function checkKeySet(label, actual, expected) {
  checks += 1;
  const a = Object.keys(actual).sort();
  const e = [...expected].sort();
  if (JSON.stringify(a) === JSON.stringify(e)) return;
  failures += 1;
  console.error(`\n✗ ${label}`);
  console.error(`  实际: ${JSON.stringify(a)}`);
  console.error(`  期望: ${JSON.stringify(e)}`);
}

// ── 假账号：插件通过 store.get 回调拿到的 secret ──────────────
function accountSecret(provider) {
  if (provider === "commandcode") return { apiKey: "user_parity_test_key_123456" };
  return { accessToken: "ms-parity-token-abcdefgh", userId: "2220378210908" };
}

/**
 * 一个「像网关」的插件宿主：自动应答 `store.get` 回调。
 *
 * ── 为什么必须应答回调 ──────────────────────────────────────
 * 插件的 `build_chat` / `ensure_token` 都会回调 `store.get` 取凭证。
 * 不应答的话那些调用会挂到超时 —— 而这个脚本要验的正是它们的结果。
 */
class Host extends Gateway {
  constructor(exe, secret) {
    super(exe);
    this.secret = secret;
    this.child.stdout.removeAllListeners("data");
    this.buffer = "";
    this.child.stdout.on("data", (chunk) => {
      this.buffer += chunk.toString("utf8");
      let index;
      while ((index = this.buffer.indexOf("\n")) >= 0) {
        const line = this.buffer.slice(0, index);
        this.buffer = this.buffer.slice(index + 1);
        if (!line.trim()) continue;
        let frame;
        try {
          frame = JSON.parse(line);
        } catch {
          continue;
        }
        // 插件的回调（有 method、没有 result/error）
        if (frame.method === "store.get") {
          const provider = frame.params?.provider ?? "";
          const id = frame.params?.account || `${provider}-parity`;
          this.child.stdin.write(
            JSON.stringify({
              id: frame.id,
              result: { id, provider, secret: this.secret, public: {} },
            }) + "\n"
          );
          continue;
        }
        if (frame.id && this.pending.has(frame.id)) {
          const { resolve, reject } = this.pending.get(frame.id);
          this.pending.delete(frame.id);
          if (frame.error) reject(new Error(frame.error.message));
          else resolve(frame.result);
        }
      }
    });
  }
}

// ══════════════════════════════════════════════════════════════
// Command Code
// ══════════════════════════════════════════════════════════════

/**
 * 内置实现的期望值（从 `providers/commandcode/plan.rs` 冻结）。
 *
 * ── 这些常量为什么写死在这里 ────────────────────────────────
 * 它们就是「内置实现在这份输入下的输出」。写死 = 冻结契约：插件侧一旦漂移
 * 就报错，而不是两边一起改然后一起错。要改这些值，必须同时改内置实现并
 * 想清楚为什么。
 */
const CC_EXPECT = {
  url: "https://api.commandcode.ai/alpha/generate",
  headerNames: [
    "Content-Type",
    "User-Agent",
    "x-command-code-version",
    "x-cli-environment",
    "x-project-slug",
    "x-taste-learning",
    "x-session-id",
    "Authorization",
    "traceparent",
  ],
  userAgent: "cli",
  cliEnvironment: "production",
  envelopeKeys: [
    "config",
    "memory",
    "taste",
    "skills",
    "permissionMode",
    "mode",
    "params",
  ],
  permissionMode: "standard",
  mode: "agent",
  defaultModel: "deepseek/deepseek-v4-flash",
  defaultMaxTokens: 64000,
  maxTokensCap: 200000,
  placeholderSystem: [{ type: "text", text: " " }],
};

async function checkCommandCode(exe) {
  const host = new Host(exe, accountSecret("commandcode"));
  const infos = await host.init();
  const info = infos.find((item) => item.id === "commandcode");
  checkTruthy("Command Code 在握手里出现", Boolean(info));
  if (!info) return host.close();

  // 能力位（与内置实现逐条对齐）
  check("cc.capabilities.chat", info.capabilities.chat, true);
  check("cc.capabilities.stateful", info.capabilities.stateful, false);
  check("cc.capabilities.refresh", info.capabilities.refresh, false);
  check("cc.capabilities.model_refresh", info.capabilities.model_refresh, true);
  check("cc.capabilities.usage", info.capabilities.usage, false);
  check("cc.capabilities.checkin", info.capabilities.checkin, false);
  check("cc.capabilities.web_login", info.capabilities.web_login, false);
  check("cc.label", info.label, "Command Code");

  // ── chat：最小请求 ───────────────────────────────────────
  const plan = await host.call("chat", {
    provider: "commandcode",
    account: "cc-1",
    proxy: null,
    body: { messages: [{ role: "user", content: "hi" }] },
    headers: {},
    stream: true,
    wire_model: "deepseek/deepseek-v4-flash",
    reasoning: "",
  });
  check("cc.url", plan.url, CC_EXPECT.url);
  check("cc.upstream", plan.upstream, "ndjson");
  const headers = headerMap(plan.headers);
  checkKeySet("cc.headers 集合", headers, CC_EXPECT.headerNames);
  check("cc.headers.User-Agent", headers["User-Agent"], CC_EXPECT.userAgent);
  check("cc.headers.x-cli-environment", headers["x-cli-environment"], CC_EXPECT.cliEnvironment);
  check("cc.headers.Authorization", headers.Authorization, "Bearer user_parity_test_key_123456");
  check("cc.headers.x-taste-learning 是字符串 false", headers["x-taste-learning"], "false");
  checkTruthy(
    "cc.headers.traceparent 是 W3C 形态",
    /^00-[0-9a-f]{32}-[0-9a-f]{16}-01$/.test(headers.traceparent ?? ""),
    headers.traceparent
  );

  // ── 信封 ────────────────────────────────────────────────
  // 有会话 id 时会多一个 threadId（派生结果恒为合法 UUID）
  const expectedKeys = [...CC_EXPECT.envelopeKeys, "threadId"];
  checkKeySet("cc.envelope 键集合", plan.body, expectedKeys);
  check("cc.envelope.memory", plan.body.memory, null);
  check("cc.envelope.taste", plan.body.taste, null);
  check("cc.envelope.skills", plan.body.skills, null);
  check("cc.envelope.permissionMode", plan.body.permissionMode, CC_EXPECT.permissionMode);
  check("cc.envelope.mode", plan.body.mode, CC_EXPECT.mode);
  checkTruthy(
    "cc.envelope.threadId 是合法 UUID",
    /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/.test(
      plan.body.threadId ?? ""
    ),
    plan.body.threadId
  );

  // ── params ──────────────────────────────────────────────
  const params = plan.body.params;
  check("cc.params.model 缺省值", params.model, CC_EXPECT.defaultModel);
  check("cc.params.max_tokens 缺省值", params.max_tokens, CC_EXPECT.defaultMaxTokens);
  check("cc.params.stream 恒 true", params.stream, true);
  check("cc.params.system 空格占位", params.system, CC_EXPECT.placeholderSystem);
  check("cc.params.tools 总是下发（空数组）", params.tools, []);
  checkTruthy("cc.params 没有 tool_choice 键", !("tool_choice" in params));
  check("cc.params.messages 是块数组", params.messages, [
    { role: "user", content: [{ type: "text", text: "hi" }] },
  ]);

  // ── config 块：本地环境字段必须是稳定中性值 ──────────────────
  const config = plan.body.config;
  check("cc.config.workingDir 是伪造值", config.workingDir, "C:\\Users\\dev\\projects\\app");
  check("cc.config.environment", config.environment, "win32");
  check("cc.config.structure", config.structure, []);
  check("cc.config.isGitRepo", config.isGitRepo, false);
  check("cc.config.currentBranch", config.currentBranch, "");
  check("cc.config.recentCommits", config.recentCommits, []);
  checkTruthy(
    "cc.config.date 是 YYYY-MM-DD",
    /^\d{4}-\d{2}-\d{2}$/.test(config.date ?? ""),
    config.date
  );

  // ── max_tokens 上限 ─────────────────────────────────────
  const capped = await host.call("chat", {
    provider: "commandcode",
    account: "cc-1",
    proxy: null,
    body: {
      messages: [{ role: "user", content: "x" }],
      max_tokens: 999999,
    },
    headers: {},
    stream: true,
    wire_model: "m",
    reasoning: "",
  });
  check("cc.params.max_tokens 被截到上限", capped.body.params.max_tokens, CC_EXPECT.maxTokensCap);

  // ── tool_choice = none → 清空 tools 且不下发 tool_choice ─────
  const noTools = await host.call("chat", {
    provider: "commandcode",
    account: "cc-1",
    proxy: null,
    body: {
      messages: [{ role: "user", content: "x" }],
      tools: [{ type: "function", function: { name: "f", parameters: {} } }],
      tool_choice: "none",
    },
    headers: {},
    stream: true,
    wire_model: "m",
    reasoning: "",
  });
  check("cc.tool_choice=none → tools 清空", noTools.body.params.tools, []);
  checkTruthy("cc.tool_choice=none → 不下发 tool_choice", !("tool_choice" in noTools.body.params));

  // ── tool_choice = required → any ────────────────────────
  const anyTools = await host.call("chat", {
    provider: "commandcode",
    account: "cc-1",
    proxy: null,
    body: {
      messages: [{ role: "user", content: "x" }],
      tools: [{ type: "function", function: { name: "f", parameters: {} } }],
      tool_choice: "required",
    },
    headers: {},
    stream: true,
    wire_model: "m",
    reasoning: "",
  });
  check("cc.tool_choice=required → any", anyTools.body.params.tool_choice, { type: "any" });
  check("cc.tools 没有 type 字段", anyTools.body.params.tools[0].type, undefined);

  // ── 错误分类（每个分支）──────────────────────────────────
  const cases = [
    [400, { message: "bad request" }, "fatal", 400],
    [401, { message: "unauthorized" }, "token_expired", null],
    [403, { message: "forbidden" }, "token_expired", null],
    [402, { message: "payment required" }, "quota_limited", 429],
    [404, { message: "not found" }, "fatal", 404],
    [429, { message: "rate limited" }, "quota_limited", 429],
    [503, { message: "unavailable" }, "fatal", 503],
    [500, { message: "boom" }, "fatal", 502],
  ];
  for (const [status, body, kind, downstream] of cases) {
    const result = await host.call("classify", { provider: "commandcode", status, body });
    check(`cc.classify(${status}).kind`, result.kind, kind);
    if (downstream !== null) {
      check(`cc.classify(${status}).status`, result.status, downstream);
    }
  }
  // 内容拦截：400 + 特征文案 → content_blocked
  const blocked = await host.call("classify", {
    provider: "commandcode",
    status: 400,
    body: { message: "Blocked by security policy" },
  });
  check("cc.classify(内容拦截).kind", blocked.kind, "content_blocked");
  checkTruthy(
    "cc.classify(内容拦截).message 带可执行提示",
    blocked.message.includes("系统提示词"),
    blocked.message
  );

  // ── 模型清单（内置兜底 26 项）──────────────────────────────
  const modelList = await host.call("models.list", {
    provider: "commandcode",
    account: "",
  });
  check("cc.models 兜底条目数", modelList.models.length, 26);
  check("cc.models.meta.remote（没拉过远程）", modelList.meta.remote, false);
  const first = modelList.models[0];
  check("cc.models[0].id", first.id, "claude-sonnet-4-6");
  check("cc.models[0].name", first.name, "Claude Sonnet 4.6");
  check("cc.models[0].supportsToolCall", first.supportsToolCall, true);
  check("cc.models[0].supportsReasoning", first.supportsReasoning, true);
  check("cc.models[0].supportsImages", first.supportsImages, false);

  host.close();
  return host;
}

// ══════════════════════════════════════════════════════════════
// ModelScope
// ══════════════════════════════════════════════════════════════

const MS_EXPECT = {
  cn: {
    id: "modelscope",
    label: "ModelScope 国内版",
    inference: "https://api-inference.modelscope.cn/v1/chat/completions",
  },
  intl: {
    id: "modelscope-intl",
    label: "ModelScope 国际版",
    inference: "https://api-inference.modelscope.ai/v1/chat/completions",
  },
};

async function checkModelScope(exe, region, expect) {
  const host = new Host(exe, accountSecret(region.id));
  const infos = await host.init();
  const info = infos.find((item) => item.id === expect.id);
  checkTruthy(`${expect.id} 在握手里出现`, Boolean(info));
  if (!info) return host.close();

  check(`${expect.id}.label`, info.label, expect.label);
  check(`${expect.id}.capabilities.chat`, info.capabilities.chat, true);
  check(`${expect.id}.capabilities.stateful`, info.capabilities.stateful, false);
  check(`${expect.id}.capabilities.refresh`, info.capabilities.refresh, false);
  check(`${expect.id}.capabilities.model_refresh`, info.capabilities.model_refresh, true);
  check(`${expect.id}.capabilities.usage`, info.capabilities.usage, true);
  check(`${expect.id}.capabilities.checkin`, info.capabilities.checkin, true);
  check(`${expect.id}.capabilities.web_login`, info.capabilities.web_login, false);

  // ── chat：URL / 头 / include_usage 注入 ─────────────────
  const plan = await host.call("chat", {
    provider: expect.id,
    account: `${expect.id}-1`,
    proxy: null,
    body: { messages: [{ role: "user", content: "hi" }], model: "Qwen/Qwen3.8-Flash-Next" },
    headers: {},
    stream: true,
    wire_model: "Qwen/Qwen3.8-Flash-Next",
    reasoning: "",
  });
  check(`${expect.id}.url`, plan.url, expect.inference);
  check(`${expect.id}.upstream`, plan.upstream, "chat");
  const headers = headerMap(plan.headers);
  check(
    `${expect.id}.Authorization`,
    headers.Authorization,
    "Bearer ms-parity-token-abcdefgh"
  );
  check(`${expect.id}.Accept`, headers.Accept, "text/event-stream");
  // ⚠️ 这是本家最关键的一处注入（漏了非流式 usage 恒为 0）
  check(
    `${expect.id}.stream_options.include_usage`,
    plan.body.stream_options?.include_usage,
    true
  );
  check(
    `${expect.id}.model 原样透传`,
    plan.body.model,
    "Qwen/Qwen3.8-Flash-Next"
  );
  check(`${expect.id}.messages 原样透传`, plan.body.messages, [
    { role: "user", content: "hi" },
  ]);

  // 客户端自己写过 stream_options 时，只补 include_usage、不覆盖整个对象
  const merged = await host.call("chat", {
    provider: expect.id,
    account: `${expect.id}-1`,
    proxy: null,
    body: { messages: [], stream_options: { custom_flag: 1 } },
    headers: {},
    stream: true,
    wire_model: "m",
    reasoning: "",
  });
  check(`${expect.id}.stream_options 其余键保留`, merged.body.stream_options.custom_flag, 1);
  check(
    `${expect.id}.stream_options.include_usage 仍被置真`,
    merged.body.stream_options.include_usage,
    true
  );

  // ── 错误分类的分支顺序（本家最容易写反的地方）────────────────
  // ① 魔粒耗尽：429 + insufficient balance → QuotaLimited + reset_at 非空
  const balance = await host.call("classify", {
    provider: expect.id,
    status: 429,
    body: { message: "insufficient balance" },
  });
  check(`${expect.id}.classify(魔粒耗尽).kind`, balance.kind, "quota_limited");
  checkTruthy(
    `${expect.id}.classify(魔粒耗尽).reset_at 必须给出来`,
    typeof balance.reset_at === "number" && balance.reset_at > Date.now(),
    balance.reset_at
  );

  // ② 未验证邮箱：403 + 特征串 → QuotaLimited（**不是** TokenExpired）
  const unverified = await host.call("classify", {
    provider: expect.id,
    status: 403,
    body: {
      message:
        "For account security, please complete your profile via email verification. " +
        "ModelScope Magicubes is only available to verified accounts.",
    },
  });
  check(`${expect.id}.classify(未验证邮箱).kind`, unverified.kind, "quota_limited");
  checkTruthy(
    `${expect.id}.classify(未验证邮箱).reset_at`,
    typeof unverified.reset_at === "number",
    unverified.reset_at
  );

  // ③ 裸 403 → TokenExpired
  const forbidden = await host.call("classify", {
    provider: expect.id,
    status: 403,
    body: { message: "access forbidden" },
  });
  check(`${expect.id}.classify(裸 403).kind`, forbidden.kind, "token_expired");

  // ④ 普通 429（速率窗口）→ QuotaLimited 且**不**给 reset_at
  const rateLimit = await host.call("classify", {
    provider: expect.id,
    status: 429,
    body: { message: "rate limit exceeded" },
  });
  check(`${expect.id}.classify(速率窗口).kind`, rateLimit.kind, "quota_limited");
  checkTruthy(
    `${expect.id}.classify(速率窗口).reset_at 应为 None`,
    rateLimit.reset_at === null || rateLimit.reset_at === undefined,
    rateLimit.reset_at
  );

  // ⑤ insufficient balance 配非 429 状态码时**不**走魔粒耗尽那条
  const wrongStatus = await host.call("classify", {
    provider: expect.id,
    status: 400,
    body: { message: "insufficient balance" },
  });
  checkTruthy(
    `${expect.id}.classify(400+insufficient).kind 不是 quota_limited`,
    wrongStatus.kind === "fatal" || wrongStatus.kind === "content_blocked",
    wrongStatus.kind
  );

  // ── 模型清单：本家没有内置兜底，没拉过就是空的 ────────────────
  const modelList = await host.call("models.list", {
    provider: expect.id,
    account: "",
  });
  check(`${expect.id}.models 初始为空`, modelList.models.length, 0);
  check(`${expect.id}.models.meta.remote`, modelList.meta.remote, false);

  host.close();
}

// ══════════════════════════════════════════════════════════════
// 主流程
// ══════════════════════════════════════════════════════════════

async function main() {
  const exe = pluginExe();
  if (!exe) {
    console.log("跳过：插件二进制不存在（先在 agent2api-providers 里 cargo build）");
    return;
  }
  console.log(`插件: ${exe}`);

  await checkCommandCode(exe);
  await checkModelScope(exe, "cn", MS_EXPECT.cn);
  await checkModelScope(exe, "intl", MS_EXPECT.intl);

  console.log(`\n${checks} 项检查，${failures} 项失败`);
  if (failures > 0) process.exit(1);
  console.log("✓ parity 通过：插件实现与内置实现行为一致");
}

main().catch((error) => {
  console.error("脚本异常:", error);
  process.exit(1);
});

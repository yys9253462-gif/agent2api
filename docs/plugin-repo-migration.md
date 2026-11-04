# 插件仓库（Agent2Providers）与网关的关系

插件实现与两个契约 crate 都在独立仓库
[ProvidersPlugin/Agent2Providers](https://github.com/ProvidersPlugin/Agent2Providers)（公有）。
网关**不包含**任何 provider 实现，只按 git tag 引用它的契约 crate。

## 当前形态

| 位置 | 内容 |
|---|---|
| `ProvidersPlugin/Agent2Providers` | 21 家实现 + `provider-protocol` + `provider-sdk` + 打包/签名/验证脚本 + CI 矩阵 |
| 本仓库 `server/Cargo.toml` | `agent2api-provider-protocol = { git = "...", tag = "v0.1.0" }` |

主仓库里**不再有** `agent2api-providers/` 与 `desktop-tauri/src-tauri/crates/`。
`.gitignore` 仍保留 `/agent2api-providers/` 一行 —— 防止有人在主仓库里误建
同名目录再提交进去。

## 改协议的流程

1. 在 Agent2Providers 仓库改 `crates/provider-protocol/`
2. 打新 tag：`git tag v0.1.1 && git push origin v0.1.1`
3. 改本仓库 `server/Cargo.toml` 的 `tag`
4. `cargo update -p agent2api-provider-protocol`（**必须**，否则 Cargo 继续用
   `Cargo.lock` 里记的旧提交）
5. 编译验证：`cd desktop-tauri/src-tauri && cargo check -p agent2api-server`

## 本地联调（同时改网关与契约库）

不想每改一次就发 tag，在**仓库根**建 `.cargo/config.toml`：

```toml
[patch."https://github.com/ProvidersPlugin/Agent2Providers"]
agent2api-provider-protocol = { path = "../Agent2Providers/crates/provider-protocol" }
```

相对路径按实际 checkout 位置写。这个文件**不要提交** —— 它绑定个人磁盘布局。

## 打插件包

在 Agent2Providers 仓库：

```bash
node scripts/pack.mjs                              # 当前平台
node scripts/verify.mjs                            # 协议验证（28 项）
node scripts/sign.mjs sign --zip dist/*.zip        # 签名（需先生成密钥）
```

产物是 `dist/agent2api-providers-<version>-<target>.zip`，五个平台各一份。
CI 矩阵（`.github/workflows/build.yml`）在五个平台各编一次、各自真机跑一遍
协议验证，产物上传为 workflow artifact，**不挂 Release**。

## 还没做的两件事

**签名密钥。** 网关的 `OFFICIAL_PUBKEYS` 仍是空表，所有包显示「未签名」。
启用步骤：在插件仓库 `node scripts/sign.mjs keygen`，把打印的公钥填进
`server/src/server/core/plugin_host/install.rs` 的 `OFFICIAL_PUBKEYS`，
重新编译网关，再给每个 zip 签名。

**更新检查。** `plugin_host/update.rs` 的 `PLUGIN_REPO` 已指向
`ProvidersPlugin/Agent2Providers`，但插件包只出 artifact、不挂 Release，所以「检查
更新」永远返回「没有可更新的版本」。要么把 zip 挂到 Release，要么把这个入口
改成「打开下载页」。

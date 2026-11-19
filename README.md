<h1 align="center">
  <img src="./src-tauri/icons/icon.png" alt="Clash" width="128" />
  <br>
  Continuation of <a href="https://github.com/zzzgydi/clash-verge">Clash Verge</a>
  <br>
</h1>

<h3 align="center">
A Clash Meta GUI based on <a href="https://github.com/tauri-apps/tauri">Tauri</a>.
</h3>

<p align="center">
  Languages:
  <a href="./docs/README_en.md">English</a> ·
  <a href="./docs/README_es.md">Español</a> ·
  <a href="./docs/README_fa.md">فارسی</a> ·
  <a href="./docs/README_ja.md">日本語</a> ·
  <a href="./docs/README_ko.md">한국어</a> ·
  <a href="./docs/README_pt.md">Português</a> ·
  <a href="./docs/README_ru.md">Русский</a> ·
  <a href="./README.md">简体中文</a>
</p>

## 流量版改动

本 fork 基于 Clash Verge Rev v2.5.10。

- 左侧「流量统计」支持按日期、时段和代理或直连筛选。可选「最近 15 分钟」「最近 1 小时」，从流量峰值进入对应时段的应用、目标地址和节点排行，再一键返回。
- 堆叠柱图展示上传与下载，灰色表示该时段存在采集缺口。图表下方展示选中时段用量和最近采集时间，内核总量、未捕获明细和历史覆盖范围可在「数据说明」中查看。
- 节点名以「| 0.35」等单价结尾时，可查看按每 GB 计价的预估费用，也可按费用排序。仅部分流量能计价时会标注「部分计价」，无法计算的金额留空。
- Windows 流量版内置自定义 Mihomo `v1.19.34-traffic.2`。它会记录采样间隔内已关闭的连接，补采时按可确认的事件时间入账。旧账本可导入应用，升级前的历史统计继续保留。
- 订阅卡片可直接开关定时更新，并用快捷按钮选择更新间隔。

## Preview

| Dark                             | Light                             |
| -------------------------------- | --------------------------------- |
| ![预览](./docs/preview_dark.png) | ![预览](./docs/preview_light.png) |

## Install

流量版提供 Windows x64 便携包。先退出正在运行的 Clash Verge，将新版解压到新的目录，再打开 `clash-verge.exe`。如果应用提示更新服务或批准内核，按提示完成后在「设置」中确认内核版本为 `v1.19.34-traffic.2`。 [下载流量版](https://github.com/Banlangenn/clash-verge-rev/releases/latest)

如果想让个别域名走本机网络，可在订阅扩展脚本中将精确域名规则放在通用代理规则之前。规则模式下，以下两条规则分别让 Chrome 优化服务和 Bing 扩展使用直连。

```text
DOMAIN,optimizationguide-pa.googleapis.com,DIRECT
DOMAIN,browserdefaults.microsoft.com,DIRECT
```

### 上游发行版

请到发布页面下载对应的安装包：[Release page](https://github.com/clash-verge-rev/clash-verge-rev/releases)<br>
Go to the [Release page](https://github.com/clash-verge-rev/clash-verge-rev/releases) to download the corresponding installation package<br>
Supports Windows (x64/x86), Linux (x64/arm64) and macOS 11+ (intel/apple).
支持 Windows (x64/x86)、Linux (x64/arm64) 和 macOS 11+ (intel/apple)。

#### 我应当怎样选择发行版

| 版本        | 特征                                     | 链接                                                                                   |
| :---------- | :--------------------------------------- | :------------------------------------------------------------------------------------- |
| Stable      | 正式版，高可靠性，适合日常使用。         | [Release](https://github.com/clash-verge-rev/clash-verge-rev/releases)                 |
| Alpha(废弃) | 测试发布流程。                           | [Alpha](https://github.com/clash-verge-rev/clash-verge-rev/releases/tag/alpha)         |
| AutoBuild   | 滚动更新版，适合测试反馈，可能存在缺陷。 | [AutoBuild](https://github.com/clash-verge-rev/clash-verge-rev/releases/tag/autobuild) |

#### 安装说明和常见问题，请到 [文档页](https://clash-verge-rev.github.io/) 查看

### TG 频道: [@clash_verge_rev](https://t.me/clash_verge_re)

---

## 推广

### 永雏塔菲的魔法卷轴 · 按量付费

永雏塔菲的魔法卷轴提供按量计费的代理订阅。[注册账号](https://hasucn.me/i/4mK1WOmx)并充值后，即可获取 Clash Verge Rev 订阅。使用代理时，流量按所选节点的单价从账户余额扣费。

导入订阅后，可在 Clash Verge Rev 的「代理」页选择节点。在本项目的「流量统计」中，还可以查看各应用、各节点用了多少流量。节点名带有每 GB 单价时，页面会同时展示预估费用。

---

## Features

- 基于性能强劲的 Rust 和 Tauri 2 框架
- 内置[Clash.Meta(mihomo)](https://github.com/MetaCubeX/mihomo)内核，并支持切换 `Alpha` 版本内核。
- 简洁美观的用户界面，支持自定义主题颜色、代理组/托盘图标以及 `CSS Injection`。
- 配置文件管理和增强（Merge 和 Script），配置文件语法提示。
- 系统代理和守卫、`TUN(虚拟网卡)` 模式。
- 可视化节点和规则编辑
- WebDav 配置备份和同步

### FAQ

Refer to [Doc FAQ Page](https://clash-verge-rev.github.io/faq/windows.html)

### Donation

[捐助Clash Verge Rev的开发](https://github.com/sponsors/clash-verge-rev)

## Development

See [CONTRIBUTING.md](./CONTRIBUTING.md) for more details.

To run the development server, execute the following commands after all prerequisites for **Tauri** are installed:

```shell
pnpm i
pnpm run prebuild
pnpm dev
```

`pnpm dev` preserves the Development Channel's installed service state: an
existing service is used, while a previously uninstalled service remains
uninstalled and the app starts in Sidecar mode. Use `pnpm dev:service` to
explicitly install or update the isolated development service before launch,
or `pnpm dev:sidecar` to force the unprivileged Sidecar workflow.

### 同步上游版本

在上游仓库选择 **Watch → Custom → Releases** 可订阅发版通知。收到稳定版通知后，从流量版默认分支建立临时集成分支，把下方 `vX.Y.Z` 换成新版本标签。 [上游发行版](https://github.com/clash-verge-rev/clash-verge-rev/releases) · [GitHub 通知设置](https://docs.github.com/en/subscriptions-and-notifications/get-started/configuring-notifications)

如果新克隆的本地仓库尚未配置 `upstream`，先执行 `git remote add upstream https://github.com/clash-verge-rev/clash-verge-rev.git`。

```powershell
git fetch origin
git fetch upstream tag vX.Y.Z
git switch -c sync/upstream-vX.Y.Z origin/traffic-history-v2.5.10
git merge vX.Y.Z
```

解决冲突后，验证流量账本、订阅更新和 Windows 构建，再将集成分支合回流量版并发版。Mihomo 升级需要单独核对补丁和连接统计。 [Mihomo 补丁及构建说明](./docs/traffic-core/README.md)

## Contributions

Issue and PR welcome!

## Acknowledgement

Clash Verge rev was based on or inspired by these projects and so on:

- [zzzgydi/clash-verge](https://github.com/zzzgydi/clash-verge): A Clash GUI based on tauri. Supports Windows, macOS and Linux.
- [tauri-apps/tauri](https://github.com/tauri-apps/tauri): Build smaller, faster, and more secure desktop applications with a web frontend.
- [Dreamacro/clash](https://github.com/Dreamacro/clash): A rule-based tunnel in Go.
- [MetaCubeX/mihomo](https://github.com/MetaCubeX/mihomo): A rule-based tunnel in Go.
- [Fndroid/clash_for_windows_pkg](https://github.com/Fndroid/clash_for_windows_pkg): A Windows/macOS GUI based on Clash.
- [vitejs/vite](https://github.com/vitejs/vite): Next generation frontend tooling. It's fast!

## Privacy

Clash Verge Rev 不收集任何用户数据，配置与日志仅保存在本地。详见[隐私政策](./PRIVACY.md)。

Clash Verge Rev does not collect any user data; configuration and logs stay on
your own device. See the [Privacy Policy](./PRIVACY.md) for details.

## License

GPL-3.0 License. See [License here](./LICENSE) for details.

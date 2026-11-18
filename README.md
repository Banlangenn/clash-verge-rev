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

- 左侧主菜单新增「流量统计」，可按日期、时段和代理或直连筛选，查看应用、目标地址及节点的流量排行和占比。
- 堆叠柱图展示上传与下载流量，悬停可查看对应时段和用量。
- 如果节点名称以「| 0.35」这类单价结尾，统计页会按每 GB 0.35 元估算上传与下载的费用。没有可用单价时隐藏费用。
- Windows 版可将旧流量账本导入应用，并继续在应用内记录用量。
- 订阅卡片可直接开关定时更新，并用快捷按钮选择更新间隔。

## Preview

| Dark                             | Light                             |
| -------------------------------- | --------------------------------- |
| ![预览](./docs/preview_dark.png) | ![预览](./docs/preview_light.png) |

## Install

流量版提供 Windows x64 便携包。解压后先退出正在运行的 Clash Verge，再打开文件夹内的 clash-verge.exe。[下载流量版](https://github.com/Banlangenn/clash-verge-rev/releases/tag/traffic-v2.5.10.1)

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

想让代理按实际用量扣费？**[用我的邀请链接注册](https://hasucn.me/i/4mK1WOmx)**，充值后即可在网站获取 Clash Verge Rev 订阅。使用代理时，流量按所选节点的单价从账户余额扣费。

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

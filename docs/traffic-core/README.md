# 流量版 Mihomo 内核

Windows x64 流量版使用 Mihomo `v1.19.34-traffic.2`。上游基准为
[`0455d0d28c4526937380505153e65a843d8612e8`](https://github.com/MetaCubeX/mihomo/tree/0455d0d28c4526937380505153e65a843d8612e8)。
补丁为 `/connections/history` 提供已关闭连接的最终流量、事件时间和历史缓冲区丢失记录数。

完整的对应源码随流量版发布，可直接下载解压。也可以在本仓库根目录执行以下 PowerShell 命令，从固定的上游提交重建。

```powershell
git clone --branch v1.19.34 --depth 1 https://github.com/MetaCubeX/mihomo.git mihomo
git -C mihomo rev-parse HEAD
git -C mihomo apply --check --unidiff-zero (Resolve-Path 'docs/traffic-core/mihomo-v1.19.34-traffic.2.patch').Path
git -C mihomo apply --unidiff-zero (Resolve-Path 'docs/traffic-core/mihomo-v1.19.34-traffic.2.patch').Path
```

确认 `rev-parse` 输出为上文的提交。进入 Mihomo 源码目录后，使用 Go 1.26.11 运行测试和构建。

```powershell
$env:GOOS = 'windows'
$env:GOARCH = 'amd64'
$env:CGO_ENABLED = '0'
go test ./tunnel/statistic -run TestTrafficHistory
go build -buildvcs=false -tags with_gvisor -trimpath -ldflags '-X github.com/metacubex/mihomo/constant.Version=v1.19.34-traffic.2 -s -w' -o verge-mihomo.exe .
```

Mihomo 源码及补丁沿用上游的 GPL-3.0 许可。

# Gitee macOS 构建（占位）

Gitee 官方流水线通常不提供免费的 macOS Runner，因此 **Mac DMG 需在自建 Mac 机器上构建**，或通过 Webhook 触发 Mac mini 等自建 Runner。

## 前置

1. 一台 Apple Silicon Mac，已安装 Xcode Command Line Tools、Node.js 18+、Rust stable
2. 在该 Mac 上 clone Gitee 仓库并完成一次本地验证：`npm run build:mac`
3. （可选）在 Mac 上安装 Gitee Go / Jenkins / GitLab Runner 作为 **self-hosted runner**

## 流水线步骤（伪代码）

```yaml
# 需在 macOS self-hosted runner 上执行
stages:
  - name: build-mac
    runs-on: macos-self-hosted
    script:
      - npm ci
      - npm run build:mac
      - |
        VERSION=$(node -p "require('./package.json').version")
        DMG="src-tauri/target/release/bundle/dmg/多多解密_${VERSION}_aarch64.dmg"
        # 上传到内网 AppStore（示例，按实际接口调整）
        curl -F "file=@${DMG}" \
          "http://172.20.2.169:7101/appStore/Software/PC/developer/jdd-crypto/upload"
```

## 产物命名

| 平台 | 文件名 |
|------|--------|
| Windows | `多多解密_{version}_x64-setup.exe` |
| macOS | `多多解密_{version}_aarch64.dmg` |

## 注意事项

- 代码签名与 Apple 公证（Notarization）未在本仓库配置，内网分发可按需后续补充
- Mac 检查更新（下载 dmg）尚未实现，当前 [`app_update.rs`](../src-tauri/src/app_update.rs) 仅支持 Windows 安装包

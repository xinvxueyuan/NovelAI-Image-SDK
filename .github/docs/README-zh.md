# NovelAI Image SDK

NovelAI 图像生成 API 的 Rust SDK，并为 Python、TypeScript（Node.js）、Kotlin、
Swift、Go、C#、C++ 生成绑定。

本仓库是从
[NovelAI-Image-MCP](https://github.com/xinvxueyuan/NovelAI-Image-MCP)
中抽取的**领域核心**，让任意语言都能生成图像、运行 Director 工具、传输 vibe、
标注图像和读取账户数据，而无需重复实现线协议。

返回 [English README](../../README.md)。

## 为什么用 Rust 核心

- **一份实现，七种语言。** 线协议（请求体形状、校验规则、Anlas 计费公式、错误分类）
  只维护一次；绑定是生成的，不是手抄的。
- **能通过 Cloudflare 的传输层。** `image.novelai.net` 会重置 TLS 指纹不像浏览器的
  连接。SDK 通过 [`wreq`](https://github.com/0x676e67/wreq) 复刻 Chrome 的 BoringSSL
  ClientHello、HTTP/2 设置与完整请求头块。实测结果见
  [`docs/cloudflare-spike.md`](../../docs/cloudflare-spike.md)。
- **字节级一致性。** 黄金夹具由参考实现（Python）生成，断言请求体与错误消息完全一致。

## 快速开始（Rust）

```rust,no_run
use novelai_image_sdk::auth::Credentials;
use novelai_image_sdk::client::NovelAiClient;
use novelai_image_sdk::config::ClientConfig;
use novelai_image_sdk::models::GenerationRequest;

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let client = NovelAiClient::new(ClientConfig::new(Credentials::token("YOUR_TOKEN")))?;
let request = GenerationRequest::new("1girl, smile").build()?;
for image in client.generate(&request).await? {
    std::fs::write(&image.filename, &image.data)?;
}
# Ok(())
# }
```

## 功能矩阵

| 领域 | API |
|---|---|
| 生成 | `generate`（V3/Furry ZIP 与 V4+/V5 MessagePack 自动嗅探） |
| 流式 | `stream_generation`（Rust `Stream`；绑定层为回调/缓冲式） |
| Director | `director`（`lineart`、`sketch`、`bg-removal`、`declutter`、`colorize`、`emotion`） |
| 增强 | `upscale`（仅 V5，4 倍）、`annotate`（ControlNet 模型，legacy 主机） |
| Vibe | `encode_vibe`，带 LRU 编码缓存 |
| 标签 | `suggest_tags` |
| 账户 | `get_subscription`、`get_user_data` |
| 本地 | `GenerationRequest::estimate_anlas_cost`、模型能力表 |

## 构建前置条件

BoringSSL 从源码编译，需要：

- `PATH` 中有 `cmake` 与 `nasm`
- `LIBCLANG_PATH` 指向 `libclang`（PyPI 的 `libclang` wheel 约 20 MB 即可，
  无需 610 MB 的完整 LLVM）

## 绑定路线图

| 语言 | 阶段 | 包名 |
|---|---|---|
| Python | v0.1 | `novelai-image-sdk`（PyPI） |
| TypeScript / Node.js | v0.1 | `@xinvxueyuan/novelai-image-sdk`（npm） |
| Kotlin（JVM/Android） | v0.1 | `io.github.xinvxueyuan:novelai-image-sdk`（Maven Central） |
| Swift | v0.1 | `NovelAIImageSDK`（Swift Package / XCFramework） |
| Go | v0.2 | `github.com/xinvxueyuan/NovelAI-Image-SDK/bindings/go` |
| C# /.NET | v0.2 | `NovelAI.Image.Sdk`（NuGet） |
| C++ | v0.2 | CMake 包（仅阻塞 API） |

## 数据与隐私

凭证永不写入日志；`Credentials` 的 `Debug`/`Display` 只打印凭证**种类**。图像以字节
返回，只有调用 `output::save_image` 才会落盘。

## 许可证

MIT。参见 [`LICENSE`](../../LICENSE) 与
[`THIRD_PARTY_NOTICES.md`](../../THIRD_PARTY_NOTICES.md)（UniFFI 运行时为 MPL-2.0；
BoringSSL 为 Apache-2.0）。

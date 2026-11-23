# NovelAI Image SDK

NovelAI 画像生成 API の Rust SDK です。Python、TypeScript（Node.js）、Kotlin、
Swift、Go、C#、C++ 向けのバインディングを生成します。

本リポジトリは
[NovelAI-Image-MCP](https://github.com/xinvxueyuan/NovelAI-Image-MCP)
から抽出した**ドメインコア**で、任意の言語から画像生成、Director ツール、
vibe 転送、アノテーション、アカウント情報の取得を、ワイヤプロトコルを
再実装せずに行えます。

[English README](../../README.md) に戻る。

## Rust コアを選ぶ理由

- **ひとつの実装、7 言語。** ワイヤプロトコル（リクエスト形状、検証ルール、
  Anlas 計算式、エラー分類）は一箇所だけ。バインディングは生成され、手で
  書き直されることはありません。
- **Cloudflare を通過できるトランスポート。** `image.novelai.net` はブラウザ
  らしくない TLS フィンガープリントの接続をリセットします。SDK は
  [`wreq`](https://github.com/0x676e67/wreq) で Chrome の BoringSSL
  ClientHello・HTTP/2 設定・ヘッダーブロックを再現します。実測結果は
  [`docs/cloudflare-spike.md`](../../docs/cloudflare-spike.md) を参照。
- **バイト単位の一致。** 参照実装（Python）から生成したゴールデンフィクスチャで、
  リクエストボディとエラーメッセージが完全一致することを検証します。

## クイックスタート（Rust）

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

## 機能

| 領域 | API |
|---|---|
| 生成 | `generate`（V3/Furry ZIP と V4+/V5 MessagePack を自動判別） |
| ストリーミング | `stream_generation`（Rust `Stream`、バインディングはコールバック/バッファ） |
| Director | `director`（`lineart`、`sketch`、`bg-removal`、`declutter`、`colorize`、`emotion`） |
| 拡張 | `upscale`（V5 のみ、4 倍）、`annotate`（ControlNet、レガシーホスト） |
| Vibe | `encode_vibe`（LRU キャッシュ付き） |
| タグ | `suggest_tags` |
| アカウント | `get_subscription`、`get_user_data` |
| ローカル | `GenerationRequest::estimate_anlas_cost`、モデル能力表 |

## ビルド要件

BoringSSL をソースからビルドするため、次が必要です:

- `PATH` 上の `cmake` と `nasm`
- `LIBCLANG_PATH`（PyPI の `libclang` wheel 約 20 MB で十分。610 MB の
  LLVM 全体は不要）

## バインディングのロードマップ

| 言語 | フェーズ | パッケージ |
|---|---|---|
| Python | v0.1 | `novelai-image-sdk`（PyPI） |
| TypeScript / Node.js | v0.1 | `@xinvxueyuan/novelai-image-sdk`（npm） |
| Kotlin（JVM/Android） | v0.1 | `io.github.xinvxueyuan:novelai-image-sdk`（Maven Central） |
| Swift | v0.1 | `NovelAIImageSDK`（Swift Package / XCFramework） |
| Go | v0.2 | `github.com/xinvxueyuan/NovelAI-Image-SDK/bindings/go` |
| C# /.NET | v0.2 | `NovelAI.Image.Sdk`（NuGet） |
| C++ | v0.2 | CMake パッケージ（ブロッキング API のみ） |

## データとプライバシー

認証情報はログに出力されません。`Credentials` の `Debug`/`Display` は
認証情報の**種類**のみを表示します。画像はバイト列として返され、
`output::save_image` を呼ばない限りファイルに書き込まれません。

## ライセンス

MIT。[`LICENSE`](../../LICENSE) と
[`THIRD_PARTY_NOTICES.md`](../../THIRD_PARTY_NOTICES.md)（UniFFI ランタイムは
MPL-2.0、BoringSSL は Apache-2.0）を参照してください。

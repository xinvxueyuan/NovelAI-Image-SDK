# NovelAI Image SDK (Node.js)

Node.js bindings for [`novelai-image-sdk`](https://github.com/xinvxueyuan/NovelAI-Image-SDK),
built with napi-rs over the Rust core.

```js
const { NovelAiClient } = require("@xinvxueyuan/novelai-image-sdk");

const client = new NovelAiClient({ token: process.env.NOVELAI_TOKEN }, null);

const request = { prompt: "1girl, smile", width: 832, height: 1216 };

// Validate and inspect without sending anything.
const resolved = client.resolveRequest(request);
console.log(resolved.width, resolved.payloadJson);
console.log(client.estimateAnlasCost(request, false));

const images = await client.generate(request);
for (const image of images) {
  await require("node:fs/promises").writeFile(image.filename, image.data);
}
```

Requires Node.js `>= 20.17`. Every field of `GenerationRequest` is optional; the
core applies the documented defaults (832x1216, 28 steps, Karras,
`nai-diffusion-4-5-full`) and the same validation the MCP server uses.

## Transport

Requests carry Chrome's TLS (JA3/JA4) and HTTP/2 fingerprint plus the matching
header block, because `image.novelai.net` sits behind Cloudflare and resets
connections that do not look like a browser. There is no configuration for
this; it is why the SDK exists.

## TypeScript

`index.d.ts` is generated with the addon, so `NovelAiClient`, `GenerationRequest`,
`Image` and the rest are typed.

## License

MIT. Binaries link BoringSSL (Apache-2.0); see `THIRD_PARTY_NOTICES.md` in the
repository.

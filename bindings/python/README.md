# NovelAI Image SDK (Python)

Python bindings for [`novelai-image-sdk`](https://github.com/xinvxueyuan/NovelAI-Image-SDK),
built with UniFFI over the Rust core.

```python
from novelai_image_sdk import (
    Credentials,
    GenerationRequest,
    NovelAiClient,
)

client = NovelAiClient(Credentials(token="YOUR_TOKEN"), None)
request = GenerationRequest(
    prompt="1girl, smile",
    base_caption=None,
    model=None,
    action=None,
    negative_prompt=None,
    width=832,
    height=1216,
    n_samples=None,
    steps=None,
    scale=None,
    sampler=None,
    seed=None,
    extra_noise_seed=None,
    noise_schedule=None,
    quality=None,
    uc_preset=None,
    cfg_rescale=None,
    image=None,
    mask=None,
    strength=None,
    noise=None,
    references=None,
    reference_information=None,
    estimate_only=None,
    # 0.2.0 fields default to `None`, so they may be omitted: smea,
    # smea_dynamic, auto_smea, character_prompts, prefer_brownian, ...
)

# Validate and inspect without sending anything.
resolved = client.resolve_request(request)
print(resolved.width, resolved.effective_prompt)
print(client.estimate_anlas_cost(request, False).anlas)

for image in client.generate(request):
    with open(image.filename, "wb") as handle:
        handle.write(bytes(image.data))
```

`GenerationRequest` mirrors the Rust record, so the pre-0.2.0 fields stay
explicit. The advanced options added in 0.2.0 default to `None` and can be
omitted. The core applies the documented defaults (832x1216, 28 steps, Karras,
`nai-diffusion-4-5-full`) and the same validation the MCP server uses.

Multi-character prompts (V4+) take `CharacterPrompt` records:

```python
from novelai_image_sdk import CharacterPrompt, GenerationRequest

request = GenerationRequest(
    prompt="2girls",
    base_caption=None,
    model=None,
    action=None,
    negative_prompt=None,
    width=None,
    height=None,
    n_samples=None,
    steps=None,
    scale=None,
    sampler=None,
    seed=None,
    extra_noise_seed=None,
    noise_schedule=None,
    quality=None,
    uc_preset=None,
    cfg_rescale=None,
    image=None,
    mask=None,
    strength=None,
    noise=None,
    references=None,
    reference_information=None,
    estimate_only=None,
    character_prompts=[
        CharacterPrompt(
            prompt="girl with red hair",
            negative_prompt="",
            x=0.25,
            y=0.5,
            enabled=True,
        )
    ],
)
```

Real-time generation (V4+ / V5) is exposed as `collect_generation`, because
UniFFI has no streaming export. It drains the event stream into a list; pass
`max_events` to cap how many events cross the boundary.

## Transport

Requests carry Chrome's TLS (JA3/JA4) and HTTP/2 fingerprint plus the matching
header block, because `image.novelai.net` sits behind Cloudflare and resets
connections that do not look like a browser. There is no configuration for
this; it is why the SDK exists.

## License

MIT. Binaries link the UniFFI runtime (MPL-2.0) and BoringSSL (Apache-2.0);
see `THIRD_PARTY_NOTICES.md` in the repository.

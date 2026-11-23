"""Generate golden fixtures from the pinned reference implementation.

Run from the repository root:

    uv run --directory ../NovelAI-Image-MCP/apps/server python ../../NovelAI-Image-SDK/scripts/gen_fixtures.py

(or `uv run --with ...` from this repo). The reference is the
`NovelAI-Image-MCP` checkout pinned in `third_party/novelai-image-mcp`; until
the submodule lands, point `NOVELAI_REFERENCE_ROOT` at a checkout.

The output is deterministic JSON with sorted keys where order does not matter
and explicit lists where it does (payload key order).
"""

from __future__ import annotations

import base64
import json
import os
import pathlib
import sys

REFERENCE = pathlib.Path(
    os.environ.get("NOVELAI_REFERENCE_ROOT", "third_party/novelai-image-mcp")
)
OUT = pathlib.Path(__file__).resolve().parent.parent / "fixtures" / "golden"

sys.path.insert(0, str(REFERENCE / "apps" / "server" / "src"))

from novelai_image_mcp.nai import (  # noqa: E402
    UPSCALE_MODELS,
    Action,
    CharacterPrompt,
    ControlNetModel,
    DirectorTool,
    Emotion,
    EmotionLevel,
    GenerationRequest,
    Model,
    NoiseSchedule,
    Sampler,
    build_generation_payload,
    check_status,
    derive_access_key,
    is_inpaint_model,
    is_v4_model,
    is_v5_model,
    params_version_for,
    supports_upscale,
    supports_vibe,
)
from novelai_image_mcp.nai.constants import (  # noqa: E402
    QUALITY_TAGS,
    UC_PRESETS,
)
from novelai_image_mcp.nai.errors import parse_error_body  # noqa: E402
from novelai_image_mcp.nai.exceptions import NovelAIError  # noqa: E402


def dump(name: str, value: object) -> None:
    path = OUT / name
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"wrote {path.relative_to(OUT.parent.parent)}")


def payload_fixture(request: GenerationRequest) -> dict:
    # ``json.dumps`` with the same separators the client sends.
    payload = build_generation_payload(request)
    return {
        "request": {
            "prompt": request.prompt,
            "model": request.model.value,
            "action": request.action.value,
            "negative_prompt": request.negative_prompt,
            "width": request.width,
            "height": request.height,
            "n_samples": request.n_samples,
            "steps": request.steps,
            "scale": request.scale,
            "sampler": getattr(request.sampler, "value", request.sampler),
            "seed": request.seed,
            "noise_schedule": getattr(
                request.noise_schedule, "value", request.noise_schedule
            ),
            "quality": request.quality,
            "uc_preset": request.uc_preset,
            "references": list(request.references),
            "character_prompts": [
                {
                    "prompt": c.prompt,
                    "negative_prompt": c.negative_prompt,
                    "x": c.x,
                    "y": c.y,
                    "enabled": c.enabled,
                }
                for c in request.character_prompts
            ],
            "image": request.image,
            "mask": request.mask,
            "strength": request.strength,
            "noise": request.noise,
            "smea": request.smea,
            "smea_dynamic": request.smea_dynamic,
            "auto_smea": request.auto_smea,
            "straight_alpha": request.straight_alpha,
        },
        # Compact JSON is what goes on the wire.
        "wire": json.dumps(payload, ensure_ascii=False, separators=(",", ":")),
        "keys": {
            "top": list(payload),
            "parameters": list(payload["parameters"]),
        },
    }


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)

    dump(
        "constants.json",
        {
            "models": [
                {
                    "id": m.value,
                    "is_v4": is_v4_model(m),
                    "is_v5": is_v5_model(m),
                    "inpaint": is_inpaint_model(m),
                    "vibe": supports_vibe(m),
                    "upscale": supports_upscale(m),
                    "params_version": params_version_for(m),
                    "quality": QUALITY_TAGS[m],
                    "uc_presets": list(UC_PRESETS[m]),
                }
                for m in Model
            ],
            "samplers": [s.value for s in Sampler],
            "noise_schedules": [n.value for n in NoiseSchedule],
            "actions": [a.value for a in Action],
            "controlnet_models": [c.value for c in ControlNetModel],
            "director_tools": [d.value for d in DirectorTool],
            "emotions": [e.value for e in Emotion],
            "emotion_levels": [[e.name, int(e)] for e in EmotionLevel],
            "upscale_models": sorted(m.value for m in UPSCALE_MODELS),
        },
    )

    dump(
        "access_key.json",
        [
            {"username": user, "password": pw, "key": derive_access_key(
                type("C", (), {"username": user, "password": pw})()
            )}
            for user, pw in [
                ("user@example.com", "hunter2"),
                ("alice", "correct horse battery staple"),
                ("a", "p"),
            ]
        ],
    )

    # Payload matrix: model family × action × quality × uc_preset × characters.
    fixtures = []
    for model in (Model.V3, Model.V4, Model.V4_5, Model.V5):
        for uc_preset in (0, 1, 2, 3):
            request = GenerationRequest(
                prompt="1girl, smile",
                negative_prompt="nsfw, blurry",
                model=model,
                uc_preset=uc_preset,
                seed=42,
            )
            fixtures.append(payload_fixture(request))
    fixtures.append(
        payload_fixture(
            GenerationRequest(
                prompt="1girl",
                model=Model.V4_5,
                character_prompts=(
                    CharacterPrompt(prompt="boy", negative_prompt="glasses", x=0.25, y=0.75),
                    CharacterPrompt(prompt="cat", enabled=False),
                ),
            )
        )
    )
    fixtures.append(
        payload_fixture(
            GenerationRequest(
                prompt="1girl",
                model=Model.V4_5,
                quality=False,
                smea=True,
                smea_dynamic=True,
                auto_smea=True,
                straight_alpha=True,
            )
        )
    )
    fixtures.append(
        payload_fixture(
            GenerationRequest(
                prompt="1girl",
                model=Model.V5,
                quality=False,
                straight_alpha=True,
            )
        )
    )
    fixtures.append(
        payload_fixture(
            GenerationRequest(
                prompt="1girl",
                model=Model.V4_5,
                action=Action.IMG2IMG,
                image="aGVsbG8=",
                strength=0.55,
                noise=0.1,
                extra_noise_seed=1234,
            )
        )
    )
    fixtures.append(
        payload_fixture(
            GenerationRequest(
                prompt="1girl",
                model=Model.V4_5_INPAINT,
                action=Action.INPAINT,
                image="aGVsbG8=",
                mask="bWFzaw==",
                extra_noise_seed=7,
            )
        )
    )
    dump("payloads.json", fixtures)

    # Error bodies and status mapping.
    bodies = [
        (400, '{"statusCode":400,"message":"bad prompt"}'),
        (401, '{"statusCode":401,"message":"Unauthorized"}'),
        (402, '{"statusCode":402,"message":"Subscribe"}'),
        (429, '{"statusCode":429,"message":"Too many requests"}'),
        (409, '{"statusCode":409,"message":"Conflict"}'),
        (500, '{"message":"boom"}'),
        (500, "raw text"),
        (400, '{"code":"E_CUSTOM","message":"bad"}'),
        (400, '{"error":"boom"}'),
        (400, '{"detail":" nope "}'),
    ]
    errors = []
    for status, body in bodies:
        info = parse_error_body(body.encode())
        try:
            check_status(status, body.encode())
            raised = None
        except NovelAIError as exc:
            raised = {
                "type": type(exc).__name__,
                "message": str(exc),
                "code": exc.code,
                "explanation": exc.explanation,
            }
        errors.append(
            {
                "status": status,
                "body": body,
                "parsed": {"code": info.code, "message": info.message},
                "raised": raised,
            }
        )
    dump("errors.json", errors)


if __name__ == "__main__":
    main()

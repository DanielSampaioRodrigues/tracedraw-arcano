# AI studio (Arcano edition)

The edition adds to AI Generate several picture services at once, each
with its own key, and an Edit page for what is done to a bitmap. The
work is in `apps/tracedraw/src/studio/`; the services, models, sizes,
shapes, choices and listed prices are data in
`assets/arcano/ai-catalog.json`.

## Services

| Service | Address | Key | Answers |
|---|---|---|---|
| Google Gemini | `generativelanguage.googleapis.com/v1beta`, `generateContent` | `x-goog-api-key` | at once, `inlineData` |
| OpenAI | `api.openai.com/v1`, `images/generations`, `images/edits` (JSON) | `Authorization: Bearer` | at once, `b64_json` |
| Recraft | `external.api.recraft.ai/v1` | `Authorization: Bearer` | at once, `b64_json` (SVG for `_vector` models and vectorize) |
| Magnific (Freepik) | `api.magnific.com/v1` | `x-magnific-api-key` | a task: `GET <path>/{task_id}` every 4 s until `COMPLETED`, then the links (valid 30 min) are downloaded |
| Stability AI | `api.stability.ai/v2beta` (multipart) | `Authorization: Bearer` | at once, `image` in base64; creative upscale and replace background a task, `GET /v2beta/results/{id}` every 10 s until 200 |

Each was written from the service's documentation on 2026-10-23. Every
adapter is a blocking pipeline run on a worker thread through a
`Transport` (the network, or a fake in tests).

- Pictures sent are fitted to the service's limits first
  (`studio::fit_image`: pixels, longer and shorter side).
- Stability takes English only: a request with non-English words is
  first translated by the main AI service (Tools > Options > AI >
  Service) with `studio::TRANSLATE_PROMPT`; without one it is refused
  with a message and nothing is sent.
- Magnific's background removal takes a link: the picture is uploaded
  (`uploads/request-url`, then a PUT with the headers named, without the
  key) and its asset link is sent.
- OpenAI sizes are the exact shape at the size chosen: about 1, 4 or
  8.3 megapixels, sides in multiples of 16, 1:3 to 3:1, at most 3840 a
  side. Expanding there is an edit of the picture on a larger
  transparent canvas, the canvas also being the mask.
- Gemini has no tool for backgrounds, light or enlarging: those are
  edits asked for in words, and removing a background asks for flat
  green, keyed out by the editor (as Remove Background upstream).

## Keys

Tools > Options > AI > Keys: one field per service (the five above and
the text services Anthropic, Maritaca AI and DeepSeek), Get a key (opens
the service's key page), Test (a request that makes nothing: Stability's
balance, Recraft's balance, the model lists of OpenAI and Gemini,
Magnific's Mystic task list) and Remember the keys on this computer (on;
kept in the preferences file in plain text). A key typed for a text
preset in Service is kept for that company here too, and the other way
round.

## AI Generate

A Service row heads Create, Remix and Create vectors: Main service (the
upstream behaviour) or a studio service, "(no key)" after those without
one. With a studio service, its Model (each with its note and price),
Size (1K, 2K, 4K as the model makes them), Shape (the model's own list,
and Match reference image) and the model's choices follow; the docker's
own model and shape rows are left out. The model taken is the one
picked, else the newest of the kind the page makes (vectors on Create
vectors), else the first. Pictures go on the page as upstream; a vector
model's SVG becomes a group of curves; on Create vectors a picture model's
pictures are traced.

## Edit page

The fifth mode button. It names the selected bitmap's size in pixels and
its resolution at its size on the page, then the operations, the service
(those that do it), How (the service's variants, each with its note and
price), the variant's choices, the words (for the operations that take
them; optional for Expand) and Apply.

| Operation | Result |
|---|---|
| Upscale | the pixels replaced in the same frame |
| Expand | the frame grown by the pixels added on each side (`studio::app::grown_rect`, page y up); Expand to the shape fills the sides to reach a shape, evenly on the two that need it (`studio::ui::pads_to`) |
| Remove background | the background made transparent (keyed out when the answer has none) |
| Replace background, Relight | the pixels replaced in the same frame |
| Vectorize | a group of curves over the picture, fitted to its frame |

Each result is one undo step ("AI Studio").

## Catalog

Built in, and replaced by a newer one (`version` higher) published at
`assets/arcano/ai-catalog.json` on the edition's repository, fetched at
most once a day at start and kept in the preferences folder.

## Checks

- `studio::catalog::tests::*`: the catalog reads and names every service;
  a fetched one replaces only an older one; sizes and shapes fall back.
- `studio::{stability,recraft,freepik,openai,gemini}::tests::*`: each
  service's requests, fields, size limits, tasks followed to the end,
  downloads, uploads and errors.
- `studio::tests::words_for_an_english_only_service_are_translated_first`
- `studio::app::tests::*`: created pictures, upscale in the same frame
  as one undo step, expand growing the frame, vectors over the picture,
  a missing key named, keys kept by company and shared with the main
  service.
- `studio::ui::tests::*`: every page draws in every state; Expand to the
  shape.
- Looked at in the window: the Service, Model, Size and Shape rows, the
  Edit page with a selected bitmap, the Keys page.

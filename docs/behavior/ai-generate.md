# AI Generate and Remove Background

Pictures made, remixed or cut out by the one AI service the user sets
up with their own API key (Tools > Options > AI > Service), when that
service can make pictures: OpenAI, Google Gemini, or a custom
OpenAI-compatible address with an image model (see `ai-options.md`).
The Messages API and the local chat-only servers make drawings only.
TraceDraw has no AI of its own and no account, and nothing is sent until
the user asks. The work is in `apps/tracedraw/src/ai_image.rs`.

## Where

- Tools > AI Generate, the AI Generate button on the standard toolbar
  (its arrow lists AI Generate, Draw with Words, Remove Background and
  Options > AI) and Window > Dockers open the AI Generate docker.
- Bitmaps > Remove Background, the Remove Background button on the
  property bar of a selected bitmap, and Remove Background and Remix with
  AI at the top of a bitmap's context menu.
- Every AI picture command is greyed out while AI features are off, the
  service makes drawings only, or it is not set up for pictures; the
  greyed commands' hover tip and the docker's banner say which, and the
  banner's Set Up AI button opens the right options page.

## Services

The image calls follow from the one service (`ai::AiPrefs::image_prefs`)
and use its key: the OpenAI preset calls the OpenAI Images API, the
Google Gemini preset (whose chat address is Gemini's OpenAI-compatible
one) calls the Gemini API at its own base, and a custom address calls
the OpenAI Images API at the same base with the image model entered.

| Image API | Address (default) | Model (default) | Key |
|---|---|---|---|
| Google Gemini API | `https://generativelanguage.googleapis.com/v1beta` | `gemini-nano-banana-2.1` (also offered: `gemini-3-pro-image`) | `x-goog-api-key` |
| OpenAI Images API, or a service with the same API | `https://api.openai.com/v1` | `gpt-image-2.5-flare` (also `gpt-image-2.5-sunburst`) | `Authorization: Bearer`; optional at an address of the user's |

The offered models are the current ones (checked 2026-10-23): the
newest of each service, and the Gemini Pro model, which costs about four
times as much for more detail. Models the services have switched off
(`ai_image::RETIRED_IMAGE_MODELS`: `gemini-2.5-flash-image`, the
`-preview` names, `gpt-image-1`, `gpt-image-1-mini`, DALL-E, Imagen 4)
are asked for as their replacement at the service's own address; at an
address of the user's the name is sent as typed.

### Size

Size (in the docker under the model, and in Tools > Options > AI >
Service, kept as `ai.image_resolution`): 1K, 2K or 4K, each with its
pixels and its width at 300 dpi; only the sizes the model makes are
listed (`ai_image::resolutions`: 1K, 2K and 4K for the current Gemini
models, 1K and 2K for GPT Image 2.5, 1K otherwise), and a size the model
does not make falls to the nearest smaller one. Gemini gets
`imageConfig.imageSize` ("2K", "4K"; 1K is its own size and is not
named), also for edits. GPT Image 2.5 at 2K is asked for 2048 x 2048,
2048 x 1360 or 1360 x 2048.

### Price

Under the model, and under each model in its list, a line gives the
price of one picture at the chosen size (`ai_image::price_estimate`):
the services' listed prices on 2026-10-23, standard tier, in US dollars
with the decimal separator of the interface language
(`aiimg.decimal_point`).

| Model | 1K | 2K | 4K |
|---|---|---|---|
| `gemini-nano-banana-2.1` | 0.034 | 0.050 | 0.113 |
| `gemini-3-pro-image` | 0.134 | 0.134 | 0.24 |
| `gemini-3.1-flash-image` (if listed by the service) | 0.067 | 0.101 | 0.151 |
| `gpt-image-2.5-flare`, `-sunburst` | 0.006 to 0.21 by quality (the service picks it) | "costs more than at 1K" | not offered |

A model whose price is not known gets no line. Prices change; the line
says it is the listed price, and the service's own price list decides.

- Gemini: `POST {address}/models/{model}:generateContent` with the
  prompt (and the picture as `inline_data` for Remix and Remove
  Background), `responseModalities` TEXT and IMAGE and the aspect ratio
  (and a size above 1K) in `imageConfig`. One picture per request: asking for 3 sends 3
  requests. Pictures come back in `inlineData`; a refusal
  (`promptFeedback.blockReason`) or text without a picture is shown.
- OpenAI: `POST {address}/images/generations` (JSON: model, prompt, `n`
  up to 4, size 1024 x 1024, 1536 x 1024 or 1024 x 1536; 1792 x 1024 or
  1024 x 1792 for `dall-e-3`, which also gets `response_format
  b64_json`) and `POST {address}/images/edits` (multipart: model, prompt,
  `image`; `background transparent` and `output_format png` for Remove
  Background). Pictures come back as `b64_json`; a reply with links only
  asks for another model.
- Test Connection (Tools > Options > AI > Service) also lists the service's image models (`GET
  {address}/models`, names with "image" or starting with "dall-e"); they
  then fill the model lists.
- Replies up to 64 MB are read; PNG, JPEG and WebP pictures are taken
  and stored as PNG.

## The docker

At the top, three picture buttons pick the mode (Create image, Remix
image, Create vectors) and an information button at the right shows a
page instead of the settings: what each mode does, each service with its
model and whether it is ready, costs (each picture is one request to the
user's service, billed by it; local servers cost nothing), what is sent,
and Set Up AI. The settings and results scroll; the footer stays at the
bottom: Images (1 to 4), Insert into drawing (on), how many requests go
to which service, and Generate (while waiting, a greyed "Generation in progress..." button,
the progress when several pictures come, a spinner and Cancel), then the outcome.

Every mode has:

- Model: the service's models (the usual ones until Test Connection lists
  its own), each shown in the list as a radio row with a line on what it
  is good at; a ? button repeats the note of the model in use.
- A reference picture: Source image for Remix (needed; the selected
  bitmap until another is chosen), Example image (optional) for the
  others. Under its thumbnail, Use Selection (the selected bitmap as it
  is, or the selected objects drawn at about 1024 px on the longer side)
  and Import File (PNG, JPEG, WebP, BMP, GIF, TIFF); Reset clears it.
- Prompt with Reset, its hint listing what to describe (characters and
  objects, what happens, the scene, important details).
- Style and Color palette tiles, each section with Reset; a second click
  on a tile turns it off. A chevron folds both away.

Modes:

- Create image: Inspire me fills in an idea; aspect ratio Square 1:1,
  Landscape 5:4, 4:3 and 3:2, Wide 16:9, Ultrawide 21:9, Portrait 4:5,
  3:4 and 2:3, Tall 9:16, and Match reference image while there is one
  (the nearest of those to its shape). With an example image Gemini gets
  it with the prompt and the OpenAI API edits it (one request per
  picture).
- Remix image: ready-made requests (Mockup, Restore photo, Colorize
  sketch, Replace background, Change season, Recolor) fill in the prompt.
  The picture goes over the source bitmap at its place and height, with
  its own proportions (centred on the page when the source came from a
  file or from vectors).
- Create vectors, made with:
  - Picture + trace: a flat illustration on white from the service's
    image model, traced with the Clipart preset into a group of curves that
    replaces the picture.
  - SVG drawing: the service's text model (Tools > Options > AI >
    Service) is asked for one SVG document (`ai_image::SVG_PROMPT`: only
    shapes and solid colours or gradients, no text, images or scripts),
    imported as a group of editable shapes centred on the page within
    four fifths of it.
  Vector styles: Flat illustration, Line art, Comic book, Icon, Hand
  drawn, Kawaii poster.

Each Generate replaces the Results (up to 12); with Insert into drawing
on, each picture is also placed centred on the page within four fifths
of it, as its own undo step ("AI Generate"), and selected. A click on a
result inserts it. Pictures are named "AI-generated bitmap", vectors
"AI-generated vector".

Object > AI Generate opens the docker in each mode (Create Image, Remix
Image, Create Vector Graphic) and has Remove Background; a bitmap's
context menu starts with Edit Bitmap, Remove Background... (on this
computer), Remove Background, Quick Trace,
Crop Bitmap and Remix with AI (the bitmap becomes the source).

## Remove Background

The selected bitmap (one) is sent with an instruction to keep the
subject. With the OpenAI API the reply has a transparent background.
Gemini cannot answer with transparency, so it is asked to paint
everything but the subject flat pure green (#00FF00), which is then
keyed out here: a pixel whose green exceeds its red and blue by 120 or
more becomes transparent, by 40 or less stays, and in between keeps part
of its opacity with the green spill taken out (green lowered to the
larger of red and blue). The picture is replaced in place, as one undo
step ("Remove Background").

## Privacy

Only the prompt, and for Remix and Remove Background the selected
picture, go to the AI service set up, when the user presses Generate
or Remove Background. Keys stay in memory unless "Remember the key on
this computer" is on (then they are in the preferences file in plain
text).

## Checks

- Requests for both services: addresses, key headers, prompt, aspect
  ratio, `n` capped at 4, sizes, multipart body with the picture and
  transparency (`ai_image::tests::requests_are_built_for_both_kinds_of_service`).
- Prompts carry the style, palette and vector words and are cut at the
  limit (`ai_image::tests::prompts_carry_style_palette_and_vector_words`).
- Replies of both services, error replies, refusals, text without a
  picture, link replies and unreadable bodies
  (`ai_image::tests::replies_and_errors_are_read`).
- The key-out of green keeps colours, removes the key and softens edges
  (`ai_image::tests::green_is_keyed_out_and_edges_keep_part_of_their_opacity`).
- Nothing is sent while AI is off, not set up, or without a prompt
  (`ai_image::tests::nothing_is_sent_while_ai_is_off_or_not_set_up`).
- Pictures are placed centred, one request each with Gemini, one undo
  step each (`ai_image::tests::created_pictures_go_on_the_page_one_undo_step_each`);
  vectors replace the picture with a traced group
  (`ai_image::tests::vectors_are_traced_from_the_picture`); Remix and
  Remove Background work on the selected bitmap and undo
  (`ai_image::tests::remix_and_remove_background_work_on_the_selected_bitmap`);
  service and network errors are shown
  (`ai_image::tests::errors_of_the_service_are_shown_and_nothing_is_placed`).
- Aspect ratios match the reference picture; unreadable files are
  refused (`ai_image::tests::aspect_ratios_match_the_reference_picture`).
- An example image goes with the prompt to Gemini and is edited by the
  OpenAI API (`ai_image::tests::an_example_image_goes_with_the_prompt`).
- Results replace the last ones and can stay out of the drawing
  (`ai_image::tests::results_replace_the_last_ones_and_can_stay_out_of_the_drawing`).
- Pictures come from the one service when it can make them, Gemini's
  at its own API, and the greyed commands say why
  (`ai_image::tests::pictures_come_from_the_one_service_when_it_can_make_them`).
- SVG vectors come from the service's text model as one fitted group of
  shapes and undo (`ai_image::tests::svg_vectors_come_from_the_drawing_service_as_shapes`).
- The docker draws in every mode and state
  (`ui::ai_generate_docker::tests::the_docker_draws_in_every_mode_and_state`).

## AI Art Styles

Bitmaps > AI Art Styles repaints the selection (a bitmap as it is,
other objects drawn as a picture first) in one of eight styles: Oil
Painting, Watercolour, Pencil Sketch, Ink Drawing, Chalk Pastel, Pop
Art, Woodcut and Stained Glass. Each is a Remix with a fixed request
that names the style and asks to keep the picture's subject and
composition; the AI Generate docker opens in Remix with that request,
no style or palette preset, and shows the results, which are placed
like any Remix result. Greyed out until the AI service makes pictures
(the hover tip says why), or while a request is under way.

## Draw with words

The fourth mode, Draw with words, holds what used to be the separate AI
Drawing panel: a request in words makes or changes objects of the
drawing through the AI service's text model (see `ai-drawing.md`). Drawings
whose docker column had AI Drawing open show AI Generate instead.
- Sizes and prices follow the model
  (`ai_image::tests::sizes_and_prices_follow_the_model`); larger sizes
  are named for Gemini and given in pixels for GPT Image 2.5
  (`ai_image::tests::larger_sizes_are_asked_for_by_name_or_in_pixels`);
  retired models are replaced only at the service's own address and the
  offered models are current
  (`ai_image::tests::retired_models_are_replaced_only_at_the_services_own_address`).

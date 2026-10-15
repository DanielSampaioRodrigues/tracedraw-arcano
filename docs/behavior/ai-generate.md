# AI Generate and Remove Background

Pictures made, remixed or cut out by an image service the user sets up
with their own API key (Tools > Options > AI > Image service).
TraceDraw has no AI of its own and no account, and nothing is sent until
the user asks. The work is in `apps/tracedraw/src/ai_image.rs`.

## Where

- Tools > AI Generate, the AI Generate button on the standard toolbar
  (its arrow lists AI Generate, AI Drawing, Remove Background and
  Options > AI) and Window > Dockers open the AI Generate docker.
- Bitmaps > Remove Background, the Remove Background button on the
  property bar of a selected bitmap, and Remove Background and Remix with
  AI at the top of a bitmap's context menu.
- Every AI command is greyed out while AI features are off or the image
  service is not set up, and the docker shows a banner saying so with a
  Set Up AI button that opens the right options page.

## Services

| Service | Address (default) | Model (default) | Key |
|---|---|---|---|
| Google Gemini API | `https://generativelanguage.googleapis.com/v1beta` | `gemini-2.5-flash-image` (also offered: `gemini-3-pro-image-preview`) | `x-goog-api-key` |
| OpenAI Images API, or a service with the same API | `https://api.openai.com/v1` | `gpt-image-1` (also `gpt-image-1-mini`, `dall-e-3`) | `Authorization: Bearer`; optional at an address of the user's |

- Gemini: `POST {address}/models/{model}:generateContent` with the
  prompt (and the picture as `inline_data` for Remix and Remove
  Background), `responseModalities` TEXT and IMAGE and the aspect ratio
  in `imageConfig`. One picture per request: asking for 3 sends 3
  requests. Pictures come back in `inlineData`; a refusal
  (`promptFeedback.blockReason`) or text without a picture is shown.
- OpenAI: `POST {address}/images/generations` (JSON: model, prompt, `n`
  up to 4, size 1024 x 1024, 1536 x 1024 or 1024 x 1536; 1792 x 1024 or
  1024 x 1792 for `dall-e-3`, which also gets `response_format
  b64_json`) and `POST {address}/images/edits` (multipart: model, prompt,
  `image`; `background transparent` and `output_format png` for Remove
  Background). Pictures come back as `b64_json`; a reply with links only
  asks for another model.
- Test Connection lists the service's image models (`GET
  {address}/models`, names with "image" or starting with "dall-e"); they
  then fill the model lists.
- Replies up to 64 MB are read; PNG, JPEG and WebP pictures are taken
  and stored as PNG.

## The docker

Three modes, as buttons at the top:

- Create image: model, prompt (with Clear and Inspire me, which fills
  in an idea), aspect ratio (Square 1:1, Landscape 4:3 and 3:2, Wide
  16:9, Portrait 3:4 and 2:3, Tall 9:16), style (Photorealistic, Line
  art, Watercolor, Illustration, 3D render, Pop art; a second click turns
  it off), color palette (Full spectrum, Grayscale, Warm, Cool, Pastel,
  Neon, Earth, Blues) and the number of images (1 to 4). Each picture is
  placed centred on the page, as large as four fifths of the page allows,
  as its own undo step ("AI Generate"), named "AI picture" and selected.
- Remix image: the selected bitmap (shown as the source), the prompt
  describing the changes, style and palette. The new picture goes over
  the source at its place and height, with its own proportions.
- Create vectors: a prompt (with palette and aspect ratio) asked for as a
  flat illustration on white, then traced with the Clipart preset into a
  group of curves that replaces the picture.

The style and palette add a sentence to the prompt; prompts are cut at
4000 characters. While waiting the docker shows a spinner, how many
answers are in and Cancel (answers that come later are dropped). The
pictures made this session are listed under Results (newest first, up to
12); a click places one on the page again.

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
picture, go to the image service set up, when the user presses Generate
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
- The docker draws in every mode and state
  (`ui::ai_generate_docker::tests::the_docker_draws_in_every_mode_and_state`).

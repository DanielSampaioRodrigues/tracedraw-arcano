# Tools > Options > AI

A dialog of its own (Tools > Options > AI..., also the last row of the
AI Generate button's list) with three pages.

## General

- Turn on AI features (on by default). Off greys out every AI command:
  AI Drawing's Draw and prompt bar, AI Generate, Remix, Create vectors
  and Remove Background.
- Services: whether the drawing service and the image service are ready
  (green "Ready", or "Not set up").
- Features: the AI commands, greyed while their service is not ready.
- What is sent: only what a command needs, to the service set up, when
  the user runs it.

## Drawing service

The text service of AI Drawing (see `ai-drawing.md`): service, address,
model, key, Show the prompt bar, Remember the key. Test Connection asks
the service for its models and says how many it has, or shows its error.

## Image service

The service of AI Generate and Remove Background (see `ai-generate.md`):
Google Gemini API or OpenAI Images API (or compatible), address, model
(a list of the usual models, or the service's own once tested), key and
Remember the key. With an address of the user's no key is needed. Test
Connection as above.

## Banner

While a service a part needs is not ready, the AI Drawing and AI
Generate dockers (and these pages while AI is off) show a banner: "AI
features", what is missing (AI turned off; or a service with the user's
own API key) and Set Up AI..., which opens the page that sets it up.

## Checks

- The banner's message follows the switch and each service, and turning
  AI off makes both services not ready
  (`ui::ai_ui::tests::the_banner_shows_until_the_service_is_set_up`).
- The pages and the banner draw in every state with both image services
  (`ui::ai_ui::tests::the_pages_and_the_banner_draw_in_every_state`);
  every options page draws (`ui::options::tests::every_page_draws`).
- The image key is written to the preferences only when it is to be
  remembered; older preference files turn AI features on
  (`ui::ai_ui::tests::the_image_key_is_kept_only_when_asked`).

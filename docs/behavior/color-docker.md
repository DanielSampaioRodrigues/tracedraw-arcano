# Color docker

Window > Panels > Color, Shift+F11, and Outline Colour (Shift+F12).

## Layout

- The top row: three picture buttons that show or hide the views (Color
  Viewers and Color Sliders shown, Color Palettes hidden at first; any
  mix can show), then the model of the sliders: RGB, CMYK (the default),
  HSB, HSL, Lab, Grayscale or YIQ.
- Under it: the selected object's fill colour (a red diagonal when it
  has no solid fill) beside the new colour, the new colour's `#rrggbb`
  (typed values apply when the field is left), the gamut warning when
  the new colour falls outside the CMYK gamut (more than 6 delta E on
  the way there and back), and the eyedropper button, which picks the
  Color Eyedropper tool; after a sample, "Use sampled #rrggbb" takes it.
- Color viewer: a square with saturation from left (0) to right (100 %)
  and brightness from bottom (0) to top (100 %) for the current hue, and
  the hue strip beside it (red at the top, then yellow, green, cyan,
  blue, magenta, red). Click or drag in either. A grey keeps the last hue
  shown; picking a hue for a grey takes full saturation and at least
  half brightness so the hue shows.
- Color sliders: one per component of the model with its value: RGB and
  YIQ 0 to 255, CMYK, saturation, brightness, lightness and grey 0 to
  100, hues 0 to 360 degrees, Lab L 0 to 100 and a, b -128 to 127.
- Color palettes: a list of the palettes and the chosen one's swatches
  (16 px, at most 512); a click takes the swatch's colour, the tooltip
  names it.
- At the bottom: Fill and Outline apply the new colour to the selection
  (pale with nothing selected); Add to Palette puts it in the document
  palette.

## Following the selection

When the selection changes, the new colour becomes the first selected
object's solid fill, in the model of the sliders. Changes by hand stay
until the selection changes again.

## Checks

- `ui::color_docker::tests::the_new_color_follows_the_selection`
- `ui::color_docker::tests::hue_sliders_count_degrees`
- `ui::color_docker::tests::the_docker_draws_every_view_and_model`

## The colour picker popup

The swatches of the Properties docker (fill, outline, the extra fills
and outlines, the text's character colours) open a popup beside the
swatch (`ui/color_picker.rs`): the viewer (saturation and brightness
square with the hue strip), a **Model** list and the sliders of the
colour's model. The colour is edited in its own model, a CMYK colour in
CMYK and an RGB one in RGB; choosing another model in the list converts
the colour and the object holds a colour of that model from then on,
with the sliders changing to it. The list offers the docker's models
(RGB, CMYK, HSB, HSL, Lab, Grayscale, YIQ); a spot ink or registration
colour is edited in CMYK until another model is chosen. Esc or a click
outside closes the popup
(`ui::color_picker::tests::the_model_can_be_changed_on_demand`).

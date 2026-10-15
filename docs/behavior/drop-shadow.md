# Drop shadow

| Control | Default | Range |
|---|---|---|
| Offset | 2 mm, -2 mm | any |
| Opacity | 50% | 0..100 |
| Feathering | 15% | 0..100 |
| Colour | black | any |

Not yet: feather direction (inside, middle, outside, average), merge
modes, perspective shadows (fade and stretch) and presets; the shadow is
always flat, feathered on both sides and drawn normally beneath the
object.

Feathering radius in mm `= feather/100 * max(w, h) * 0.2`, rendered as
three box blurs (approximating a Gaussian). The shadow is the object's
silhouette filled with the colour, blurred, drawn beneath the object.

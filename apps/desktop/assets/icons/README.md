# App icon

Flat Fluent blue squircle (`#0078D4`, `rx=200` on 1024 canvas) with the lucide
`Brain` glyph stroked in white at `stroke-width=2.0` (≈50px in 1024 space). No
gradients, no highlights, no shadows. Mirrors the agency-cowork sibling SVG
language and complies with `apps/desktop/DESIGN.md`.

## Files

- `source/icon.svg` — single source of truth (1024×1024)
- `icon.iconset/` — rendered PNG set required by `iconutil`
- `icon.icns` — packed bundle wired into `apps/desktop/forge.config.ts`

## Regenerate

Requires `rsvg-convert` (`brew install librsvg`) and macOS `iconutil`.

```bash
cd apps/desktop/assets/icons
RSVG=/opt/homebrew/bin/rsvg-convert
rm -rf icon.iconset && mkdir icon.iconset
for entry in 16:icon_16x16.png 32:icon_16x16@2x.png 32:icon_32x32.png \
             64:icon_32x32@2x.png 128:icon_128x128.png 256:icon_128x128@2x.png \
             256:icon_256x256.png 512:icon_256x256@2x.png 512:icon_512x512.png \
             1024:icon_512x512@2x.png; do
  px="${entry%%:*}"; name="${entry##*:}"
  $RSVG -w "$px" -h "$px" source/icon.svg -o "icon.iconset/$name"
done
iconutil -c icns icon.iconset -o icon.icns
```

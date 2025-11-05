# Third-Party Licenses

This software uses the following third-party libraries:

## Image Processing (Optional Build-time Dependency)

### sharp and libvips

- **Packages:** `sharp@0.34.4`, `@img/sharp-libvips-darwin-arm64@1.2.3`
- **License:** Apache-2.0 (sharp) / LGPL-3.0-or-later (libvips native binaries)
- **Usage:** Build-time image optimization for Next.js (not distributed to end users)
- **Source:** https://github.com/lovell/sharp
- **Note:** Contains LGPL-licensed native libraries including:
  - libvips (core image processing)
  - fribidi (bidirectional text)
  - glib (core library)
  - libexif (EXIF metadata)
  - libheif (HEIF image format)
  - librsvg (SVG rendering)
  - pango (text layout)
  - proxy-libintl (internationalization)

#### LGPL Compliance

For LGPL-3.0 compliance:
- **Source code for LGPL components:** https://github.com/lovell/sharp-libvips
- **LGPL-3.0 license text:** https://www.gnu.org/licenses/lgpl-3.0.html

The LGPL libraries are used as dynamically linked dependencies during the build process only. They are not included in the distributed website code or served to end users.

---

## Core Dependencies

All other dependencies are licensed under permissive open source licenses:

- **MIT License:** React, Next.js, TypeScript definitions, Tailwind CSS, PostCSS, Headless UI, marked, isomorphic-dompurify, shiki, and various supporting packages
- **Apache-2.0:** TypeScript
- **ISC, BSD-2-Clause, BSD-3-Clause:** Various utility libraries

For a complete list of dependencies, see `package.json` and run:
```bash
npm list --production
```

To generate a detailed license report:
```bash
npx license-checker --production --markdown
```

---

## License Compatibility

All dependencies are compatible with the MIT License under which this website is distributed. The LGPL-3.0 components are used only as build-time tools and do not impose copyleft requirements on the website code itself.

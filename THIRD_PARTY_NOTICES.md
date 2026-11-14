# Third-party notices

## Milaclone

Milano Pen's data model (boards, cards, soft-delete by subtree, deep copy with
id remapping), its Markdown export layout and its video/music embed mapping
are adapted from [Milaclone](https://github.com/ellshep/milaclone), used under
the MIT License:

```
MIT License

Copyright (c) 2026 ellshep

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

Files containing adapted code say so in a comment
(`crates/milano-core/migrations/0001_initial.sql`,
`crates/milano-core/src/services/export.rs`, `src/cards/embeds.ts`,
`src/editors/richText.ts`).

## Dependencies

Rust crates and npm packages keep their own licenses (MIT, Apache-2.0 and
compatible permissive licenses); see `Cargo.lock` and `package-lock.json`.
Icons are from [Lucide](https://lucide.dev) (ISC).

### Bundled fonts

The note fonts Inter, Lora, Playfair Display, Caveat and JetBrains Mono are
bundled from [Fontsource](https://fontsource.org) (`@fontsource/*` packages)
and are licensed under the [SIL Open Font License 1.1](https://openfontlicense.org);
each package's `LICENSE` file names its copyright holders. They are served
from the app bundle and never downloaded at run time.

Milano Pen is an independent project. It is not affiliated with or endorsed by
Milanote, Miro or Milaclone. Milanote is a trademark of its owner and is
mentioned only to describe the kind of tool Milano Pen is.

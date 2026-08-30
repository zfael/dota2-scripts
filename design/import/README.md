# Imported UI design

`D2 Scripts.dc.html` is a verbatim copy of the redesign artboard from the Claude
Design project, pulled down on 2026-08-27:

<https://claude.ai/design/p/9b4a6aa3-ba02-4cf4-b375-9a86ddf2322e?file=D2+Scripts.dc.html>

It is **reference only** — nothing in `src-ui/` imports it. It is kept here so
the next person changing the UI can diff intent against implementation without
re-fetching the project.

The design system it draws against ("Core") lives in the same project under
`_ds/core-00591b48-…/`. Its tokens are ported into
`src-ui/src/styles/global.css` as the `--core-*` custom properties; the Tailwind
colour namespace is mapped onto them in the `@theme inline` block, which is why
pre-existing class names (`bg-surface`, `text-subtle`, `text-gold`, …) kept
working while their values changed.

Deliberate deviations from the artboard, and the app features it does not cover,
are listed in the redesign notes for this change.

# lisage: web interface v1 update

Contents of the archive, to be handed to the Vibe Code agent of the
`saucisson/lisage` repository:

- `web-interface-directive.md`: the complete, self-contained directive for
  updating the lisage web interface (common contract on zones / auth /
  Mistral / UI + work specific to the repository + issues + definition of
  done). To be given as-is.
- `logos/`: the reference files of the lisage logo.
  - light version: white background, black text;
  - dark version: `#0F172A` background, `#F1F5F9` text, the one displayed
    by the theme selector in dark mode;
  - favicon: symbol alone, without text.

Integration: drop the logo files into the static assets of the front (the
location appears in the directive), do not trace or redraw them; the theme
selector switches between the light version and the dark version, the
favicon is deployed as-is. Palette and common rules: see the directive.

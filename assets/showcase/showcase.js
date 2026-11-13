/*  This file is part of the OmniDiff code diffing tool.
 *
 *  Copyright (C) 2026 Marko Ivankovic
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU Affero General Public License as published
 *  by the Free Software Foundation, version 3 of the License.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

// The showcase's half of the page: a `fetch` shim that answers the viewer's `/api/*` calls from
// the JSON `generate_showcase` baked, and the strip of chrome above the viewer that picks a case,
// flips between `diff`'s view and omnidiff's, and picks the theme. app.js and model.js are the
// viewer under assets/viewer/, loaded unchanged. Switching a case or a view rewrites what the shim
// will answer and then presses the viewer's own `r` (reload), which makes it ask again; the theme
// goes to the viewer as an `omnidiff:set-theme` event, and comes back from its `c` dialog through
// `/api/settings`.
//
// A case is a code change or a change to other content (a PDF, a font, an archive, a picture). The
// viewer is the code viewer, so a content case is drawn here instead, over the viewer's panels:
// `diff`'s line on the pair, the TUI's content viewer as the generator drew it offscreen - styled
// cells with its pictures laid over them, one still per step and theme - and last omnidiff's
// headless report.
//
// Loaded before app.js, whose `init()` runs at load and asks `/api/state` first - so the shim
// must be installed synchronously here, and it is; the data it answers with is fetched lazily
// behind a promise the first `/api/` call awaits.
//
// The pure parts (URL <-> selection, the API routing table, what a content case shows) are
// exported for showcase.test.js, run by `make test-showcase-js`; the DOM wiring is not.
"use strict";

const OmniDiffShowcase = (() => {
  const GROUPS = [
    { id: "diff_wrong", label: "Code: where diff gets it wrong and OmniDiff is exact" },
    { id: "both_right", label: "Code: where both are right" },
    { id: "content", label: "Other content: PDFs, fonts, archives, pictures" },
  ];
  const VIEWS = ["diff", "omnidiff"];
  const VIEW_LABEL = { diff: "Unix diff", omnidiff: "OmniDiff" };

  // ----- URL <-> selection ------------------------------------------------------------------

  // The case a visitor lands on without a `?case=` (the README's link): two loops replaced by
  // built-ins, where `diff` marks whole blocks and OmniDiff shows what each line became.
  const LANDING_CASE = "python-refactoring";

  // LANDING_CASE, or the first case if the site has no such case.
  function landingCase(cases) {
    if (cases.some((c) => c.name === LANDING_CASE)) return LANDING_CASE;
    return cases.length ? cases[0].name : null;
  }

  function isContent(entry) {
    return Boolean(entry) && entry.group === "content";
  }

  // `?case=<name>&view=diff|omnidiff&theme=<id>&step=<n>`. Anything missing or unknown falls
  // back: to the landing case, to `diff` - the view a newcomer already knows, so the flip to
  // OmniDiff is the reveal - to no theme (the site's default) and to the first step.
  function parseQuery(search, cases, themes) {
    const params = new URLSearchParams(search || "");
    const wanted = params.get("case");
    const known = cases.find((c) => c.name === wanted);
    const view = params.get("view");
    const theme = params.get("theme");
    const step = Number.parseInt(params.get("step"), 10);
    return {
      name: known ? known.name : landingCase(cases),
      view: VIEWS.includes(view) ? view : "diff",
      theme: (themes || []).some((t) => t.id === theme) ? theme : null,
      step: Number.isInteger(step) && step > 0 ? step : 0,
    };
  }

  // The theme and the step are named only when they are not the default, so a link to a code
  // change stays as short as it was.
  function buildQuery(name, view, theme, step) {
    const params = new URLSearchParams();
    params.set("case", name);
    params.set("view", view);
    if (theme) params.set("theme", theme);
    if (step) params.set("step", String(step));
    return `?${params.toString()}`;
  }

  // ----- the API the page thinks it is talking to ------------------------------------------

  // Which baked payload answers for a (view, render options) pair. `diff`'s view has one
  // rendering; omnidiff's has the default and the two presets the `M` panel can pick, and any
  // other combination of options gets the default rather than a recomputation nobody can run.
  function payloadFor(view, payloads, presets, options) {
    if (view === "diff") return payloads.diff;
    if (options && presets) {
      if (sameOptions(options, presets.minimal)) return payloads.minimal;
      if (sameOptions(options, presets.full)) return payloads.full;
    }
    return payloads.omnidiff;
  }

  function sameOptions(a, b) {
    const keys = new Set([...Object.keys(a), ...Object.keys(b)]);
    for (const key of keys) if (a[key] !== b[key]) return false;
    return true;
  }

  // `payload` highlighted in syntax theme `name`: the baked spans of that theme, or the payload
  // as it is when the theme was not baked (or none is chosen, which is the payload's own).
  function themed(payload, syntax, name) {
    const spans = name && syntax ? syntax[name] : null;
    if (!spans) return payload;
    return {
      ...payload,
      before: { ...payload.before, spans: spans.before },
      after: { ...payload.after, spans: spans.after },
      syntax: spans.syntax,
    };
  }

  // What `/api/diff` answers while a content case is shown: an empty pair, so the viewer under
  // the content view holds nothing, coloured as the syntax theme colours the page.
  function emptyPayload(entry, presets, colors) {
    const side = (prefix) => ({
      path: `${prefix}/${entry.path}`,
      name: entry.path.split("/").pop(),
      language: "",
      lines: [""],
      ranges: [],
      spans: [[]],
    });
    return {
      before: side("a"),
      after: side("b"),
      summary: null,
      change_counts: { insertions: 0, deletions: 0, updates: 0, moves: 0 },
      plain_text_fallback: false,
      large_residual: false,
      render_options: presets ? presets.full : {},
      syntax: colors,
    };
  }

  // The page colours of syntax theme `name`, from themes.json; the viewer's own dark defaults
  // for a name it does not list.
  function syntaxColors(themes, name) {
    const found = ((themes && themes.syntax) || []).find((s) => s.theme === name);
    return found || { theme: name || "", background: "#1e1e1e", foreground: "#d4d4d4" };
  }

  // One request in, one `{status, json}` out. `ctx` is what the page has selected and loaded:
  // `state` (state.json), `themes` (themes.json), `entry` (the selected case's entry of
  // cases.json), `view`, `payloads` and `syntax` (the selected code case's JSONs), `presets`
  // (from state.json), `options` (the last render options the page sent, if any), and `theme`
  // and `syntaxTheme` (null until one is chosen).
  function routeApi(path, body, ctx) {
    const ok = (json) => ({ status: 200, json });
    const refuse = (message) => ({ status: 400, json: { error: message } });
    const syntaxName = ctx.syntaxTheme || (ctx.state && ctx.state.default_syntax_theme);
    const current = (options, name) =>
      isContent(ctx.entry)
        ? emptyPayload(ctx.entry, ctx.presets, syntaxColors(ctx.themes, name || syntaxName))
        : themed(payloadFor(ctx.view, ctx.payloads, ctx.presets, options), ctx.syntax, name || ctx.syntaxTheme);
    switch (path) {
      case "/api/state": {
        const settings = { ...ctx.state.settings };
        if (ctx.theme) settings.theme = ctx.theme;
        if (ctx.syntaxTheme) settings.syntax_theme = ctx.syntaxTheme;
        return ok({ ...ctx.state, settings, before: "before", after: "after" });
      }
      case "/api/diff":
        return ok(current(ctx.options));
      case "/api/render_options": {
        ctx.options = body;
        // The page shows what it asked for in its footer badge; on `diff`'s view every option
        // paints the same whole lines, so echoing the request back is honest there too.
        return ok({ ...current(body), render_options: body });
      }
      case "/api/highlight": {
        // A preview: the theme becomes the page's only when `/api/settings` saves it.
        const name = body && body.syntax_theme;
        const payload = current(ctx.options, name);
        // A theme the site was not baked with gets the same spans under its own name, which
        // keeps the page consistent rather than half-recoloured.
        return ok({
          before: payload.before.spans,
          after: payload.after.spans,
          syntax: { ...payload.syntax, theme: name || payload.syntax.theme },
        });
      }
      case "/api/settings":
        if (body && body.theme) ctx.theme = body.theme;
        if (body && body.syntax_theme) ctx.syntaxTheme = body.syntax_theme;
        return ok({});
      case "/api/cancel":
        return ok({});
      case "/api/quit":
        return refuse("This page never quits. Pick another change above.");
      case "/api/edit":
        return refuse("Editing is not available in the showcase.");
      case "/api/ls":
      case "/api/review":
      case "/api/review/open":
        return refuse("Only the changes listed above can be opened here. Install omnidiff to diff your own.");
      default:
        return { status: 404, json: { error: `no such endpoint: ${path}` } };
    }
  }

  // The first row either side paints, or Infinity when nothing is painted - what decides whether
  // a freshly loaded diff opens on line 1 or jumps to its first change.
  function firstPaintedRow(payload) {
    let first = Infinity;
    for (const side of [payload.before, payload.after]) {
      for (const range of (side && side.ranges) || []) {
        if (range.op !== "identical" && range.op !== "unset") first = Math.min(first, range.source[0]);
      }
    }
    return first;
  }

  // ----- a content case ---------------------------------------------------------------------

  // What a content case shows under `view`, in order: `{ caption, command, output }` for a
  // terminal's text, `{ caption, command, still }` for the viewer's still number `still`.
  // `diff` has one screen, its own line; omnidiff has the viewer's steps and then its report.
  function contentScreens(entry, view) {
    const name = entry.path.split("/").pop();
    const pair = `a/${name} b/${name}`;
    if (view === "diff") {
      return [{ caption: "All that diff says", command: `diff ${pair}`, output: entry.diff_output }];
    }
    return [
      ...entry.steps.map((caption, still) => ({ caption, command: `omnidiff ${pair}`, still })),
      { caption: "OmniDiff's report, as git diff and a pipe get it", command: `omnidiff --headless ${pair}`, output: entry.report },
    ];
  }

  // The theme whose stills a content case shows: the chosen one if the site baked it, else the
  // site's default (the user's own Custom palette has no stills).
  function stillTheme(theme, themes, fallback) {
    const baked = ((themes && themes.themes) || []).map((t) => t.id);
    return baked.includes(theme) ? theme : fallback;
  }

  // Where a still's picture goes, in pixels, for cells `cellWidth` by `rowHeight`: scaled to fit
  // the cells it covers, keeping its shape, from their top left, as a terminal that speaks a
  // graphics protocol scales it. Enlarged pictures keep hard pixels.
  function pictureBox(picture, cellWidth, rowHeight) {
    const boxWidth = picture.cols * cellWidth;
    const boxHeight = picture.rows * rowHeight;
    const scale = Math.min(boxWidth / picture.width, boxHeight / picture.height);
    return {
      left: picture.col * cellWidth,
      top: picture.row * rowHeight,
      width: Math.max(1, Math.round(picture.width * scale)),
      height: Math.max(1, Math.round(picture.height * scale)),
      pixelated: scale >= 1,
    };
  }

  // ----- the strip's text -------------------------------------------------------------------

  function plural(n, word) {
    return `${n} ${word}${n === 1 ? "" : "s"}`;
  }

  function formatCounts(counts) {
    const parts = [];
    if (counts.updates) parts.push(plural(counts.updates, "update"));
    if (counts.moves) parts.push(plural(counts.moves, "move"));
    if (counts.insertions) parts.push(plural(counts.insertions, "insertion"));
    if (counts.deletions) parts.push(plural(counts.deletions, "deletion"));
    return parts.length ? parts.join(", ") : "nothing";
  }

  function statsLine(entry) {
    if (isContent(entry)) {
      const human = entry.human_verdict || "not judged yet";
      return `Unix diff says only that the files differ · OmniDiff calls the change ${entry.verdict}; a human, ${human}`;
    }
    const summary = entry.summary ? ` (${entry.summary})` : "";
    return (
      `Unix diff marks ${plural(entry.diff_marked, "line")} · ` +
      `OmniDiff marks ${formatCounts(entry.omnidiff)}${summary}`
    );
  }

  // What the picker calls a case.
  function optionLabel(entry) {
    return isContent(entry) ? entry.title : `${entry.title} (${entry.language})`;
  }

  return {
    GROUPS,
    VIEWS,
    VIEW_LABEL,
    parseQuery,
    LANDING_CASE,
    buildQuery,
    isContent,
    payloadFor,
    themed,
    emptyPayload,
    syntaxColors,
    routeApi,
    firstPaintedRow,
    contentScreens,
    stillTheme,
    pictureBox,
    plural,
    formatCounts,
    statsLine,
    optionLabel,
  };
})();

if (typeof module !== "undefined") {
  module.exports = OmniDiffShowcase;
} else {
  (() => {
    const S = OmniDiffShowcase;
    const $ = (selector) => document.querySelector(selector);
    // Rows visible without scrolling on a typical window, roughly; a first change past this many
    // rows is one the reader would otherwise not see on load.
    const FOLD_ROWS = 14;
    const realFetch = window.fetch.bind(window);

    const ctx = {
      state: null,
      themes: null,
      entry: null,
      view: "diff",
      payloads: null,
      syntax: null,
      presets: null,
      options: null,
      theme: null,
      syntaxTheme: null,
    };
    let cases = [];
    let selected = null;
    // Which of a content case's screens is shown.
    let step = 0;
    const cache = new Map();
    const stills = new Map();
    // Whether the next diff the page loads should land on its first change rather than on line
    // 1: yes on the first load and on a change of case, no on a flip between views, where the
    // viewer restores the reader's own cursor and that is the point of flipping.
    let jumpToFirstChange = true;
    // True while this file presses one of the viewer's keys for the reader (`press`), so that
    // its own filter (`onKey`) lets the key through.
    let pressing = false;

    // One of the viewer's own keys, pressed for the reader.
    function press(key) {
      pressing = true;
      try {
        document.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
      } finally {
        pressing = false;
      }
    }

    const fetchJson = (path) => realFetch(path).then((r) => r.json());

    const ready = Promise.all([fetchJson("cases.json"), fetchJson("state.json"), fetchJson("themes.json")]).then(
      ([list, state, themes]) => {
        cases = list;
        ctx.state = state;
        ctx.themes = themes;
        ctx.presets = state.presets;
        const initial = S.parseQuery(window.location.search, cases, themes.themes);
        selected = initial.name;
        ctx.entry = cases.find((c) => c.name === selected) || null;
        ctx.view = initial.view;
        step = initial.step;
        if (initial.theme) {
          ctx.theme = initial.theme;
          ctx.syntaxTheme = themes.themes.find((t) => t.id === initial.theme).syntax_theme;
        }
        buildChrome();
        showSelection();
      },
    );

    const EMPTY = { payloads: null, syntax: null };

    async function loadCase(name) {
      const entry = cases.find((c) => c.name === name);
      if (S.isContent(entry)) return EMPTY;
      if (!cache.has(name)) {
        const load = (suffix) => fetchJson(`cases/${name}.${suffix}.json`);
        cache.set(
          name,
          Promise.all([load("diff"), load("omnidiff"), load("minimal"), load("full"), load("syntax")]).then(
            ([diff, omnidiff, minimal, full, syntax]) => ({ payloads: { diff, omnidiff, minimal, full }, syntax }),
          ),
        );
      }
      return cache.get(name);
    }

    // The shim. Anything outside /api/ (the JSON files themselves) goes to the network as usual.
    window.fetch = async (path, init) => {
      if (typeof path !== "string" || !path.startsWith("/api/")) return realFetch(path, init);
      await ready;
      const loaded = await loadCase(selected);
      ctx.payloads = loaded.payloads;
      ctx.syntax = loaded.syntax;
      let body = {};
      try {
        body = init && init.body ? JSON.parse(init.body) : {};
      } catch (_) {
        body = {};
      }
      const { status, json } = S.routeApi(path, body, ctx);
      if (path === "/api/diff") {
        const jump = jumpToFirstChange && S.firstPaintedRow(json) >= FOLD_ROWS && S.firstPaintedRow(json) !== Infinity;
        jumpToFirstChange = false;
        setTimeout(() => {
          updateTitle();
          // The viewer's own "next change" from line 1, once the payload has been painted:
          // a change forty lines down would otherwise open on an unchanged screen.
          if (jump) press("n");
        }, 150);
      }
      // The viewer's `c` dialog saved a theme: the strip and a content case follow it.
      if (path === "/api/settings" && (body.theme || body.syntax_theme)) showSelection();
      return new Response(JSON.stringify(json), {
        status,
        headers: { "Content-Type": "application/json" },
      });
    };

    // ----- chrome -----------------------------------------------------------------------------

    function buildChrome() {
      const select = $("#case-select");
      select.innerHTML = "";
      for (const group of S.GROUPS) {
        const entries = cases.filter((c) => c.group === group.id);
        if (!entries.length) continue;
        const optgroup = document.createElement("optgroup");
        optgroup.label = group.label;
        for (const entry of entries) {
          const option = document.createElement("option");
          option.value = entry.name;
          option.textContent = S.optionLabel(entry);
          optgroup.append(option);
        }
        select.append(optgroup);
      }
      select.addEventListener("change", () => select_(select.value, ctx.view));
      $("#view-diff").addEventListener("click", () => select_(selected, "diff"));
      $("#view-omnidiff").addEventListener("click", () => select_(selected, "omnidiff"));

      const themeSelect = $("#theme-select");
      themeSelect.innerHTML = "";
      for (const theme of ctx.themes.themes) {
        const option = document.createElement("option");
        option.value = theme.id;
        option.textContent = theme.label;
        themeSelect.append(option);
      }
      themeSelect.addEventListener("change", () => setTheme(themeSelect.value));

      $("#step-previous").addEventListener("click", () => setStep(step - 1));
      $("#step-next").addEventListener("click", () => setStep(step + 1));
      // The viewer's own `M`, for a touch screen with no keyboard to press it on.
      $("#diff-options").addEventListener("click", () => press("M"));
      document.addEventListener("keydown", onKey, true);
      window.addEventListener("resize", () => {
        if (S.isContent(ctx.entry)) showContent();
      });
    }

    // Before the viewer sees a key. `q` and Escape quit the real viewer; here they would leave
    // the reader on a dead screen, so at the top level (no dialog open) they are swallowed, and
    // inside a dialog they still close it, which is app.js's own handling. Over a content case
    // the viewer underneath holds nothing: the arrows step through the case's screens, `c` and
    // `?` still open the viewer's theme dialog and help, and nothing else reaches it.
    function onKey(event) {
      const dialogOpen = !$("#overlay").hidden;
      if (pressing || dialogOpen || isTyping(event) || event.ctrlKey || event.metaKey || event.altKey) return;
      const swallow = () => {
        event.stopImmediatePropagation();
        event.preventDefault();
      };
      if (event.key === "q" || event.key === "Escape") return swallow();
      if (!S.isContent(ctx.entry)) return;
      if (event.key === "c" || event.key === "?") return;
      const forward = ["ArrowRight", "ArrowDown", "n", "j", "l", " "];
      const back = ["ArrowLeft", "ArrowUp", "p", "k", "h"];
      if (forward.includes(event.key)) {
        setStep(step + 1);
        swallow();
      } else if (back.includes(event.key)) {
        setStep(step - 1);
        swallow();
      } else {
        event.stopImmediatePropagation();
      }
    }

    function isTyping(event) {
      const tag = event.target && event.target.tagName;
      return tag === "INPUT" || tag === "SELECT" || tag === "TEXTAREA";
    }

    function currentTheme() {
      return ctx.theme || ctx.state.settings.theme;
    }

    function setTheme(id) {
      const theme = ctx.themes.themes.find((t) => t.id === id);
      if (!theme) return;
      ctx.theme = theme.id;
      ctx.syntaxTheme = theme.syntax_theme;
      $("#theme-select").blur();
      document.dispatchEvent(
        new CustomEvent("omnidiff:set-theme", { detail: { theme: theme.id, syntax_theme: theme.syntax_theme } }),
      );
      showSelection();
    }

    function setStep(wanted) {
      const screens = S.contentScreens(ctx.entry, ctx.view);
      const clamped = Math.min(Math.max(wanted, 0), screens.length - 1);
      if (clamped === step) return;
      step = clamped;
      showSelection();
    }

    function select_(name, view) {
      const changed = name !== selected || view !== ctx.view;
      if (name !== selected) jumpToFirstChange = true;
      if (changed) step = 0;
      selected = name;
      ctx.entry = cases.find((c) => c.name === selected) || null;
      ctx.view = view;
      ctx.options = null;
      showSelection();
      if (!changed) return;
      // The viewer's own reload: it asks `/api/diff` again, and the shim now answers differently.
      // Focus leaves the select first so the keystroke reaches the document, not the control.
      $("#case-select").blur();
      press("r");
    }

    function showSelection() {
      const entry = ctx.entry;
      if (!entry) return;
      const content = S.isContent(entry);
      const screens = content ? S.contentScreens(entry, ctx.view) : [];
      if (content) step = Math.min(step, screens.length - 1);
      window.history.replaceState(null, "", S.buildQuery(selected, ctx.view, ctx.theme, content ? step : 0));
      $("#case-select").value = selected;
      for (const view of S.VIEWS) {
        const button = $(`#view-${view}`);
        const active = view === ctx.view;
        button.classList.toggle("active", active);
        button.setAttribute("aria-pressed", String(active));
      }
      showTheme();
      $("#case-stats").textContent = S.statsLine(entry);
      const links = $("#case-links");
      links.innerHTML = "";
      if (content) {
        const path = document.createElement("code");
        path.textContent = entry.path;
        links.append(path, " · ");
      }
      if (entry.upstream) {
        const commit = document.createElement("a");
        commit.href = entry.upstream;
        commit.textContent = "Upstream commit";
        links.append(commit);
      } else {
        links.append("Hand-written example");
      }
      links.append(" · ");
      document.body.classList.toggle("content-case", content);
      $("#content").hidden = !content;
      $("#steps").hidden = !content || screens.length < 2;
      $("#hint-code").hidden = content;
      $("#hint-content").hidden = !content;
      if (content) showContent();
      updateTitle();
    }

    // The picker shows the theme in use. A theme it does not list - the `c` dialog's Custom -
    // gets an entry of its own while it is the one in use.
    function showTheme() {
      const select = $("#theme-select");
      const id = currentTheme();
      for (const option of [...select.options]) if (option.dataset.extra) option.remove();
      if (![...select.options].some((option) => option.value === id)) {
        const option = document.createElement("option");
        option.value = id;
        option.textContent = id;
        option.dataset.extra = "1";
        select.append(option);
      }
      select.value = id;
    }

    function updateTitle() {
      const entry = ctx.entry;
      if (entry) document.title = `${entry.title} · ${S.VIEW_LABEL[ctx.view]} · OmniDiff examples`;
    }

    // ----- a content case ---------------------------------------------------------------------

    function loadStill(name, theme, index) {
      const key = `${name}/${theme}/${index}`;
      if (!stills.has(key)) stills.set(key, fetchJson(`content/${key}.json`));
      return stills.get(key);
    }

    // The size of one cell of the viewer's font, measured on the viewer's own probe.
    function cell() {
      const probe = $("#measure");
      return {
        width: probe.getBoundingClientRect().width / probe.textContent.length,
        height: parseFloat(getComputedStyle(probe).lineHeight) || 18,
      };
    }

    async function showContent() {
      const entry = ctx.entry;
      const view = ctx.view;
      const shown = step;
      const screens = S.contentScreens(entry, view);
      const screen = screens[shown];
      $("#step-count").textContent = `${step + 1}/${screens.length}`;
      $("#step-caption").textContent = screen.caption;
      $("#step-previous").disabled = step === 0;
      $("#step-next").disabled = step === screens.length - 1;

      const terminal = document.createElement("div");
      terminal.className = "terminal";
      if (screen.output !== undefined) {
        const prompt = document.createElement("div");
        prompt.className = "terminal-row";
        const sign = document.createElement("span");
        sign.className = "terminal-prompt";
        sign.textContent = "$ ";
        prompt.append(sign, screen.command);
        const output = document.createElement("div");
        output.className = "terminal-output";
        output.textContent = screen.output;
        terminal.append(prompt, output);
      } else {
        const theme = S.stillTheme(currentTheme(), ctx.themes, ctx.state.settings.theme);
        const still = await loadStill(entry.name, theme, screen.still);
        // A newer selection overtook this one while its still was on the way.
        if (ctx.entry !== entry || ctx.view !== view || step !== shown) return;
        const size = cell();
        terminal.classList.add("still");
        terminal.style.width = `${still.cols * size.width}px`;
        terminal.style.height = `${still.rows * size.height}px`;
        for (const runs of still.lines) {
          const row = document.createElement("div");
          row.className = "terminal-row";
          for (const run of runs) {
            const span = document.createElement("span");
            span.textContent = run.text;
            // A run is as wide as its cells whatever the font makes of its glyphs: a box-drawing
            // character from a fallback font must not push the rest of the row along.
            span.style.width = `${run.cells * size.width}px`;
            if (run.fg) span.style.color = run.fg;
            if (run.bg) span.style.background = run.bg;
            if (run.bold) span.style.fontWeight = "bold";
            if (run.dim) span.style.opacity = "0.6";
            if (run.italic) span.style.fontStyle = "italic";
            if (run.underline) span.style.textDecoration = "underline";
            row.append(span);
          }
          terminal.append(row);
        }
        for (const picture of still.pictures) {
          const box = S.pictureBox(picture, size.width, size.height);
          const image = document.createElement("img");
          image.src = `content/${entry.name}/${picture.file}`;
          image.alt = "";
          image.style.left = `${box.left}px`;
          image.style.top = `${box.top}px`;
          image.style.width = `${box.width}px`;
          image.style.height = `${box.height}px`;
          if (box.pixelated) image.classList.add("pixelated");
          terminal.append(image);
        }
      }
      const host = $("#content");
      host.innerHTML = "";
      host.append(terminal);
    }
  })();
}

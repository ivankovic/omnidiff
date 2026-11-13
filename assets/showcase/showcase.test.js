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

// Run with `node assets/showcase/showcase.test.js` (`make test-showcase-js`). Plain Node, no
// framework, like the mapping site's and the viewer's own tests: the DOM wiring is not exercised,
// the parts that decide what the page shows are.
"use strict";
const assert = require("node:assert/strict");
const S = require("./showcase.js");

const cases = [
  { name: "a", group: "diff_wrong", title: "A", lines: 10, diff_marked: 4, omnidiff: { updates: 1, moves: 0, insertions: 0, deletions: 0 }, summary: null },
  { name: "b", group: "both_right", title: "B", lines: 3, diff_marked: 1, omnidiff: { updates: 0, moves: 1, insertions: 2, deletions: 0 }, summary: "Whitespace changes only" },
];

// ----- parseQuery / buildQuery ----------------------------------------------------------------

const themes = [
  { id: "Dracula", label: "Dracula (default)", syntax_theme: "base16-ocean.dark" },
  { id: "SolarizedLight", label: "Solarized Light", syntax_theme: "Solarized (light)" },
];
const none = { theme: null, step: 0 };

{
  assert.deepEqual(S.parseQuery("", cases), { name: "a", view: "diff", ...none });
  assert.deepEqual(S.parseQuery("?case=b&view=omnidiff", cases), { name: "b", view: "omnidiff", ...none });
  // Unknown case or view: fall back, never leave the page empty.
  assert.deepEqual(S.parseQuery("?case=nope&view=banana", cases), { name: "a", view: "diff", ...none });
  assert.deepEqual(S.parseQuery("?view=omnidiff", cases), { name: "a", view: "omnidiff", ...none });
  assert.deepEqual(S.parseQuery("", []), { name: null, view: "diff", ...none });
  // Without a `?case=`, the landing case when the site has it, not the first in the list.
  const withLanding = [...cases, { ...cases[0], name: S.LANDING_CASE }];
  assert.deepEqual(S.parseQuery("", withLanding), { name: S.LANDING_CASE, view: "diff", ...none });
  assert.deepEqual(S.parseQuery("?case=nope", withLanding), { name: S.LANDING_CASE, view: "diff", ...none });
  assert.deepEqual(S.parseQuery("?case=b", withLanding), { name: "b", view: "diff", ...none });
  assert.equal(S.buildQuery("b", "omnidiff"), "?case=b&view=omnidiff");
  const round = S.parseQuery(S.buildQuery("b", "omnidiff"), cases);
  assert.deepEqual(round, { name: "b", view: "omnidiff", ...none });

  // The theme and the step: named only when chosen, and only a theme the site lists.
  assert.equal(S.buildQuery("b", "omnidiff", "SolarizedLight", 2), "?case=b&view=omnidiff&theme=SolarizedLight&step=2");
  assert.equal(S.buildQuery("b", "omnidiff", null, 0), "?case=b&view=omnidiff");
  assert.deepEqual(S.parseQuery("?case=b&theme=SolarizedLight&step=2", cases, themes), {
    name: "b",
    view: "diff",
    theme: "SolarizedLight",
    step: 2,
  });
  assert.deepEqual(S.parseQuery("?theme=Custom&step=-1", cases, themes), { name: "a", view: "diff", ...none });
  assert.deepEqual(S.parseQuery("?theme=SolarizedLight&step=x", cases), { name: "a", view: "diff", ...none });
}

// ----- payloadFor -----------------------------------------------------------------------------

const presets = { minimal: { x: false, y: false }, full: { x: true, y: true } };
const payloads = {
  diff: { id: "diff", before: { spans: [1] }, after: { spans: [2] }, syntax: { theme: "t" } },
  omnidiff: { id: "omnidiff", before: { spans: [3] }, after: { spans: [4] }, syntax: { theme: "t" } },
  minimal: { id: "minimal" },
  full: { id: "full" },
};

{
  assert.equal(S.payloadFor("diff", payloads, presets, presets.full).id, "diff");
  assert.equal(S.payloadFor("omnidiff", payloads, presets, null).id, "omnidiff");
  assert.equal(S.payloadFor("omnidiff", payloads, presets, { x: false, y: false }).id, "minimal");
  assert.equal(S.payloadFor("omnidiff", payloads, presets, { x: true, y: true }).id, "full");
  // Anything that is not a preset gets the default: there is nothing to recompute with.
  assert.equal(S.payloadFor("omnidiff", payloads, presets, { x: true, y: false }).id, "omnidiff");
}

// ----- routeApi -------------------------------------------------------------------------------

const pageThemes = {
  themes,
  syntax: [{ theme: "Solarized (light)", background: "#fdf6e3", foreground: "#657b83" }],
};
const syntax = {
  "Solarized (light)": { before: [5], after: [6], syntax: { theme: "Solarized (light)", background: "#fdf6e3" } },
};
const pdf = {
  name: "pdf",
  group: "content",
  title: "A PDF",
  family: "documents",
  path: "docs/model.pdf",
  diff_output: "Binary files a/model.pdf and b/model.pdf differ",
  report: "Documents a/model.pdf and b/model.pdf: ...",
  verdict: "edited",
  human_verdict: null,
  steps: ["The metadata", "Page 1"],
};

function ctx(view, entry) {
  return {
    state: { presets, settings: { theme: "Dracula", syntax_theme: null }, default_syntax_theme: "base16-ocean.dark" },
    themes: pageThemes,
    entry: entry || cases[0],
    view,
    payloads,
    syntax,
    presets,
    options: null,
    theme: null,
    syntaxTheme: null,
  };
}

{
  const state = S.routeApi("/api/state", {}, ctx("diff"));
  assert.equal(state.status, 200);
  // A pair is always "open", so the viewer starts diffing on load.
  assert.equal(state.json.before, "before");
  assert.equal(state.json.after, "after");
  assert.equal(state.json.presets, presets);

  assert.equal(S.routeApi("/api/diff", {}, ctx("diff")).json.id, "diff");
  assert.equal(S.routeApi("/api/diff", {}, ctx("omnidiff")).json.id, "omnidiff");

  // Render options are remembered for the next /api/diff and echoed in the answer.
  const c = ctx("omnidiff");
  const filtered = S.routeApi("/api/render_options", presets.full, c);
  assert.equal(filtered.json.id, "full");
  assert.deepEqual(filtered.json.render_options, presets.full);
  assert.equal(S.routeApi("/api/diff", {}, c).json.id, "full");

  // A syntax theme the site was not baked with: the same spans under its name.
  const highlight = S.routeApi("/api/highlight", { syntax_theme: "other" }, ctx("omnidiff"));
  assert.deepEqual(highlight.json.before, [3]);
  assert.deepEqual(highlight.json.after, [4]);
  assert.equal(highlight.json.syntax.theme, "other");
  // One it was baked with: that theme's spans and page colours. A preview only.
  const previewing = ctx("omnidiff");
  const baked = S.routeApi("/api/highlight", { syntax_theme: "Solarized (light)" }, previewing);
  assert.deepEqual(baked.json.before, [5]);
  assert.deepEqual(baked.json.after, [6]);
  assert.equal(baked.json.syntax.background, "#fdf6e3");
  assert.equal(previewing.syntaxTheme, null);
  assert.deepEqual(S.routeApi("/api/diff", {}, previewing).json.before.spans, [3]);

  // Saving a theme: the next state, diff and render options are in it.
  const saved = ctx("omnidiff");
  const answer = S.routeApi("/api/settings", { theme: "SolarizedLight", syntax_theme: "Solarized (light)" }, saved);
  assert.deepEqual(answer, { status: 200, json: {} });
  assert.equal(saved.theme, "SolarizedLight");
  assert.equal(saved.syntaxTheme, "Solarized (light)");
  const settings = S.routeApi("/api/state", {}, saved).json.settings;
  assert.equal(settings.theme, "SolarizedLight");
  assert.equal(settings.syntax_theme, "Solarized (light)");
  const themedDiff = S.routeApi("/api/diff", {}, saved).json;
  assert.equal(themedDiff.id, "omnidiff");
  assert.deepEqual(themedDiff.before.spans, [5]);
  assert.equal(themedDiff.syntax.theme, "Solarized (light)");
  assert.deepEqual(S.routeApi("/api/render_options", presets.full, saved).json.before, { spans: [5] });
  // Settings that name no theme (the layout, the node highlight) change nothing.
  S.routeApi("/api/settings", { layout: "single" }, saved);
  assert.equal(saved.theme, "SolarizedLight");
  // The state before any choice is the site's own.
  assert.equal(S.routeApi("/api/state", {}, ctx("diff")).json.settings.theme, "Dracula");

  // A content case: the viewer under it gets an empty pair, in the syntax theme's page colours.
  const content = ctx("omnidiff", pdf);
  content.payloads = null;
  content.syntax = null;
  const empty = S.routeApi("/api/diff", {}, content).json;
  assert.deepEqual(empty.before.lines, [""]);
  assert.equal(empty.before.path, "a/docs/model.pdf");
  assert.equal(empty.after.name, "model.pdf");
  assert.deepEqual(empty.render_options, presets.full);
  assert.equal(empty.syntax.background, "#1e1e1e");
  content.syntaxTheme = "Solarized (light)";
  assert.equal(S.routeApi("/api/diff", {}, content).json.syntax.background, "#fdf6e3");
  assert.equal(S.routeApi("/api/highlight", { syntax_theme: "Solarized (light)" }, content).json.syntax.foreground, "#657b83");
  assert.equal(S.firstPaintedRow(empty), Infinity);

  for (const path of ["/api/settings", "/api/cancel"]) {
    assert.deepEqual(S.routeApi(path, {}, ctx("diff")), { status: 200, json: {} });
  }
  for (const path of ["/api/quit", "/api/edit", "/api/ls", "/api/review", "/api/review/open"]) {
    const answer = S.routeApi(path, {}, ctx("diff"));
    assert.equal(answer.status, 400, path);
    assert.ok(answer.json.error, path);
  }
  assert.equal(S.routeApi("/api/nothing", {}, ctx("diff")).status, 404);
}

// ----- the strip's text -----------------------------------------------------------------------

{
  assert.equal(S.plural(1, "span"), "1 span");
  assert.equal(S.plural(0, "line"), "0 lines");
  assert.equal(S.formatCounts({ updates: 1, moves: 2, insertions: 0, deletions: 1 }), "1 update, 2 moves, 1 deletion");
  assert.equal(S.formatCounts({ updates: 0, moves: 0, insertions: 0, deletions: 0 }), "nothing");
  assert.equal(S.statsLine(cases[0]), "Unix diff marks 4 lines · OmniDiff marks 1 update");
  assert.equal(
    S.statsLine(cases[1]),
    "Unix diff marks 1 line · OmniDiff marks 1 move, 2 insertions (Whitespace changes only)",
  );
  assert.equal(
    S.firstPaintedRow({
      before: { ranges: [{ op: "identical", source: [0, 0, 9, 0] }, { op: "delete", source: [7, 0, 8, 0] }] },
      after: { ranges: [{ op: "insert", source: [3, 2, 3, 9] }] },
    }),
    3,
  );
  assert.equal(S.firstPaintedRow({ before: { ranges: [] }, after: { ranges: [{ op: "unset", source: [0, 0, 1, 0] }] } }), Infinity);
  assert.deepEqual(S.GROUPS.map((group) => group.id), ["diff_wrong", "both_right", "content"]);
  assert.deepEqual(S.VIEWS, ["diff", "omnidiff"]);
  assert.equal(S.optionLabel({ ...cases[0], language: "Rust" }), "A (Rust)");
  assert.equal(S.optionLabel(pdf), "A PDF");
  assert.equal(
    S.statsLine(pdf),
    "Unix diff says only that the files differ · OmniDiff calls the change edited; a human, not judged yet",
  );
  assert.ok(S.statsLine({ ...pdf, human_verdict: "redrawn" }).endsWith("a human, redrawn"));
}

// ----- a content case -------------------------------------------------------------------------

{
  assert.ok(S.isContent(pdf));
  assert.ok(!S.isContent(cases[0]));
  assert.ok(!S.isContent(null));

  // `diff` has its one line; omnidiff one screen per step of the viewer, then its report.
  const diff = S.contentScreens(pdf, "diff");
  assert.equal(diff.length, 1);
  assert.equal(diff[0].command, "diff a/model.pdf b/model.pdf");
  assert.equal(diff[0].output, pdf.diff_output);
  const omnidiff = S.contentScreens(pdf, "omnidiff");
  assert.equal(omnidiff.length, 3);
  assert.deepEqual(omnidiff[0], { caption: "The metadata", command: "omnidiff a/model.pdf b/model.pdf", still: 0 });
  assert.equal(omnidiff[1].still, 1);
  assert.equal(omnidiff[1].output, undefined);
  assert.equal(omnidiff[2].command, "omnidiff --headless a/model.pdf b/model.pdf");
  assert.equal(omnidiff[2].output, pdf.report);

  // The stills of the theme in use, or of the site's default when it has none (Custom).
  assert.equal(S.stillTheme("SolarizedLight", pageThemes, "Dracula"), "SolarizedLight");
  assert.equal(S.stillTheme("Custom", pageThemes, "Dracula"), "Dracula");

  // A picture is scaled to fit its cells, keeping its shape, from their top left.
  const picture = { col: 30, row: 2, cols: 40, rows: 15, width: 320, height: 540 };
  // 320x270 of cells for a 320x540 picture: height decides, at half size.
  assert.deepEqual(S.pictureBox(picture, 8, 18), { left: 240, top: 36, width: 160, height: 270, pixelated: false });
  // A small picture is enlarged, with hard pixels.
  assert.deepEqual(S.pictureBox({ ...picture, width: 16, height: 16 }, 8, 18), {
    left: 240,
    top: 36,
    width: 270,
    height: 270,
    pixelated: true,
  });

  assert.deepEqual(S.syntaxColors(pageThemes, "Solarized (light)").background, "#fdf6e3");
  assert.equal(S.syntaxColors(pageThemes, "unknown").background, "#1e1e1e");
  assert.deepEqual(S.themed(payloads.omnidiff, syntax, null), payloads.omnidiff);
  assert.deepEqual(S.themed(payloads.omnidiff, syntax, "not baked"), payloads.omnidiff);
}

console.log("showcase.test.js: all assertions passed");

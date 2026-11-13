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

// The half of scripts/showcase_shots/run.py that runs in the container: serves a baked showcase
// (`generate_showcase`'s output) over HTTP, opens it in Playwright's Chromium, and saves one PNG
// per job. Nothing here decides what is on the page: a job names an address of the showcase and
// the part of the page to take.
//
//   node record.js <site directory> <jobs.json> <output directory>
//
// A job is `{ name, query, scale, width, height, selector }` - the element `selector` in a
// `width` by `height` window - or, with `terminal: { cols, rows }` in place of `selector` and
// `height`, a content case's terminal: the window is sized so that a terminal of that many cells
// fits under the page's strip, and the picture is exactly that terminal with its padding, so
// every screen of one case has the same size whatever it holds.
"use strict";

const fs = require("node:fs");
const http = require("node:http");
const path = require("node:path");
const { chromium } = require("playwright");

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json",
  ".png": "image/png",
};

function serve(root) {
  const server = http.createServer((request, response) => {
    const url = new URL(request.url, "http://localhost");
    const file = path.join(root, path.normalize(decodeURIComponent(url.pathname)));
    if (!file.startsWith(root)) {
      response.writeHead(403).end();
      return;
    }
    fs.readFile(file, (error, data) => {
      if (error) {
        response.writeHead(404).end();
        return;
      }
      response.writeHead(200, { "Content-Type": TYPES[path.extname(file)] || "application/octet-stream" });
      response.end(data);
    });
  });
  return new Promise((resolve) => server.listen(0, "127.0.0.1", () => resolve(server)));
}

// The page has painted what its address names: the picker is filled, the viewer has left its
// "diffing" screen, a content case's terminal is on the page with its pictures decoded, and
// nothing has changed for a moment (the shim jumps to the first change after a short delay).
async function settled(page, content) {
  // Attached, not visible: an option of a closed select is never visible.
  await page.waitForSelector("#case-select option", { state: "attached" });
  await page.waitForFunction(() => document.querySelector("#diffing").hidden);
  if (content) {
    await page.waitForSelector("#content .terminal");
    await page.waitForFunction(() => [...document.querySelectorAll("#content img")].every((image) => image.complete));
  }
  await page.waitForLoadState("networkidle");
  await page.waitForTimeout(400);
}

async function main() {
  const [site, jobsFile, out] = process.argv.slice(2);
  if (!site || !jobsFile || !out) {
    console.error("usage: node record.js <site directory> <jobs.json> <output directory>");
    process.exit(2);
  }
  const jobs = JSON.parse(fs.readFileSync(jobsFile, "utf8"));
  const server = await serve(path.resolve(site));
  const base = `http://127.0.0.1:${server.address().port}/index.html`;
  const browser = await chromium.launch();
  let failed = false;
  for (const job of jobs) {
    const context = await browser.newContext({
      viewport: { width: job.width, height: job.height || 800 },
      deviceScaleFactor: job.scale || 1,
    });
    const page = await context.newPage();
    page.on("pageerror", (error) => {
      console.error(`${job.name}: ${error.message}`);
      failed = true;
    });
    await page.goto(base + job.query);
    await settled(page, Boolean(job.terminal));
    const target = path.join(out, `${job.name}.png`);
    if (job.terminal) {
      const box = async () =>
        page.evaluate(({ cols, rows }) => {
          const probe = document.querySelector("#measure");
          const cellWidth = probe.getBoundingClientRect().width / probe.textContent.length;
          const rowHeight = parseFloat(getComputedStyle(probe).lineHeight);
          const host = document.querySelector("#content");
          const style = getComputedStyle(host);
          const pad = (side) => parseFloat(style[`padding${side}`]);
          const rect = host.getBoundingClientRect();
          return {
            x: rect.x,
            y: rect.y,
            width: Math.ceil(cols * cellWidth + pad("Left") + pad("Right")),
            height: Math.ceil(rows * rowHeight + pad("Top") + pad("Bottom")),
          };
        }, job.terminal);
      const first = await box();
      await page.setViewportSize({ width: first.width, height: Math.ceil(first.y + first.height) });
      await settled(page, true);
      // The strip may have wrapped differently at the new width.
      const second = await box();
      await page.setViewportSize({ width: second.width, height: Math.ceil(second.y + second.height) });
      await settled(page, true);
      await page.screenshot({ path: target, clip: await box() });
    } else {
      await page.locator(job.selector).screenshot({ path: target });
    }
    await context.close();
    console.log(`${job.name}.png`);
  }
  await browser.close();
  server.close();
  process.exit(failed ? 1 : 0);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});

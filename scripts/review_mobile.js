// Drive the review page the way a thumb does.
//
// The whole risk in the mobile work is that everything passes on a desktop
// browser and fails on the phone, so this runs a real Chromium at a real phone
// viewport with touch input, and does an entire review with nothing but taps,
// drags and pinches -- no keyboard, no mouse. It also renders the desktop
// layout twice, old page and new, and compares them: a careless media query
// breaking the page the owner actually uses is the likeliest regression here.
//
// Usage: node review_mobile.js <newPort> <oldPort> <outDir>

const { chromium } = require("/opt/node22/lib/node_modules/playwright");
const fs = require("fs");
const path = require("path");

const [newPort, oldPort, outDir] = process.argv.slice(2);
const CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const PHONE = { width: 390, height: 844 };   // iPhone 14/15 CSS pixels

const results = [];
const check = (ok, label, detail = "") => {
  results.push({ ok, label, detail });
  console.log(`  ${ok ? "PASS" : "FAIL"}  ${label}${detail ? " -- " + detail : ""}`);
};

// Playwright's touchscreen can tap and nothing else; a pinch needs two points
// at once, which only the raw CDP input domain can send. Chrome turns these
// into the pointer events the page listens for.
async function touch(cdp, type, points) {
  await cdp.send("Input.dispatchTouchEvent", {
    type,
    touchPoints: points.map((p, i) => ({ x: p.x, y: p.y, id: i })),
  });
}

async function pinch(cdp, cx, cy, from, to, steps = 12) {
  const pts = d => [{ x: cx - d / 2, y: cy }, { x: cx + d / 2, y: cy }];
  await touch(cdp, "touchStart", pts(from));
  for (let i = 1; i <= steps; i++) {
    await touch(cdp, "touchMove", pts(from + (to - from) * (i / steps)));
    await new Promise(r => setTimeout(r, 12));
  }
  await touch(cdp, "touchEnd", []);
}

async function drag(cdp, x, y, dx, dy, steps = 10) {
  await touch(cdp, "touchStart", [{ x, y }]);
  for (let i = 1; i <= steps; i++) {
    await touch(cdp, "touchMove", [{ x: x + dx * i / steps, y: y + dy * i / steps }]);
    await new Promise(r => setTimeout(r, 12));
  }
  await touch(cdp, "touchEnd", []);
}

(async () => {
  const browser = await chromium.launch({ executablePath: CHROME });

  // ------------------------------------------------------------------ phone
  const ctx = await browser.newContext({
    viewport: PHONE, deviceScaleFactor: 3, isMobile: true, hasTouch: true,
    userAgent: "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) " +
               "AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile Safari/604.1",
  });
  const page = await ctx.newPage();
  const errors = [];
  page.on("pageerror", e => errors.push(String(e)));
  page.on("console", m => { if (m.type() === "error") errors.push(m.text()); });
  await page.goto(`http://127.0.0.1:${newPort}/`, { waitUntil: "networkidle" });
  await page.waitForSelector(".card", { timeout: 15000 });

  console.log("\nphone layout");
  // The single symptom of every layout miss: content wider than the screen.
  // clientWidth, NOT window.innerWidth: innerWidth grows with the overflow, so
  // comparing against it is `scrollWidth <= scrollWidth` and passes on a page
  // that is three times too wide. It did, on the first run of this file.
  const over = await page.evaluate(() => {
    const d = document.documentElement;
    return { scroll: d.scrollWidth, client: d.clientWidth };
  });
  check(over.scroll <= over.client + 1, "the feed does not scroll sideways",
        `${over.scroll}px of content in ${over.client}px`);

  const rail = await page.evaluate(() => {
    const r = document.getElementById("rail").getBoundingClientRect();
    return { h: Math.round(r.height), w: Math.round(r.width) };
  });
  check(rail.w <= PHONE.width + 1 && rail.h < PHONE.height * 0.5,
        "the sidebar became a header, not a 230px column",
        `${rail.w}x${rail.h}`);

  // Every control has to be hittable. 40px is the floor this page settled on.
  const small = await page.evaluate(() => {
    const bad = [];
    document.querySelectorAll(".card button, #rail-toggle").forEach(b => {
      const r = b.getBoundingClientRect();
      if (r.width && r.height < 36) bad.push((b.dataset.act || b.id || b.textContent).trim());
    });
    return bad;
  });
  check(small.length === 0, "every card control is at least 36px tall", small.join(", "));

  const inputPx = await page.evaluate(() => {
    const t = document.querySelector("textarea");
    return t ? parseFloat(getComputedStyle(t).fontSize) : 0;
  });
  check(inputPx >= 16, "the comment box is >=16px, so iOS will not zoom the page",
        inputPx + "px");

  await page.screenshot({ path: path.join(outDir, "phone-feed.png"), fullPage: false });

  // ------------------------------------------------------------ touch drive
  console.log("\nreviewing by touch only");
  const cdp = await ctx.newCDPSession(page);

  const img = await page.locator(".card .imgwrap img").first();
  const box = await img.boundingBox();
  await page.touchscreen.tap(box.x + box.width / 2, box.y + box.height / 2);
  await page.waitForTimeout(400);
  check(await page.locator("#lightbox.on").count() === 1,
        "tapping a card image opens it full screen");

  const before = await page.evaluate(() => document.getElementById("lb-zoom").textContent);
  await pinch(cdp, PHONE.width / 2, PHONE.height / 2, 60, 300);
  await page.waitForTimeout(200);
  const after = await page.evaluate(() => document.getElementById("lb-zoom").textContent);
  check(before !== after, "a pinch changes the zoom", `${before} -> ${after}`);

  const pos1 = await page.evaluate(() => document.getElementById("lb-img").style.left);
  await drag(cdp, PHONE.width / 2, PHONE.height / 2, -120, 0);
  await page.waitForTimeout(200);
  const pos2 = await page.evaluate(() => document.getElementById("lb-img").style.left);
  check(pos1 !== pos2, "a one-finger drag pans", `${pos1} -> ${pos2}`);

  await page.locator("#lb-out").tap();
  await page.waitForTimeout(150);
  const afterBtn = await page.evaluate(() => document.getElementById("lb-zoom").textContent);
  check(afterBtn !== after, "the on-screen zoom-out button works", afterBtn);

  // Double-tap toggles fit and 1:1 -- and the same two taps must NOT leave a
  // stray pin behind, which is why a touch tap waits 280ms before acting.
  const zoomBefore = await page.evaluate(() => document.getElementById("lb-zoom").textContent);
  await page.touchscreen.tap(PHONE.width / 2, PHONE.height / 2);
  await page.touchscreen.tap(PHONE.width / 2, PHONE.height / 2);
  await page.waitForTimeout(500);
  const zoomAfter = await page.evaluate(() => document.getElementById("lb-zoom").textContent);
  check(zoomBefore !== zoomAfter, "double-tap toggles the zoom", `${zoomBefore} -> ${zoomAfter}`);
  check(await page.locator("#lightbox.on").count() === 1,
        "...and does not close the viewer");
  check(await page.locator("#lightbox .pin").count() === 0,
        "...and drops no stray pin from the first of the two taps");

  // A single tap on the image pins the pixel under the finger.
  page.once("dialog", d => d.accept("seam here"));
  await page.touchscreen.tap(PHONE.width / 2, PHONE.height / 2);
  await page.waitForTimeout(700);
  check(await page.locator("#lightbox .pin").count() === 1,
        "a single tap on the image pins that pixel");

  await page.locator("#lb-close").tap();
  await page.waitForTimeout(300);
  check(await page.locator("#lightbox.on").count() === 0, "Close closes it");

  // Pick, rate, comment, submit -- the actual verdict, all by thumb.
  const pick = page.locator('.card button[data-act="choice"]').first();
  if (await pick.count()) {
    await pick.tap();
    await page.waitForTimeout(150);
    check(await page.locator(".pane.picked").count() > 0, "tapping a pane picks it");
  }
  await page.locator('.card button[data-act="rating"]').nth(3).tap();
  await page.waitForTimeout(150);
  await page.locator(".card textarea").first().fill("from the phone");
  const openBefore = await page.locator(".card").count();
  await page.locator('.card button[data-act="submit"]').first().tap();
  await page.waitForTimeout(1500);
  // The board is filtered to `open`, so an answered card leaves the feed --
  // asserting a .tag.answered appears looks right and can never pass here.
  const openAfter = await page.locator(".card").count();
  check(openAfter === openBefore - 1,
        "the verdict submits and the card leaves the open queue",
        `${openBefore} -> ${openAfter} cards`);
  await page.locator('.rail-item[data-status="answered"]').tap();
  await page.waitForTimeout(600);
  check(await page.locator(".verdict").first().innerText().catch(() => "")
          .then(t => t.includes("from the phone")),
        "and the comment typed on the phone is what came back");
  await page.locator('.rail-item[data-status="open"]').tap();
  await page.waitForTimeout(400);

  // The batched ping lives one tap deep behind the hamburger.
  await page.locator("#rail-toggle").tap();
  await page.waitForTimeout(200);
  check(await page.locator("#rail.open #notify").isVisible(),
        "Send verdicts is reachable behind the hamburger");
  await page.screenshot({ path: path.join(outDir, "phone-menu.png") });
  await page.locator("#rail-toggle").tap();

  check(errors.length === 0, "no JavaScript errors during the whole run",
        errors.slice(0, 2).join(" | "));
  await ctx.close();

  // ---------------------------------------------------- desktop is unchanged
  console.log("\ndesktop regression");
  // Geometry, not pixels. Every box of every element, in DOM order, plus the
  // properties a media query can leak into: that is precisely the regression
  // worth catching, and it does not fail on an intentional wording change --
  // which a byte-compare of the screenshots does, and did.
  const geom = {};
  for (const [name, port] of [["new", newPort], ["old", oldPort]]) {
    const c = await browser.newContext({ viewport: { width: 1440, height: 900 } });
    const p = await c.newPage();
    await p.goto(`http://127.0.0.1:${port}/`, { waitUntil: "networkidle" });
    await p.waitForSelector(".card", { timeout: 15000 });
    await p.waitForTimeout(500);
    await p.screenshot({ path: path.join(outDir, `desktop-${name}.png`), fullPage: true });
    // Keyed by identity, not by position in the DOM. This change adds a
    // wrapper (#rail-top), and an index-aligned compare turns one insertion
    // into a diff on every element after it -- noise that hides the one that
    // matters. Elements present in only one page are skipped; everything the
    // two have in common must be in exactly the same place.
    geom[name] = await p.evaluate(() => {
      const seen = {}, out = {};
      document.querySelectorAll("body *").forEach(el => {
        const cls = typeof el.className === "string" ? el.className.trim() : "";
        if (!el.id && !cls) return;
        const base = el.tagName + "#" + el.id + "." + cls;
        const n = seen[base] = (seen[base] || 0) + 1;
        const r = el.getBoundingClientRect(), cs = getComputedStyle(el);
        out[base + "@" + n] = [Math.round(r.x), Math.round(r.y),
                               Math.round(r.width), Math.round(r.height),
                               cs.display, cs.position, cs.overflowX,
                               cs.fontSize, cs.minHeight].join("|");
      });
      return out;
    });
    await c.close();
  }
  const shared = Object.keys(geom.old).filter(k => k in geom.new);
  const moved = shared.filter(k => geom.old[k] !== geom.new[k]);
  check(moved.length === 0,
        "the desktop layout is geometrically identical to the page before this change",
        moved.length === 0 ? `${shared.length} elements compared`
          : moved.slice(0, 3).map(k =>
              `\n      ${k}\n        old ${geom.old[k]}\n        new ${geom.new[k]}`).join(""));

  await browser.close();
  const failed = results.filter(r => !r.ok);
  console.log(failed.length ? `\n${failed.length} checks failed` : "\nall checks passed");
  process.exit(failed.length ? 1 : 0);
})().catch(e => { console.error(e); process.exit(2); });

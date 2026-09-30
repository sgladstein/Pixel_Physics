---
paths:
  - "src/main.rs"
  - "src/app.rs"
  - "src/render.rs"
  - "src/hud.rs"
  - "src/bin/**"
  - "examples/filmstrip.rs"
---

# Running and screenshotting the real app

Moved out of `CLAUDE.md` on 2026-09-30; the full account is in
`Reports/claude-md-evidence-2026-09-30.md`, *Commands*.

On a headless Linux box:

```
apt-get install -y libxkbcommon-x11-0 mesa-vulkan-drivers   # once per container
xvfb-run -a -s "-screen 0 1280x800x24" \
  env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
      PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES=3 \
  ./target/release/pixel-physics                            # writes %TEMP%/pixel_physics_screenshot.png
```

- `lvp_icd.json` is lavapipe. Without it `Pixels::new` fails with "Unable to
  create a surface", which looks like a code bug and is a missing driver.
- It is seconds per frame: for **looking at one frame**, never for timing.
  Frame timings come from `ascii`; `PIXEL_PHYSICS_CAPTURE_SEQUENCE=<start>,
  <interval>,<count>` still gives a strip.
- **The three binaries write different files**: `./target/release/druid`
  writes `pixel_physics_druid_screenshot.png`.
- **None of them exits after the shutter.** Kill it yourself with
  `for p in $(pgrep -x druid); do kill $p; done` — `pkill -f target/release/druid`
  matches the wrapping shell's command line and kills your own script.
- In the app, `F7` selects the `flat` preset (dead-level rock, 200 rows of
  sky, the structural test bed).
- **The app locks its own exe** while running; `cargo test --lib` still works.

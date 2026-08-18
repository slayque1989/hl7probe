#!/usr/bin/env python3
"""Regenerate the images in docs/ from real hl7probe output.

    python3 docs/tools/render_media.py

Writes docs/report.svg, docs/tui.svg and docs/demo.gif. Everything shown is
captured by running the tool, so the images cannot drift from what it prints.

Needs a release build (made automatically), and Pillow for the GIF.
"""
import html
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DOCS = ROOT / "docs"
BINARY = ROOT / "target" / "release" / "hl7probe"

# One terminal palette shared by every renderer.
PALETTE = {
    31: "#e06c75", 32: "#98c379", 33: "#e5c07b", 34: "#61afef",
    35: "#c678dd", 36: "#56b6c2", 90: "#7f848e", 39: "#c8ccd4",
}
FOREGROUND = "#c8ccd4"
BACKGROUND = "#181b21"
TITLE_BAR = "#22262e"
SGR = re.compile(r"\x1b\[([0-9;]*)m")


def runs(line):
    """Split an ANSI line into (text, colour, bold, dim) runs, merging neighbours
    that share a style."""
    out, pos = [], 0
    colour, bold, dim = None, False, False
    for match in SGR.finditer(line):
        if match.start() > pos:
            out.append((line[pos:match.start()], colour, bold, dim))
        pos = match.end()
        codes = [int(c) for c in match.group(1).split(";") if c] or [0]
        index = 0
        while index < len(codes):
            code = codes[index]
            if code == 0:
                colour, bold, dim = None, False, False
            elif code == 1:
                bold = True
            elif code == 2:
                dim = True
            elif code == 22:
                bold = dim = False
            elif code == 39:
                colour = None
            elif code in PALETTE:
                colour = PALETTE[code]
            elif code == 38 and codes[index + 1:index + 2] == [2]:
                colour = "#%02x%02x%02x" % tuple(codes[index + 2:index + 5])
                index += 4
            index += 1
    if pos < len(line):
        out.append((line[pos:], colour, bold, dim))

    merged = []
    for run in out:
        if merged and merged[-1][1:] == run[1:]:
            merged[-1] = (merged[-1][0] + run[0],) + run[1:]
        else:
            merged.append(run)
    return merged


def plain(line):
    return SGR.sub("", line)


# --------------------------------------------------------------------- SVG

CHAR_W, LINE_H, PAD_X, PAD_Y = 8.4, 19.0, 22, 34


def render_svg(text, title):
    lines = text.rstrip("\n").split("\n")
    cols = max((len(plain(line)) for line in lines), default=80)
    width = PAD_X * 2 + cols * CHAR_W
    height = PAD_Y + len(lines) * LINE_H + 18
    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width:.0f}" height="{height:.0f}" '
        f'viewBox="0 0 {width:.0f} {height:.0f}" '
        f'font-family="SFMono-Regular,Menlo,Consolas,monospace" font-size="13">',
        f'<rect width="100%" height="100%" rx="10" fill="{BACKGROUND}"/>',
        f'<rect width="100%" height="28" rx="10" fill="{TITLE_BAR}"/>',
        '<circle cx="18" cy="14" r="5.5" fill="#e06c75"/>'
        '<circle cx="36" cy="14" r="5.5" fill="#e5c07b"/>'
        '<circle cx="54" cy="14" r="5.5" fill="#98c379"/>',
        f'<text x="74" y="18" fill="#7f848e" font-size="11">{html.escape(title)}</text>',
    ]
    for row, line in enumerate(lines):
        column, chunks = 0, []
        for text_run, colour, bold, dim in runs(line):
            if not text_run:
                continue
            fill = colour or FOREGROUND
            opacity = ' opacity="0.55"' if dim and not colour else ""
            weight = ' font-weight="600"' if bold else ""
            x = PAD_X + column * CHAR_W
            chunks.append(
                f'<tspan x="{x:.1f}" textLength="{len(text_run) * CHAR_W:.1f}" '
                f'lengthAdjust="spacingAndGlyphs" fill="{fill}"{weight}{opacity}>'
                f"{html.escape(text_run)}</tspan>"
            )
            column += len(text_run)
        if chunks:
            y = PAD_Y + row * LINE_H
            parts.append(f'<text y="{y:.1f}" xml:space="preserve">' + "".join(chunks) + "</text>")
    parts.append("</svg>")
    return "\n".join(parts)


# --------------------------------------------------------------------- GIF

FONT_PATH = "/System/Library/Fonts/Menlo.ttc"
FONT_SIZE = 15
GIF_COLS, GIF_ROWS = 88, 37


def render_gif(scenes, path):
    """Animates a terminal session. `scenes` is a list of (command, output)."""
    from PIL import Image, ImageDraw, ImageFont

    regular = ImageFont.truetype(FONT_PATH, FONT_SIZE, index=0)
    bold_font = ImageFont.truetype(FONT_PATH, FONT_SIZE, index=1)
    char_w = regular.getlength("M")
    line_h = FONT_SIZE * 1.45
    width = int(PAD_X * 2 + GIF_COLS * char_w)
    height = int(40 + GIF_ROWS * line_h + 12)

    def rgb(colour):
        colour = colour.lstrip("#")
        return tuple(int(colour[i:i + 2], 16) for i in (0, 2, 4))

    def draw_screen(lines, cursor):
        image = Image.new("RGB", (width, height), rgb(BACKGROUND))
        draw = ImageDraw.Draw(image)
        draw.rectangle([0, 0, width, 26], fill=rgb(TITLE_BAR))
        for i, colour in enumerate(("#e06c75", "#e5c07b", "#98c379")):
            draw.ellipse([16 + i * 18, 8, 26 + i * 18, 18], fill=rgb(colour))
        draw.text((78, 6), "hl7probe", font=regular, fill=rgb("#7f848e"))
        visible = lines[-GIF_ROWS:]
        for row, line in enumerate(visible):
            y = 40 + row * line_h
            x = PAD_X
            for text_run, colour, bold, dim in runs(line):
                fill = rgb(colour or FOREGROUND)
                if dim:
                    fill = tuple(int(c * 0.55) for c in fill)
                draw.text((x, y), text_run, font=bold_font if bold else regular, fill=fill)
                x += char_w * len(text_run)
            if cursor and row == len(visible) - 1:
                draw.rectangle([x, y + 2, x + char_w - 1, y + FONT_SIZE + 3], fill=rgb("#56b6c2"))
        return image

    frames, delays, screen = [], [], []

    def add(lines, milliseconds, cursor=False):
        frames.append(draw_screen(lines, cursor))
        delays.append(milliseconds)

    for command, output, hold, chunk in scenes:
        screen.clear()
        for i in range(0, len(command) + 1, 2):
            add(screen + [f"\x1b[32m$\x1b[0m {command[:i]}"], 45, cursor=True)
        screen.append(f"\x1b[32m$\x1b[0m {command}")
        add(screen, 260)
        lines = output.rstrip("\n").split("\n")
        for i in range(chunk, len(lines) + chunk, chunk):
            screen.extend(lines[i - chunk:i])
            add(screen, 55)
        screen.append("")
        add(screen, hold)

    base = frames[0].quantize(colors=96, method=Image.MEDIANCUT)
    converted = [frame.quantize(palette=base, dither=Image.NONE) for frame in frames]
    converted[0].save(
        path,
        save_all=True,
        append_images=converted[1:],
        duration=delays,
        loop=0,
        optimize=True,
        disposal=1,
    )
    return len(frames), sum(delays) / 1000


# -------------------------------------------------------------------- capture


def run_tool(*args):
    result = subprocess.run(
        [str(BINARY), *args], capture_output=True, text=True, cwd=ROOT, check=False
    )
    if result.returncode > 1:
        sys.exit(f"hl7probe {' '.join(args)} failed: {result.stderr}")
    return result.stdout


def capture_tui(example, step, rows):
    """Draws one interactive frame through the viewer's own dump test."""
    result = subprocess.run(
        ["cargo", "test", "--quiet", "dump_screen", "--", "--ignored", "--nocapture"],
        capture_output=True,
        text=True,
        cwd=ROOT,
        env={
            **__import__("os").environ,
            "HL7TEST_DUMP": example,
            "HL7TEST_DUMP_STEP": str(step),
            "HL7TEST_DUMP_ROWS": str(rows),
        },
        check=True,
    )
    skip = ("running", "test result", ".", "")
    return "\n".join(
        line for line in result.stdout.split("\n")
        if not any(plain(line).strip().startswith(s) and s for s in skip)
        and plain(line).strip() != ""
    )


def main():
    subprocess.run(["cargo", "build", "--release", "--quiet"], cwd=ROOT, check=True)

    report = run_tool("examples/invalid.hl7", "--color", "always", "--width", "88", "-s", "PID,PV1")
    (DOCS / "report.svg").write_text(render_svg(report, "hl7probe admit.hl7"))

    viewer = capture_tui("examples/oru_r01.hl7", step=4, rows=24)
    (DOCS / "tui.svg").write_text(render_svg(viewer, "hl7probe --tui examples/oru_r01.hl7"))

    scenes = [
        (
            "hl7probe examples/adt_a01.hl7 -s PID",
            run_tool("examples/adt_a01.hl7", "--color", "always", "--width", "84", "-s", "PID"),
            3200,
            4,
        ),
        (
            "hl7probe -f PID-5.1 examples/adt_a01.hl7",
            run_tool("-f", "PID-5.1", "examples/adt_a01.hl7", "--color", "always"),
            1800,
            1,
        ),
        (
            "hl7probe examples/invalid.hl7 --summary",
            run_tool("examples/invalid.hl7", "--color", "always", "--width", "84", "--summary"),
            4000,
            4,
        ),
    ]
    count, seconds = render_gif(scenes, DOCS / "demo.gif")
    print(f"docs/report.svg, docs/tui.svg, docs/demo.gif ({count} frames, {seconds:.1f}s)")


if __name__ == "__main__":
    main()

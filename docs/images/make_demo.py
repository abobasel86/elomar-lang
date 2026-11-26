"""An animated terminal demo (GIF) of real Elomar-Lang output.

    python -I mkdemo.py <elomar.exe> <work dir> <out.gif>

Runs the commands for real in <work dir> and draws what they print."""
import os
import pathlib
import re
import shutil
import subprocess
import sys

import arabic_reshaper
from bidi.algorithm import get_display
from PIL import Image, ImageDraw, ImageFont

ELOMAR, WORK, OUT = sys.argv[1], pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3])
F = "C:/Windows/Fonts/"
W, H = 1040, 580
SIZE, LINE = 19, 27
MONO = ImageFont.truetype(F + "consola.ttf", SIZE)
MONO_B = ImageFont.truetype(F + "consolab.ttf", SIZE)
ARAB = ImageFont.truetype(F + "tahoma.ttf", SIZE)
CAPTION = ImageFont.truetype(F + "segoeuib.ttf", 22)
CELL = MONO.getlength("M")
BG, BAR, FG = (22, 27, 31), (36, 42, 48), (222, 228, 226)
GREEN, GOLD, RED, MUTED, TEAL = (120, 214, 160), (240, 196, 98), (255, 128, 118), (140, 150, 150), (92, 195, 179)
TOP = 52
ARABIC = re.compile(r"[\u0600-\u06FF\uFB50-\uFDFF\uFE70-\uFEFF]")


def run(*args, cwd):
    env = dict(os.environ, ELOMAR_LANG="en")
    r = subprocess.run([ELOMAR, *args], cwd=cwd, capture_output=True, text=True,
                       encoding="utf-8", env=env)
    return (r.stdout + r.stderr).rstrip("\n").split("\n")


def draw_line(d, x, y, text, color):
    """Draws one terminal line; Arabic runs in Tahoma, in visual order,
    each Arabic run kept as wide as its characters would be in the grid."""
    if ARABIC.search(text):
        rtl = bool(ARABIC.match(text.lstrip()))
        text = get_display(arabic_reshaper.reshape(text), base_dir="R" if rtl else "L")
        if rtl:  # a right-to-left line starts at the right edge
            x = W - 24 - run_width(text.strip())
            text = text.strip()
    for run_text, is_ar in split_runs(text):
        if is_ar:
            w = ARAB.getlength(run_text)
            cells = len(run_text) * CELL
            d.text((x + max(0, cells - w) / 2, y + SIZE), run_text, font=ARAB, fill=color, anchor="ls")
            x += max(w, cells)
        else:
            d.text((x, y + SIZE), run_text, font=MONO, fill=color, anchor="ls")
            x += MONO.getlength(run_text)


def split_runs(text):
    runs = []
    for ch in text:
        is_ar = bool(ARABIC.match(ch))
        if runs and runs[-1][1] == is_ar:
            runs[-1][0] += ch
        else:
            runs.append([ch, is_ar])
    return runs


def run_width(text):
    return sum(max(ARAB.getlength(t), len(t) * CELL) if a else MONO.getlength(t)
               for t, a in split_runs(text))


def color_of(line):
    if line.startswith("$ "):
        return FG
    if re.match(r"^(error|خطأ)", line.strip()) or line.lstrip().startswith(("^", "|")) and "^" in line:
        return RED
    if line.lstrip().startswith("//"):
        return MUTED
    if line.startswith("pub ") or line.startswith("    return await"):
        return GOLD
    if line.startswith("[URGENT]") or line.startswith("closest note"):
        return GREEN
    return MUTED if line.startswith(("  helpdesk", "  elomar", " -->")) else FG


def frame(lines, caption, cursor=False):
    img = Image.new("RGB", (W, H), BG)
    d = ImageDraw.Draw(img)
    d.rectangle([0, 0, W, 38], fill=BAR)
    for i, c in enumerate([(255, 95, 86), (255, 189, 46), (39, 201, 63)]):
        d.ellipse([16 + i * 22, 13, 28 + i * 22, 25], fill=c)
    d.text((W / 2, 19), "Elomar-Lang", font=MONO_B, fill=MUTED, anchor="mm")
    y = TOP
    for ln in lines:
        if ln.startswith("$ "):
            d.text((24, y + SIZE), "$", font=MONO_B, fill=TEAL, anchor="ls")
            draw_line(d, 24 + 2 * CELL, y, ln[2:], FG)
        else:
            draw_line(d, 24, y, ln, color_of(ln))
        y += LINE
    if cursor:
        last = lines[-1] if lines else ""
        cx = 24 + len(last) * CELL
        d.rectangle([cx, y - LINE + 3, cx + CELL - 2, y - 4], fill=FG)
    # The caption bar at the bottom says what the scene shows.
    d.rectangle([0, H - 58, W, H], fill=(15, 110, 99))
    d.text((W / 2, H - 29), caption, font=CAPTION, fill=(255, 255, 255), anchor="mm")
    return img


frames, durations = [], []


def add(img, ms):
    frames.append(img)
    durations.append(ms)


def scene(cmd, output, caption, before=()):
    lines = list(before)
    for i in range(0, len(cmd) + 1, 2):
        add(frame(lines + ["$ " + cmd[:i]], caption, cursor=True), 45)
    add(frame(lines + ["$ " + cmd], caption, cursor=True), 450)
    shown = lines + ["$ " + cmd]
    for ln in output:
        shown.append(ln)
        add(frame(shown, caption), 70)
    add(frame(shown, caption), 2600)
    return shown


shutil.rmtree(WORK, ignore_errors=True)
WORK.mkdir(parents=True)

out1 = run("new", "helpdesk", "--ai", "mock", cwd=WORK)
scene("elomar new helpdesk --ai mock", out1,
      "A new project with AI: classify tickets into your own types")

proj = WORK / "helpdesk"
code = (proj / "src/tickets.elo").read_text(encoding="utf-8").splitlines()
start = next(i for i, l in enumerate(code) if l.startswith("pub type Ticket"))
end = next(i for i, l in enumerate(code) if l.strip() == ")") + 1
shown = scene("cat src/tickets.elo", code[start:end], "The model must answer with your own type",
              before=["$ cd helpdesk"])
out2 = run("run", "src/main.elo", cwd=proj)
scene("elomar run src/main.elo", out2,
      "Typed answers from the model, and search by meaning", before=shown)

src = 'fn main() {\n    let الدرجة: Int = "95"\n    print(الدرجة)\n}\n'
(WORK / "grade.elo").write_text(src, encoding="utf-8")
shown = scene("cat grade.elo", src.rstrip("\n").split("\n"), "Names and error messages in Arabic")
out3 = run("run", "--lang", "ar", "grade.elo", cwd=WORK)
scene("elomar run --lang ar grade.elo", out3, "Names and error messages in Arabic",
      before=shown)

# The last card: where to try it.
end = Image.new("RGB", (W, H))
d = ImageDraw.Draw(end)
for x in range(W):
    t = x / (W - 1)
    d.line([(x, 0), (x, H)], fill=tuple(int(a + (b - a) * t) for a, b in zip((8, 66, 62), (16, 112, 100))))
big = ImageFont.truetype(F + "consolab.ttf", 64)
mid = ImageFont.truetype(F + "segoeui.ttf", 30)
d.text((W / 2, 200), "Elomar-Lang", font=big, fill=(240, 248, 246), anchor="mm")
d.text((W / 2, 270), "A typed language with AI built in, in English and Arabic", font=mid, fill=(150, 226, 210), anchor="mm")
d.text((W / 2, 360), "Try it in your browser:", font=mid, fill=(240, 248, 246), anchor="mm")
d.text((W / 2, 405), "abobasel86.github.io/elomar-lang", font=ImageFont.truetype(F + "consolab.ttf", 32), fill=GOLD, anchor="mm")
add(end, 3500)

pal = [f.convert("P", palette=Image.ADAPTIVE, colors=64, dither=Image.Dither.NONE) for f in frames]
pal[0].save(OUT, save_all=True, append_images=pal[1:], duration=durations, loop=0, optimize=True, disposal=1)
print(len(frames), "frames,", round(OUT.stat().st_size / 1024), "KB")

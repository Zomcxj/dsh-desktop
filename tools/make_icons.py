import io
from PIL import Image

SRC = "assets/icon.svg"
PNG_OUT = "assets/icon.png"
ICO_OUT = "assets/icon.ico"

# cairosvg 若不可用则报错（PIL 不支持 SVG）
try:
    import cairosvg
except ImportError:
    raise SystemExit("需要 cairosvg: pip install cairosvg")

sizes = [(256, 256), (128, 128), (64, 64), (48, 48), (32, 32), (16, 16)]
png_256 = cairosvg.svg2png(url=SRC, output_width=256, output_height=256)
with open(PNG_OUT, "wb") as f:
    f.write(png_256)

imgs = []
for w, h in sizes:
    data = cairosvg.svg2png(url=SRC, output_width=w, output_height=h)
    imgs.append(Image.open(io.BytesIO(data)))
imgs[0].save(ICO_OUT, format="ICO", sizes=[(w, h) for w, h in sizes], append_images=imgs[1:])
print("OK: assets/icon.png, assets/icon.ico")
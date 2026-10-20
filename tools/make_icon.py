"""Gera assets/icon.ico (16–256 px) a partir de assets/icon.png. Requer Pillow."""
from pathlib import Path

from PIL import Image

root = Path(__file__).resolve().parent.parent
source = Image.open(root / "assets" / "icon.png").convert("RGBA")
sizes = [(16, 16), (20, 20), (24, 24), (32, 32), (40, 40), (48, 48), (64, 64), (128, 128), (256, 256)]
source.save(root / "assets" / "icon.ico", sizes=sizes)
print("assets/icon.ico:", (root / "assets" / "icon.ico").stat().st_size, "bytes")

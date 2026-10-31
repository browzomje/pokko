"""Spanish translations for Pokko."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
LOCALES_DIR = ROOT / "src" / "lib" / "i18n" / "locales"

with open(LOCALES_DIR / "es.json", "r", encoding="utf-8") as f:
    ES = json.load(f)

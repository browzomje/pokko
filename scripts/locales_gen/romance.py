"""Romance languages translations for Pokko: es, fr, pt."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
LOCALES_DIR = ROOT / "src" / "lib" / "i18n" / "locales"

from .romance_es import ES
from .romance_fr import FR
from .romance_pt import PT

def export():
    for code, data in [("es", ES), ("fr", FR), ("pt", PT)]:
        with open(LOCALES_DIR / f"{code}.json", "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
        print(f"Exported {code}.json ({len(data)} keys).")

if __name__ == "__main__":
    export()

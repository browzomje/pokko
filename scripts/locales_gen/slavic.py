"""Export slavic languages: ru, pl."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
LOCALES_DIR = ROOT / "src" / "lib" / "i18n" / "locales"

from .slavic_ru import RU
from .slavic_pl import PL

def export():
    for code, data in [("ru", RU), ("pl", PL)]:
        with open(LOCALES_DIR / f"{code}.json", "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
        print(f"Exported {code}.json ({len(data)} keys).")

if __name__ == "__main__":
    export()

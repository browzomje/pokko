"""Export other languages: tr, ar, hi."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
LOCALES_DIR = ROOT / "src" / "lib" / "i18n" / "locales"

from .other_tr import TR
from .other_ar import AR
from .other_hi import HI

def export():
    for code, data in [("tr", TR), ("ar", AR), ("hi", HI)]:
        with open(LOCALES_DIR / f"{code}.json", "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
        print(f"Exported {code}.json ({len(data)} keys).")

if __name__ == "__main__":
    export()

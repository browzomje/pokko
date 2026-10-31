"""Export germanic languages: de, nl."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
LOCALES_DIR = ROOT / "src" / "lib" / "i18n" / "locales"

from .germanic_de import DE
from .germanic_nl import NL

def export():
    for code, data in [("de", DE), ("nl", NL)]:
        with open(LOCALES_DIR / f"{code}.json", "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
        print(f"Exported {code}.json ({len(data)} keys).")

if __name__ == "__main__":
    export()

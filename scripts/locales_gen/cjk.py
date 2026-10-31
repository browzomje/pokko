"""Export CJK languages: zh, ja, ko."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
LOCALES_DIR = ROOT / "src" / "lib" / "i18n" / "locales"

from .cjk_zh import ZH
from .cjk_ja import JA
from .cjk_ko import KO

def export():
    for code, data in [("zh", ZH), ("ja", JA), ("ko", KO)]:
        with open(LOCALES_DIR / f"{code}.json", "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
        print(f"Exported {code}.json ({len(data)} keys).")

if __name__ == "__main__":
    export()

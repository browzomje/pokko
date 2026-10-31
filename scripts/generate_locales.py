#!/usr/bin/env python3
"""Pokko 15-Language Locale Manager and Generator.

Master reference: en.json & it.json
15 Languages (matching Vesta UI specifications):
- ar: Arabic (العربية) 🇸🇦
- zh: Chinese (中文) 🇨🇳
- nl: Dutch (Nederlands) 🇳🇱
- en: English (English) 🇬🇧
- fr: French (Français) 🇫🇷
- de: German (Deutsch) 🇩🇪
- hi: Hindi (हिन्दी) 🇮🇳
- it: Italian (Italiano) 🇮🇹
- ja: Japanese (日本語) 🇯🇵
- ko: Korean (한국어) 🇰🇷
- pl: Polish (Polski) 🇵🇱
- pt: Portuguese (Português) 🇵🇹
- ru: Russian (Русский) 🇷🇺
- es: Spanish (Español) 🇪🇸
- tr: Turkish (Türkçe) 🇹🇷
"""

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LOCALES_DIR = ROOT / "src" / "lib" / "i18n" / "locales"

from locales_gen.romance import export as export_romance
from locales_gen.germanic import export as export_germanic
from locales_gen.slavic import export as export_slavic
from locales_gen.cjk import export as export_cjk
from locales_gen.other import export as export_other

ALL_CODES = ["ar", "zh", "nl", "en", "fr", "de", "hi", "it", "ja", "ko", "pl", "pt", "ru", "es", "tr"]

def export_all():
    print("Generating/Updating all 13 non-master locales (preserving en.json & it.json)...")
    export_romance()
    export_germanic()
    export_slavic()
    export_cjk()
    export_other()
    print("All 15 locales successfully generated!")

def check_all() -> int:
    en_path = LOCALES_DIR / "en.json"
    with open(en_path, "r", encoding="utf-8") as f:
        en = json.load(f)
    ref_keys = set(en.keys())
    print(f"Reference en.json has {len(ref_keys)} keys.\n")

    errors = 0
    for code in sorted(ALL_CODES):
        fpath = LOCALES_DIR / f"{code}.json"
        if not fpath.exists():
            print(f"  [✗] Missing locale file: {code}.json", file=sys.stderr)
            errors += 1
            continue
        with open(fpath, "r", encoding="utf-8") as f:
            data = json.load(f)
        keys = set(data.keys())
        missing = ref_keys - keys
        extra = keys - ref_keys
        if missing:
            print(f"  [✗] {code}.json missing {len(missing)} keys: {list(missing)[:5]}", file=sys.stderr)
            errors += len(missing)
        if extra:
            print(f"  [✗] {code}.json has {len(extra)} extra keys: {list(extra)[:5]}", file=sys.stderr)
            errors += len(extra)
        if not missing and not extra:
            print(f"  [✓] {code}.json: 100% key parity ({len(keys)} keys)")

    return 1 if errors > 0 else 0

def main():
    parser = argparse.ArgumentParser(description="Pokko i18n manager")
    parser.add_argument("--generate", action="store_true", help="Generate all 13 locales")
    parser.add_argument("--check", action="store_true", help="Check all 15 locales for parity")
    args = parser.parse_args()

    if args.generate:
        export_all()
    if args.check or not args.generate:
        return check_all()
    return 0

if __name__ == "__main__":
    sys.exit(main())

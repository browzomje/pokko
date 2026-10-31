"""Placeholder validation helper."""
import json
import re

PLACEHOLDER_RE = re.compile(r"\{\{?(\w+)\}?\}")

def get_placeholders(s: str) -> list[str]:
    return sorted(PLACEHOLDER_RE.findall(str(s)))

def check_parity(ref: dict[str, str], target: dict[str, str], lang: str) -> list[str]:
    errs = []
    for k in ref:
        if k not in target:
            errs.append(f"Missing key in {lang}: {k}")
        else:
            p_ref = get_placeholders(ref[k])
            p_tgt = get_placeholders(target[k])
            if p_ref != p_tgt:
                errs.append(f"Placeholder mismatch in {lang} for {k}: expected {p_ref}, got {p_tgt}")
    for k in target:
        if k not in ref:
            errs.append(f"Extra key in {lang}: {k}")
    return errs

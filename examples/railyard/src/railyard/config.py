"""Reads config/railyard.toml."""

import tomllib


def load(path):
    with open(path, "rb") as handle:
        raw = tomllib.load(handle)
    return {
        "queue": raw["queue"]["path"],
        "max_attempts": int(raw.get("retry", {}).get("max_attempts", 3)),
    }


def queue_path():
    """RAILYARD_QUEUE wins over the file, so staging can point elsewhere."""
    import os
    return os.environ.get("RAILYARD_QUEUE", "")

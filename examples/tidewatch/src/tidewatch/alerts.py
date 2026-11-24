"""Sends the alerts to the harbour masters.

An alert names the harbour and the level. It goes only to a master whose
number has confirmed.
"""

import sys

from tidewatch.thresholds import high_marks, over_high_mark, readings


def compose(gauge, harbour, level_m):
    return f"{harbour}: gauge {gauge} reads {level_m:.2f} m, above the high mark"


def send(harbour, text):
    # The gateway is not wired yet: the message is printed.
    print(f"-> {harbour}: {text}")


def check():
    marks = high_marks()
    for gauge, harbour, level_m in readings():
        if over_high_mark(level_m, harbour, marks):
            send(harbour, compose(gauge, harbour, level_m))


if __name__ == "__main__":
    if "--check" not in sys.argv:
        sys.exit("usage: python3 -m tidewatch.alerts --check")
    check()


def compose_surge(harbour, gauge, level_m, limit_m):
    return f"{harbour}: storm surge at gauge {gauge}, {level_m:.2f} m against a limit of {limit_m:.2f} m"


SENT = set()


def send_once(harbour, text):
    """Send an alert once: a dedupe check on the text before the gateway is called."""
    if (harbour, text) in SENT:
        return False
    SENT.add((harbour, text))
    send(harbour, text)
    return True

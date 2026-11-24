"""The backoff between attempts: doubled each time, capped at five minutes."""

CAP_SECONDS = 300


def backoff(attempt):
    return min(CAP_SECONDS, 2 ** attempt)

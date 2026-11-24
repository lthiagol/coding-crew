# Railyard

Railyard is a small CI runner. It takes a job from a queue, runs its steps in
a clean checkout, and reports the result. A failed job is retried with a
backoff; a job that keeps failing is parked for a person to look at.

## What it is made of

- `src/railyard/runner.py` — takes a job, runs its steps, reports.
- `src/railyard/retry.py` — the backoff between attempts.
- `src/railyard/config.py` — reads `config/railyard.toml`.
- `config/railyard.toml` — the queue location and the attempt limit.

## Running it

    python3 -m railyard.runner config/railyard.toml

## How the team works

Open work is listed in `TODO.md`. A job that touches the queue format is
reviewed by the person who owns the queue.

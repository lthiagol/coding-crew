"""Takes a job from the queue, runs its steps, reports the result."""

import subprocess
import sys
import time

from railyard.config import load
from railyard.retry import backoff


def run_steps(steps, cwd):
    for step in steps:
        completed = subprocess.run(step, shell=True, cwd=cwd)
        if completed.returncode != 0:
            return completed.returncode
    return 0


def run_job(job, config):
    attempts = config["max_attempts"]
    for attempt in range(1, attempts + 1):
        code = run_steps(job["steps"], job["checkout"])
        if code == 0:
            return "passed"
        if attempt < attempts:
            time.sleep(backoff(attempt))
    return "parked"


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit("usage: python3 -m railyard.runner <config.toml>")
    config = load(sys.argv[1])
    print(f"queue at {config['queue']}, up to {config['max_attempts']} attempts")

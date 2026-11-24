# TODO

- [ ] Retry a failed job with the backoff
- [~] Cap the backoff at five minutes (retry.py has the cap; no test yet)
- [ ] Park a job after the last attempt and say why
- [ ] Log each attempt's exit code
- [~] Read the queue path from the environment too
- [ ] Show the queue depth in `railyard status`
- [ ] Timeout a step after 30 minutes
- [ ] Clean the checkout before each attempt
- [ ] Fix the runner crashing on a missing config file
- [~] Write the config reader's tests
- [ ] Allow a job to name the branch it builds
- [ ] Notify the owner when a job is parked

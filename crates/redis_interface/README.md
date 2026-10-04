# Redis Interface

A user-friendly interface to Redis.

## Fred replay

With `fred,deja` enabled, substitutable stream, list, hash, consumer-group,
and script calls capture typed replies and Redis errors for replay. Recordings
made before these codecs were added do not contain reconstructable values for
those calls; re-record them rather than falling through to live Redis.

To check a real substitution, run `bash crates/redis_interface/fred-replay-smoke.sh`
from the workspace root. It starts a disposable local Redis, records an XDEL,
stops that Redis, and asserts the same call returns the recorded count on replay.
Set `CARGO_TARGET_DIR` to an isolated build directory when other worktrees
are building concurrently.

# Redis Interface

A user-friendly interface to Redis.

## Fred replay

With `fred,deja` enabled, substitutable stream, list, hash, consumer-group,
and script calls capture typed replies and Redis errors for replay. Recordings
made before these codecs were added do not contain reconstructable values for
those calls; re-record them rather than falling through to live Redis.

# master

* Add new type `Before` for reading event streams backward

# [[0.2.1] 2019-04-29](https://github.com/cq-rs/cqrs/releases/tag/cqrs-core-0.2.1)

* Breaking change to `SnapshotSink` and `SnapshotStrategy` to allow
  differentiating between entities with an initial snapshot and those that
  were not in the snapshot store.

# [[0.1.1] 2019-03-08](https://github.com/cq-rs/cqrs/releases/tag/cqrs-core-0.1.1)

* Add `VersionedEventWithMetadata`.
* Breaking change to `EventSource::Events` so that the iterated items are no
  longer results.

# [[0.1.0] 2019-02-01](https://github.com/cq-rs/cqrs/releases/tag/cqrs-core-0.1.0)

* Initial release

use serde_json;
pub use super::*;

#[test]
fn deserialize_sample_text() {
    const DATA: &'static str = r#"
    [
      {
        "uri": "http://127.0.0.1:2113/streams/%24stats-127.0.0.1%3A2113",
        "relation": "self"
      },
      {
        "uri": "http://127.0.0.1:2113/streams/%24stats-127.0.0.1%3A2113/head/backward/20",
        "relation": "first"
      },
      {
        "uri": "http://127.0.0.1:2113/streams/%24stats-127.0.0.1%3A2113/19/forward/20",
        "relation": "previous"
      },
      {
        "uri": "http://127.0.0.1:2113/streams/%24stats-127.0.0.1%3A2113/metadata",
        "relation": "metadata"
      }
    ]
    "#;

    let result: Vec<LinkRelation> = serde_json::from_str(DATA).unwrap();
    assert_eq!(result[0].relation, Relation::SelfRel);
}

#[test]
fn deserialize_example_stream() {
    const DATA: &'static str = include_str!("samples/stream_page.json");
    let _result: StreamPage = serde_json::from_str(DATA).unwrap();
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all="camelCase")]
struct Statistics {
    #[serde(rename="es-queue-Index Committer-lastProcessedMessage")]
    es_queue_index_committer_last_processed_message: String,
    #[serde(rename="proc-diskIo-readOps")]
    proc_disk_io_read_ops: usize,
}

#[test]
fn deserialize_example_event() {
    const DATA: &'static str = include_str!("samples/event_sample.json");
    let result: EventEnvelope<Statistics, String> = serde_json::from_str(DATA).unwrap();
    assert_eq!(result.data.es_queue_index_committer_last_processed_message, "CommitAck");
}

#[test]
fn deserialize_owned_example_event() {
    const DATA: &'static str = include_str!("samples/event_sample.json");
    let cursor = ::std::io::Cursor::new(DATA);
    let result: EventEnvelope<Statistics, String> = serde_json::from_reader(cursor).unwrap();
    assert_eq!(result.data.es_queue_index_committer_last_processed_message, "CommitAck");
}


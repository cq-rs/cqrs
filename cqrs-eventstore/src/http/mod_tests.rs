pub use super::*;
use hyper::client::IntoUrl;

#[test]
fn stream_page_url_builds_as_expected() {
    assert_eq!(
        build_stream_page_url(
            &"http://example.com:143/".into_url().unwrap(),
            "test-stream",
            EventNumber::new(0),
            20,
        ),
        "http://example.com:143/streams/test-stream/0/forward/20".into_url().unwrap(),
    );
}

#[test]
#[cfg(with_local_es)]
fn for_fun() {
    let es_conn = EventStoreConnection {
        client: hyper::Client::new(),
        credentials: hyper::header::Authorization(hyper::header::Basic {
            username: "admin".to_string(),
            password: Some("changeit".to_string()),
        }),
        base_url: "http://127.0.0.1:2113/".into_url().unwrap(),
    };

    let result = es_conn.get_stream_page("$stats-127.0.0.1:2113", 0, 20);

    println!("{:#?}", result);

    assert!(result.is_ok());
}

assert_impl!(conn; EventStoreConnection, Send, Sync);
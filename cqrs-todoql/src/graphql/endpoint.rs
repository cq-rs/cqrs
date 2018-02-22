use std::sync::Arc;
use mount::Mount;
use juniper::http::graphiql::graphiql_source;
use juniper_iron::GraphQLHandler;
use iron;
use iron::headers::ContentType;
use iron::mime::{Mime, TopLevel, SubLevel};

use super::{InnerContext, Context};
use super::schema::{Query, Mutations};

pub fn create_chain(context: InnerContext) -> iron::Chain {
    let context_arc = Arc::new(context);

    let context_factory = move |_: &mut iron::Request| {
        Context {
            inner: Arc::clone(&context_arc),
        }
    };

    let mut mount = Mount::new();

    let graphql_endpoint = GraphQLHandler::new(
        context_factory,
        Query,
        Mutations,
    );

    mount.mount("/graphql", graphql_endpoint);
    mount.mount("/graphiql", |req: &mut iron::Request| {
        let url = req.url.as_ref().join("/graphql").unwrap();
        let graphiql_text = graphiql_source(url.as_ref());

        let json_header = ContentType(Mime(TopLevel::Text, SubLevel::Html, Vec::new()));

        let mut res = iron::Response::new();
        res.body = Some(Box::new(graphiql_text));
        res.headers.set(json_header);
        Ok(res)
    });

    iron::Chain::new(mount)
}
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all="camelCase")]
pub enum Relation {
    #[serde(rename="self")]
    SelfRel,
    First,
    Last,
    Previous,
    Next,
    Metadata,
    Edit,
    Alternate,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct LinkRelation {
    pub uri: String,
    pub relation: Relation,
}


#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(untagged)]
// Ordering is important since this is untagged.
pub enum EventEntry {
    WithEmbeddedEvent(EventHeaderWithEmbed),
    Header(EventHeader)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct EventHeaderWithEmbed {
    pub summary: String,
    pub links: Vec<LinkRelation>,
    pub id: String,
    pub data: String,
    #[serde(default)]
    #[serde(rename = "metaData")]
    pub metadata: Option<String>,
    pub event_number: usize,
    pub event_type: String,
    pub event_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct EventHeader {
    pub summary: String,
    pub links: Vec<LinkRelation>,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct StreamPage {
    pub links: Vec<LinkRelation>,
    pub head_of_stream: bool,
    pub entries: Vec<EventEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct EventEnvelope<D, M> {
    pub event_number: usize,
    pub event_type: String,
    pub event_id: Uuid,
    pub data: D,
    pub metadata: Option<M>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all="camelCase")]
pub struct AppendEvent<'a, D: 'a, M: 'a> {
    pub event_id: Uuid,
    pub event_type: &'a str,
    pub data: &'a D,
    pub metadata: Option<&'a M>,
}

#[cfg(test)]
#[path = "dto_tests.rs"]
mod tests;
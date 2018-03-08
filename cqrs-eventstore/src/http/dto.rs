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
#[serde(rename_all="camelCase")]
pub struct EventEntry {
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
    pub metadata: M,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all="camelCase")]
pub struct AppendEvent<D, M> {
    pub event_id: Uuid,
    pub event_type: String,
    pub data: D,
    pub metadata: M,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NoMetadata;

#[cfg(test)]
#[path = "dto_tests.rs"]
mod tests;
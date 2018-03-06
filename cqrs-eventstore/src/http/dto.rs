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
pub struct EventEnvelope<D> {
    event_number: usize,
    event_type: String,
    event_id: Uuid,
    data: D,
    metadata: String,
}

#[cfg(test)]
#[path = "dto_tests.rs"]
mod tests;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::application::use_cases::search_content::SearchContentResponse;
use crate::presentation::http::dto::document_dto::DocumentWithChunksDto;

#[derive(Debug, Deserialize, ToSchema)]
pub struct SearchRequestDto {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: Option<i32>,
    pub similarity_threshold: Option<f32>,
    pub file_id: Option<Uuid>,
    /// Comma-separated file ids to restrict the search to. Takes precedence
    /// over `file_id`. Present but empty matches nothing rather than widening
    /// to the whole tenant.
    #[schema(value_type = Option<String>, example = "uuid-a,uuid-b")]
    #[serde(default, deserialize_with = "comma_separated_uuids")]
    pub file_ids: Option<Vec<Uuid>>,
}

fn default_limit() -> Option<i32> {
    Some(10)
}

// Query strings carry no list type, and serde_urlencoded drops repeated keys,
// so a list arrives as one comma-separated value.
fn comma_separated_uuids<'de, D>(deserializer: D) -> Result<Option<Vec<Uuid>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let Some(raw) = Option::<String>::deserialize(deserializer)? else {
        return Ok(None);
    };
    raw.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(Uuid::parse_str)
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
        .map_err(serde::de::Error::custom)
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SearchResponseDto {
    pub query: String,
    pub documents: Vec<DocumentWithChunksDto>,
    pub total_documents: usize,
    pub total_chunk_matches: i32,
    pub search_time_ms: u64,
}

impl From<SearchContentResponse> for SearchResponseDto {
    fn from(response: SearchContentResponse) -> Self {
        let documents: Vec<DocumentWithChunksDto> = response
            .documents
            .into_iter()
            .map(DocumentWithChunksDto::from)
            .collect();
        Self {
            query: response.query,
            total_documents: documents.len(),
            total_chunk_matches: response.total_chunk_matches,
            documents,
            search_time_ms: response.search_time_ms,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SearchQueryDto {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub query_text: String,
    pub results_count: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub user_id: Option<String>,
    pub search_parameters: Option<serde_json::Value>,
}

impl From<crate::domain::entities::SearchQuery> for SearchQueryDto {
    fn from(q: crate::domain::entities::SearchQuery) -> Self {
        Self {
            id: q.id(),
            tenant_id: q.tenant_id(),
            query_text: q.query_text().to_string(),
            results_count: q.results_count(),
            created_at: q.created_at(),
            user_id: q.user_id().map(|s| s.to_string()),
            search_parameters: q.search_parameters().cloned(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SearchQueryListResponseDto {
    pub queries: Vec<SearchQueryDto>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> SearchRequestDto {
        serde_json::from_str(json).expect("request should deserialize")
    }

    #[test]
    fn absent_file_ids_searches_the_whole_tenant() {
        assert_eq!(parse(r#"{"query":"attention"}"#).file_ids, None);
    }

    // The distinction the caller depends on: an empty set restricts the search
    // to nothing, where `None` would widen it to every file in the tenant. A
    // caller that resolves a scope to zero files must not get everything back.
    #[test]
    fn present_but_empty_file_ids_matches_nothing() {
        assert_eq!(
            parse(r#"{"query":"attention","file_ids":""}"#).file_ids,
            Some(vec![])
        );
    }

    #[test]
    fn file_ids_parses_a_comma_separated_list() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let body = format!(r#"{{"query":"attention","file_ids":"{a},{b}"}}"#);
        assert_eq!(parse(&body).file_ids, Some(vec![a, b]));
    }

    #[test]
    fn surrounding_whitespace_is_tolerated() {
        let a = Uuid::new_v4();
        let body = format!(r#"{{"query":"attention","file_ids":" {a} , "}}"#);
        assert_eq!(parse(&body).file_ids, Some(vec![a]));
    }

    #[test]
    fn an_unparseable_id_fails_the_request() {
        let result = serde_json::from_str::<SearchRequestDto>(
            r#"{"query":"attention","file_ids":"not-a-uuid"}"#,
        );
        assert!(
            result.is_err(),
            "a malformed id must not be silently dropped"
        );
    }
}

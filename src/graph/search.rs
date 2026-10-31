use std::sync::Arc;

use reqwest::Method;
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};

use crate::{
    Result,
    http::{Body, RawRequest, Transport},
};

const DEFAULT_PAGE_SIZE: u32 = 25;

/// One search hit.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    /// Hit id.
    #[serde(default)]
    pub hit_id: Option<String>,
    /// Rank, 1 being the best match.
    #[serde(default)]
    pub rank: Option<u32>,
    /// Text excerpt with the matched terms.
    #[serde(default)]
    pub summary: Option<String>,
    /// The matching entity (a drive item, list item, site, ...), as returned by the service.
    #[serde(default)]
    pub resource: Value,
}

impl SearchHit {
    /// Reads the resource as a typed value, for example [`DriveItem`](crate::graph::DriveItem).
    pub fn resource_as<T: DeserializeOwned>(&self) -> Result<T> {
        Ok(serde_json::from_value(self.resource.clone())?)
    }
}

/// One page of search results.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SearchPage {
    /// Hits of this page.
    pub hits: Vec<SearchHit>,
    /// Total number of matches, when reported.
    pub total: Option<u64>,
    /// Whether more results exist after this page.
    pub more_results_available: bool,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Container {
    #[serde(default)]
    hits: Vec<SearchHit>,
    #[serde(default)]
    total: Option<u64>,
    #[serde(default)]
    more_results_available: bool,
}

#[derive(Deserialize, Default)]
struct Response {
    #[serde(default)]
    value: Vec<ResponseItem>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ResponseItem {
    #[serde(default)]
    hits_containers: Vec<Container>,
}

/// A Graph search request (`/search/query`). Get one from `GraphClient::search`.
#[derive(Debug, Clone)]
pub struct GraphSearch {
    t: Arc<Transport>,
    entity_types: Vec<String>,
    query: String,
    region: Option<String>,
    from: u32,
    size: u32,
}

impl GraphSearch {
    pub(crate) fn new(t: Arc<Transport>) -> Self {
        Self {
            t,
            entity_types: Vec::new(),
            query: String::new(),
            region: None,
            from: 0,
            size: DEFAULT_PAGE_SIZE,
        }
    }

    /// Sets the entity types to search, such as `driveItem`, `listItem` or `site`.
    pub fn entity_types<I, S>(mut self, types: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.entity_types = types.into_iter().map(Into::into).collect();
        self
    }

    /// Sets the query string (KQL).
    pub fn query(mut self, query: impl Into<String>) -> Self {
        self.query = query.into();
        self
    }

    /// Sets the search region, which the service requires with application permissions.
    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Sets the index of the first hit to return.
    pub fn from(mut self, from: u32) -> Self {
        self.from = from;
        self
    }

    /// Sets the page size (default 25).
    pub fn size(mut self, size: u32) -> Self {
        self.size = size;
        self
    }

    fn body(&self, from: u32) -> Value {
        let mut r = json!({
            "entityTypes": self.entity_types,
            "query": { "queryString": self.query },
            "from": from,
            "size": self.size,
        });
        if let Some(region) = &self.region {
            r["region"] = region.clone().into();
        }
        json!({ "requests": [r] })
    }

    async fn page(&self, from: u32) -> Result<SearchPage> {
        let mut req = RawRequest::new(Method::POST, "/search/query");
        req.body = Body::Json(self.body(from));
        let resp: Response = self.t.send(&req).await?.json()?;
        let container = resp
            .value
            .into_iter()
            .next()
            .and_then(|v| v.hits_containers.into_iter().next())
            .unwrap_or_default();
        Ok(SearchPage {
            hits: container.hits,
            total: container.total,
            more_results_available: container.more_results_available,
        })
    }

    /// Runs the search and returns the page that starts at [`from`](Self::from).
    pub async fn send(&self) -> Result<SearchPage> {
        self.page(self.from).await
    }

    /// Runs the search page by page until the service reports no more results and returns all
    /// hits, starting at [`from`](Self::from).
    pub async fn all(&self) -> Result<Vec<SearchHit>> {
        let mut hits = Vec::new();
        let mut from = self.from;
        loop {
            let page = self.page(from).await?;
            let n = page.hits.len() as u32;
            hits.extend(page.hits);
            if !page.more_results_available || n == 0 {
                return Ok(hits);
            }
            from += n;
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{body_json, method, path},
    };

    use crate::graph::{DriveItem, testutil::client};

    fn response(hits: Value, total: u64, more: bool) -> Value {
        json!({"value":[{"searchTerms":["budget"],"hitsContainers":[
            {"hits":hits,"total":total,"moreResultsAvailable":more}
        ]}]})
    }
    use serde_json::Value;

    #[tokio::test]
    async fn searches_drive_items() {
        let (c, s) = client().await;
        Mock::given(method("POST"))
            .and(path("/search/query"))
            .and(body_json(json!({"requests":[{
                "entityTypes":["driveItem"],"query":{"queryString":"budget"},"from":0,"size":25
            }]})))
            .respond_with(ResponseTemplate::new(200).set_body_json(response(
                json!([{"hitId":"h1","rank":1,"summary":"<c0>budget</c0>",
                        "resource":{"id":"i1","name":"budget.xlsx"}}]),
                1,
                false,
            )))
            .expect(1)
            .mount(&s)
            .await;
        let page = c
            .search()
            .entity_types(["driveItem"])
            .query("budget")
            .send()
            .await
            .unwrap();
        assert_eq!((page.total, page.more_results_available), (Some(1), false));
        assert_eq!(page.hits[0].rank, Some(1));
        let item: DriveItem = page.hits[0].resource_as().unwrap();
        assert_eq!(item.name.as_deref(), Some("budget.xlsx"));
    }

    #[tokio::test]
    async fn paging_advances_from_and_all_collects_every_page() {
        let (c, s) = client().await;
        for (from, hits, more) in [
            (0, json!([{"hitId":"a"},{"hitId":"b"}]), true),
            (2, json!([{"hitId":"c"},{"hitId":"d"}]), true),
            (4, json!([{"hitId":"e"}]), false),
        ] {
            Mock::given(path("/search/query"))
                .and(wiremock::matchers::body_partial_json(
                    json!({"requests":[{"from":from,"size":2,"region":"EMEA"}]}),
                ))
                .respond_with(ResponseTemplate::new(200).set_body_json(response(hits, 5, more)))
                .expect(1)
                .mount(&s)
                .await;
        }
        let hits = c
            .search()
            .entity_types(["listItem"])
            .query("x")
            .region("EMEA")
            .size(2)
            .all()
            .await
            .unwrap();
        let ids: Vec<_> = hits.iter().filter_map(|h| h.hit_id.as_deref()).collect();
        assert_eq!(ids, ["a", "b", "c", "d", "e"]);
    }

    #[tokio::test]
    async fn no_hits_is_an_empty_page() {
        let (c, s) = client().await;
        Mock::given(path("/search/query"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                json!({"value":[{"hitsContainers":[{"total":0,"moreResultsAvailable":false}]}]}),
            ))
            .mount(&s)
            .await;
        let page = c.search().query("zzz").send().await.unwrap();
        assert!(page.hits.is_empty());
        assert_eq!(page.total, Some(0));
        assert!(c.search().query("zzz").all().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn region_errors_and_bad_queries_are_http_errors() {
        let (c, s) = client().await;
        Mock::given(path("/search/query"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "error":{"code":"BadRequest","message":"Region is required"}
            })))
            .mount(&s)
            .await;
        let e = c.search().query("x").send().await.unwrap_err();
        assert_eq!(e.status(), Some(400));
        assert!(e.to_string().contains("Region is required"));
    }

    #[tokio::test]
    async fn a_response_of_the_wrong_shape_is_a_deserialization_error() {
        let (c, s) = client().await;
        Mock::given(path("/search/query"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
            .mount(&s)
            .await;
        let e = c.search().query("x").send().await.unwrap_err();
        assert!(matches!(e, crate::Error::Deserialize(_)));
    }
}

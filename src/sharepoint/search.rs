use std::{collections::HashMap, sync::Arc};

use reqwest::Method;
use serde_json::Value;

use super::Inner;
use crate::{Error, Result, http::RawRequest};

const DEFAULT_ROW_LIMIT: u32 = 50;

/// One row of SharePoint search results: the selected properties by name.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpSearchRow {
    cells: HashMap<String, Option<String>>,
}

impl SpSearchRow {
    /// The value of a property, if the row has it and it is not empty.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.cells
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .and_then(|(_, v)| v.as_deref())
    }

    /// All properties of the row.
    pub fn properties(&self) -> impl Iterator<Item = (&str, Option<&str>)> {
        self.cells.iter().map(|(k, v)| (k.as_str(), v.as_deref()))
    }
}

/// One page of SharePoint search results.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpSearchPage {
    /// Rows of this page.
    pub rows: Vec<SpSearchRow>,
    /// Number of rows in this page as counted by the service.
    pub row_count: u64,
    /// Total number of matching rows, when reported.
    pub total_rows: Option<u64>,
}

/// A SharePoint search request (`/_api/search/query`). Get one from `SharePointClient::search`.
#[derive(Debug, Clone)]
pub struct SpSearch {
    inner: Arc<Inner>,
    text: String,
    template: Option<String>,
    select: Vec<String>,
    row_limit: u32,
    start_row: u32,
    sort: Vec<String>,
    refiners: Option<String>,
    trim_duplicates: Option<bool>,
}

impl SpSearch {
    pub(crate) fn new(inner: Arc<Inner>, text: &str) -> Self {
        Self {
            inner,
            text: text.to_owned(),
            template: None,
            select: Vec::new(),
            row_limit: DEFAULT_ROW_LIMIT,
            start_row: 0,
            sort: Vec::new(),
            refiners: None,
            trim_duplicates: None,
        }
    }

    /// Sets the query template, for example `{searchTerms} IsDocument:true`.
    pub fn template(mut self, template: impl Into<String>) -> Self {
        self.template = Some(template.into());
        self
    }

    /// Sets the managed properties to return, such as `Title` and `Path`.
    pub fn select<I, S>(mut self, properties: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.select = properties.into_iter().map(Into::into).collect();
        self
    }

    /// Sets the number of rows per page (default 50).
    pub fn row_limit(mut self, n: u32) -> Self {
        self.row_limit = n;
        self
    }

    /// Sets the index of the first row to return.
    pub fn start_row(mut self, n: u32) -> Self {
        self.start_row = n;
        self
    }

    /// Adds a sort, for example `("LastModifiedTime", false)` for newest first.
    pub fn sort_by(mut self, property: &str, ascending: bool) -> Self {
        let dir = if ascending { "ascending" } else { "descending" };
        self.sort.push(format!("{property}:{dir}"));
        self
    }

    /// Sets the refiners to request, for example `FileType`.
    pub fn refiners(mut self, refiners: impl Into<String>) -> Self {
        self.refiners = Some(refiners.into());
        self
    }

    /// Turns the removal of duplicate results on or off.
    pub fn trim_duplicates(mut self, on: bool) -> Self {
        self.trim_duplicates = Some(on);
        self
    }

    fn request(&self, start_row: u32) -> RawRequest {
        let mut r = RawRequest::new(Method::GET, "/search/query");
        r.query
            .push(("querytext".into(), name_literal_raw(&self.text)));
        if let Some(t) = &self.template {
            r.query.push(("querytemplate".into(), name_literal_raw(t)));
        }
        if !self.select.is_empty() {
            r.query.push((
                "selectproperties".into(),
                name_literal_raw(&self.select.join(",")),
            ));
        }
        if !self.sort.is_empty() {
            r.query
                .push(("sortlist".into(), name_literal_raw(&self.sort.join(","))));
        }
        if let Some(x) = &self.refiners {
            r.query.push(("refiners".into(), name_literal_raw(x)));
        }
        if let Some(b) = self.trim_duplicates {
            r.query.push(("trimduplicates".into(), b.to_string()));
        }
        r.query
            .push(("rowlimit".into(), self.row_limit.to_string()));
        r.query.push(("startrow".into(), start_row.to_string()));
        r
    }

    async fn page(&self, start_row: u32) -> Result<SpSearchPage> {
        let doc: Value = self.inner.t.send(&self.request(start_row)).await?.json()?;
        parse_page(&doc)
    }

    /// Runs the search and returns the page that starts at [`start_row`](Self::start_row).
    pub async fn send(&self) -> Result<SpSearchPage> {
        self.page(self.start_row).await
    }

    /// Runs the search page by page until every matching row was read and returns them all.
    pub async fn all(&self) -> Result<Vec<SpSearchRow>> {
        let mut rows = Vec::new();
        let mut start = self.start_row;
        loop {
            let page = self.page(start).await?;
            let n = page.rows.len() as u32;
            rows.extend(page.rows);
            let done = n == 0 || page.total_rows.is_some_and(|t| u64::from(start + n) >= t);
            if done {
                return Ok(rows);
            }
            start += n;
        }
    }
}

/// Wraps a value in single quotes, doubling quotes inside it, as the search endpoint expects
/// for string parameters. The transport percent-encodes the result.
fn name_literal_raw(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// Reads an array that may be a plain array (light format) or `{ "results": [...] }` (verbose).
fn items(v: &Value) -> &[Value] {
    v.as_array()
        .or_else(|| v.get("results").and_then(Value::as_array))
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn parse_page(doc: &Value) -> Result<SpSearchPage> {
    let root = doc.pointer("/d/query").unwrap_or(doc);
    let relevant = root
        .pointer("/PrimaryQueryResult/RelevantResults")
        .ok_or_else(|| {
            Error::Deserialize(serde::de::Error::custom(
                "the search response has no PrimaryQueryResult.RelevantResults",
            ))
        })?;
    let rows = relevant
        .pointer("/Table/Rows")
        .map(items)
        .unwrap_or(&[])
        .iter()
        .map(|row| SpSearchRow {
            cells: row
                .get("Cells")
                .map(items)
                .unwrap_or(&[])
                .iter()
                .filter_map(|c| {
                    let key = c.get("Key")?.as_str()?.to_owned();
                    let value = c.get("Value").and_then(Value::as_str).map(str::to_owned);
                    Some((key, value))
                })
                .collect(),
        })
        .collect::<Vec<_>>();
    let number = |name: &str| {
        relevant.get(name).and_then(|v| {
            v.as_u64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
    };
    Ok(SpSearchPage {
        row_count: number("RowCount").unwrap_or(rows.len() as u64),
        total_rows: number("TotalRows"),
        rows,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{method, path, query_param},
    };

    use crate::sharepoint::testutil::client;

    fn light(rows: &[(&str, &str)], total: u64) -> serde_json::Value {
        json!({"PrimaryQueryResult":{"RelevantResults":{
            "RowCount": rows.len(), "TotalRows": total,
            "Table":{"Rows": rows.iter().map(|(t, p)| json!({"Cells":[
                {"Key":"Title","Value":t,"ValueType":"Edm.String"},
                {"Key":"Path","Value":p,"ValueType":"Edm.String"},
                {"Key":"Empty","Value":null,"ValueType":"Edm.String"}
            ]})).collect::<Vec<_>>()}
        }}})
    }

    #[tokio::test]
    async fn basic_query_selects_properties() {
        let (c, s) = client().await;
        Mock::given(method("GET"))
            .and(path("/_api/search/query"))
            .and(query_param("querytext", "'contract'"))
            .and(query_param("selectproperties", "'Title,Path'"))
            .and(query_param("rowlimit", "10"))
            .and(query_param("startrow", "0"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(light(&[("Deal", "https://x/deal")], 1)),
            )
            .expect(1)
            .mount(&s)
            .await;
        let page = c
            .search("contract")
            .select(["Title", "Path"])
            .row_limit(10)
            .send()
            .await
            .unwrap();
        assert_eq!((page.row_count, page.total_rows), (1, Some(1)));
        assert_eq!(page.rows[0].get("Title"), Some("Deal"));
        assert_eq!(page.rows[0].get("path"), Some("https://x/deal"));
        assert_eq!(page.rows[0].get("Empty"), None);
        assert_eq!(page.rows[0].properties().count(), 3);
    }

    #[tokio::test]
    async fn verbose_format_is_understood() {
        let (c, s) = client().await;
        Mock::given(path("/_api/search/query"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"d":{"query":{
                    "PrimaryQueryResult":{"RelevantResults":{
                        "RowCount":1,"TotalRows":"1",
                        "Table":{"Rows":{"results":[{"Cells":{"results":[
                            {"Key":"Title","Value":"Verbose","ValueType":"Edm.String"}
                        ]}}]}}
                    }}
                }}})),
            )
            .mount(&s)
            .await;
        let page = c.search("x").send().await.unwrap();
        assert_eq!(page.rows[0].get("Title"), Some("Verbose"));
        assert_eq!(page.total_rows, Some(1));
    }

    #[tokio::test]
    async fn start_row_sort_template_and_refiners_are_sent() {
        let (c, s) = client().await;
        Mock::given(path("/_api/search/query"))
            .and(query_param("startrow", "20"))
            .and(query_param("sortlist", "'LastModifiedTime:descending'"))
            .and(query_param(
                "querytemplate",
                "'{searchTerms} IsDocument:true'",
            ))
            .and(query_param("refiners", "'FileType'"))
            .and(query_param("trimduplicates", "false"))
            .respond_with(ResponseTemplate::new(200).set_body_json(light(&[], 0)))
            .expect(1)
            .mount(&s)
            .await;
        let page = c
            .search("x")
            .start_row(20)
            .sort_by("LastModifiedTime", false)
            .template("{searchTerms} IsDocument:true")
            .refiners("FileType")
            .trim_duplicates(false)
            .send()
            .await
            .unwrap();
        assert!(page.rows.is_empty());
    }

    #[tokio::test]
    async fn quotes_in_the_query_are_doubled() {
        let (c, s) = client().await;
        Mock::given(path("/_api/search/query"))
            .and(query_param("querytext", "'it''s a \"test\"'"))
            .respond_with(ResponseTemplate::new(200).set_body_json(light(&[], 0)))
            .expect(1)
            .mount(&s)
            .await;
        c.search("it's a \"test\"").send().await.unwrap();
    }

    #[tokio::test]
    async fn all_pages_are_collected() {
        let (c, s) = client().await;
        for (start, rows) in [
            ("0", vec![("a", "1"), ("b", "2")]),
            ("2", vec![("c", "3"), ("d", "4")]),
            ("4", vec![("e", "5")]),
        ] {
            Mock::given(path("/_api/search/query"))
                .and(query_param("startrow", start))
                .respond_with(ResponseTemplate::new(200).set_body_json(light(&rows, 5)))
                .expect(1)
                .mount(&s)
                .await;
        }
        let rows = c.search("x").row_limit(2).all().await.unwrap();
        let titles: Vec<_> = rows.iter().filter_map(|r| r.get("Title")).collect();
        assert_eq!(titles, ["a", "b", "c", "d", "e"]);
    }

    #[tokio::test]
    async fn bad_queries_and_odd_responses_are_errors() {
        let (c, s) = client().await;
        Mock::given(path("/_api/search/query"))
            .and(query_param("querytext", "'bad'"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "odata.error":{"code":"-1, Microsoft.Office.Server.Search","message":{"value":"syntax error"}}
            })))
            .mount(&s)
            .await;
        Mock::given(path("/_api/search/query"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"unexpected":true})))
            .mount(&s)
            .await;
        assert_eq!(
            c.search("bad").send().await.unwrap_err().status(),
            Some(400)
        );
        assert!(matches!(
            c.search("other").send().await,
            Err(crate::Error::Deserialize(_))
        ));
    }
}

use std::{marker::PhantomData, sync::Arc};

use reqwest::Method;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::{RawRequest, Transport};
use crate::Result;

/// Iterator-like helper over a paged collection, following `nextLink` continuation links.
///
/// Understands Graph (`@odata.nextLink`), SharePoint light (`odata.nextLink`) and
/// SharePoint verbose (`d.__next`) payloads.
#[derive(Debug)]
pub struct Pager<T> {
    transport: Arc<Transport>,
    next: Option<RawRequest>,
    _marker: PhantomData<fn() -> T>,
}

impl<T: DeserializeOwned> Pager<T> {
    pub(crate) fn new(transport: Arc<Transport>, first: RawRequest) -> Self {
        Self {
            transport,
            next: Some(first),
            _marker: PhantomData,
        }
    }

    /// Fetches the next page, or `None` when exhausted.
    pub async fn next_page(&mut self) -> Result<Option<Vec<T>>> {
        let Some(req) = self.next.take() else {
            return Ok(None);
        };
        let doc: Value = self.transport.send(&req).await?.json()?;
        let (items, link) = split_page(doc);
        if let Some(link) = link {
            let mut n = RawRequest::new(Method::GET, link);
            n.headers = req.headers.clone();
            self.next = Some(n);
        }
        Ok(Some(serde_json::from_value(Value::Array(items))?))
    }

    /// Fetches every remaining page and concatenates the items.
    pub async fn collect_all(mut self) -> Result<Vec<T>> {
        let mut all = Vec::new();
        while let Some(mut page) = self.next_page().await? {
            all.append(&mut page);
        }
        Ok(all)
    }
}

fn split_page(doc: Value) -> (Vec<Value>, Option<String>) {
    let link = |v: &Value| {
        ["@odata.nextLink", "odata.nextLink", "__next"]
            .iter()
            .find_map(|k| v.get(k).and_then(Value::as_str))
            .map(str::to_owned)
    };
    if let Some(d) = doc.get("d") {
        let items = d
            .get("results")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        return (items, link(d));
    }
    let items = doc
        .get("value")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    (items, link(&doc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn splits_graph_and_sharepoint() {
        let (i, l) = split_page(json!({"value":[1,2],"@odata.nextLink":"http://x"}));
        assert_eq!((i.len(), l.as_deref()), (2, Some("http://x")));
        let (i, l) = split_page(json!({"value":[1],"odata.nextLink":"http://y"}));
        assert_eq!((i.len(), l.as_deref()), (1, Some("http://y")));
        let (i, l) = split_page(json!({"d":{"results":[1,2,3],"__next":"http://z"}}));
        assert_eq!((i.len(), l.as_deref()), (3, Some("http://z")));
        let (i, l) = split_page(json!({"value":[]}));
        assert_eq!((i.len(), l), (0, None));
    }
}

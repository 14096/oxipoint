use std::sync::Arc;

use super::{Inner, Web};
use crate::{
    Result,
    http::{Pager, QueryOptions, RawRequest, query::query_methods},
};
use reqwest::Method;

/// `/web` — the site itself.
#[derive(Debug, Clone)]
pub struct WebRequest {
    inner: Arc<Inner>,
    query: QueryOptions,
}

impl WebRequest {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self {
            inner,
            query: QueryOptions::new(),
        }
    }

    fn req(&self, path: &str) -> RawRequest {
        let mut r = RawRequest::new(Method::GET, path);
        r.query = self.query.pairs().to_vec();
        r
    }

    /// Fetches the web properties.
    pub async fn get(&self) -> Result<Web> {
        self.inner.t.send(&self.req("/web")).await?.json()
    }

    /// Lists child webs.
    pub async fn subwebs(&self) -> Result<Vec<Web>> {
        Pager::new(self.inner.t.clone(), self.req("/web/webs"))
            .collect_all()
            .await
    }
}
query_methods!(WebRequest);

#[cfg(test)]
mod tests {
    use crate::sharepoint::testutil::client;
    use serde_json::json;
    use wiremock::{Mock, ResponseTemplate, matchers::path};

    #[tokio::test]
    async fn get_web_and_subwebs() {
        let (c, s) = client().await;
        Mock::given(path("/_api/web"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"Id":"g","Title":"Team","Url":"https://x/sites/team"})),
            )
            .mount(&s)
            .await;
        Mock::given(path("/_api/web/webs"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"value":[{"Id":"c","Title":"Child"}]})),
            )
            .mount(&s)
            .await;
        let w = c.web().get().await.unwrap();
        assert_eq!(
            (w.title.as_deref(), w.url.as_deref()),
            (Some("Team"), Some("https://x/sites/team"))
        );
        assert_eq!(
            c.web().subwebs().await.unwrap()[0].title.as_deref(),
            Some("Child")
        );
    }
}

use std::sync::Arc;

use serde::Serialize;
use serde_json::{Value, json};

use crate::sharepoint::inner::name_literal;
use crate::{
    Result,
    http::{Body, Pager, QueryOptions, query::query_methods},
    sharepoint::{
        Inner, NewSubscription, Subscription,
        inner::{get_req, json_body, post},
    },
};

/// The webhook subscriptions of a list.
#[derive(Debug, Clone)]
pub struct SubscriptionsRequest {
    inner: Arc<Inner>,
    list_base: String,
    query: QueryOptions,
}

impl SubscriptionsRequest {
    pub(crate) fn new(inner: Arc<Inner>, list_base: String) -> Self {
        Self {
            inner,
            list_base,
            query: QueryOptions::new(),
        }
    }

    fn base(&self) -> String {
        format!("{}/subscriptions", self.list_base)
    }

    /// Lists the subscriptions.
    pub async fn list(&self) -> Result<Vec<Subscription>> {
        Pager::new(self.inner.t.clone(), get_req(&self.base(), &self.query))
            .collect_all()
            .await
    }

    /// Adds a subscription; the watched resource is this list.
    pub async fn add(&self, new: &NewSubscription) -> Result<Subscription> {
        let mut body = json!({
            "resource": format!("{}{}", self.inner.t.base_url(), self.list_base),
            "notificationUrl": new.notification_url,
            "expirationDateTime": new.expiration_date_time,
        });
        if let (Some(state), Some(o)) = (&new.client_state, body.as_object_mut()) {
            o.insert("clientState".into(), Value::String(state.clone()));
        }
        post(&self.inner, self.base(), Body::Json(body)).await
    }

    /// Updates a subscription, e.g. `{"expirationDateTime": ...}` (MERGE).
    pub async fn update<F: Serialize>(&self, id: &str, properties: &F) -> Result<()> {
        self.inner
            .tunnel(
                "MERGE",
                format!("{}({})", self.base(), name_literal(id)),
                json_body(properties)?,
            )
            .await?;
        Ok(())
    }

    /// Deletes a subscription.
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.inner
            .tunnel(
                "DELETE",
                format!("{}({})", self.base(), name_literal(id)),
                Body::None,
            )
            .await?;
        Ok(())
    }
}
query_methods!(SubscriptionsRequest);

#[cfg(test)]
mod tests {
    use crate::sharepoint::{
        NewSubscription,
        testutil::{client, mock_get, mock_write, mount_digest},
    };
    use serde_json::json;

    const P: &str = "/_api/web/lists/getbytitle('T')/subscriptions";

    #[tokio::test]
    async fn list_add_update_delete() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        mock_get(&s, P, json!({"value":[{"id":"s1"}]})).await;
        let mut new = NewSubscription::new("https://hook", "2030-01-01T00:00:00Z");
        new.client_state = Some("st".into());
        mock_write(
            &s,
            P,
            None,
            Some(json!({
                "resource": format!("{}/_api/web/lists/getbytitle('T')", s.uri()),
                "notificationUrl":"https://hook",
                "expirationDateTime":"2030-01-01T00:00:00Z",
                "clientState":"st"
            })),
            json!({"id":"s2","notificationUrl":"https://hook"}),
        )
        .await;
        mock_write(
            &s,
            &format!("{P}('s2')"),
            Some("MERGE"),
            Some(json!({"expirationDateTime":"2031-01-01T00:00:00Z"})),
            json!(null),
        )
        .await;
        mock_write(&s, &format!("{P}('s2')"), Some("DELETE"), None, json!(null)).await;
        let subs = c.list_by_title("T").subscriptions();
        assert_eq!(subs.list().await.unwrap()[0].id, "s1");
        assert_eq!(subs.add(&new).await.unwrap().id, "s2");
        subs.update("s2", &json!({"expirationDateTime":"2031-01-01T00:00:00Z"}))
            .await
            .unwrap();
        subs.delete("s2").await.unwrap();
    }
}

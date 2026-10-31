use crate::{
    Result,
    graph::*,
    http::{Body, QueryOptions, RawRequest, Transport, encode_path},
};
use reqwest::Method;
use serde::Serialize;
use serde_json::{Value, json};
use std::sync::Arc;

/// Change-notification subscriptions on one list.
pub struct ListSubscriptionsRequest {
    pub(super) t: Arc<Transport>,
    pub(super) resource: String,
}

impl ListSubscriptionsRequest {
    /// Lists this app's subscriptions that watch the list.
    pub async fn list(&self) -> Result<Vec<Subscription>> {
        let all: Vec<Subscription> = pager(&self.t, "/subscriptions", &QueryOptions::new())
            .collect_all()
            .await?;
        Ok(all
            .into_iter()
            .filter(|s| {
                s.resource.as_deref().map(|r| r.trim_start_matches('/')) == Some(&self.resource)
            })
            .collect())
    }

    /// Subscribes to changes of the list.
    pub async fn create(&self, new: &NewSubscription) -> Result<Subscription> {
        let mut body = json!({
            "changeType": new.change_type,
            "notificationUrl": new.notification_url,
            "resource": self.resource,
            "expirationDateTime": new.expiration_date_time,
        });
        if let (Some(state), Some(o)) = (&new.client_state, body.as_object_mut()) {
            o.insert("clientState".into(), Value::String(state.clone()));
        }
        let mut req = RawRequest::new(Method::POST, "/subscriptions");
        req.body = Body::Json(body);
        self.t.send(&req).await?.json()
    }

    /// Updates a subscription (for example `{"expirationDateTime": ...}`) and returns it.
    pub async fn update<F: Serialize>(&self, id: &str, changes: &F) -> Result<Subscription> {
        let mut req = RawRequest::new(Method::PATCH, format!("/subscriptions/{}", encode_path(id)));
        req.body = Body::Json(serde_json::to_value(changes)?);
        self.t.send(&req).await?.json()
    }

    /// Deletes a subscription.
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.t
            .send(&RawRequest::new(
                Method::DELETE,
                format!("/subscriptions/{}", encode_path(id)),
            ))
            .await?;
        Ok(())
    }
}

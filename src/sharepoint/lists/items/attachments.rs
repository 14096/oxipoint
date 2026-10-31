use crate::{
    Result,
    http::{Body, RawRequest},
    sharepoint::{Attachment, inner::name_literal},
};
use bytes::Bytes;
use reqwest::Method;

collection!(
    /// The attachments of an item.
    AttachmentsRequest => Attachment
);

impl AttachmentsRequest {
    /// Attaches a file.
    pub async fn add(&self, file_name: &str, content: impl Into<Bytes>) -> Result<Attachment> {
        let mut req = RawRequest::new(
            Method::POST,
            format!("{}/add(FileName={})", self.base, name_literal(file_name)),
        );
        req.body = Body::Bytes(content.into());
        self.inner.write(req).await?.json()
    }

    /// An attachment by file name.
    pub fn by_name(&self, file_name: &str) -> AttachmentRequest {
        AttachmentRequest::new(
            self.inner.clone(),
            format!("{}({})", self.base, name_literal(file_name)),
        )
    }
}

entity!(
    /// One attachment.
    AttachmentRequest => Attachment
);

impl AttachmentRequest {
    /// Deletes the attachment.
    pub async fn delete(&self) -> Result<()> {
        self.inner
            .tunnel("DELETE", self.base.clone(), Body::None)
            .await?;
        Ok(())
    }
}

/// Defines a handle over a paged collection of `$item`.
macro_rules! collection {
    ($(#[$m:meta])* $name:ident => $item:ty) => {
        $(#[$m])*
        #[derive(Debug, Clone)]
        pub struct $name {
            t: ::std::sync::Arc<$crate::http::Transport>,
            base: String,
            query: $crate::http::QueryOptions,
        }

        impl $name {
            pub(crate) fn new(t: ::std::sync::Arc<$crate::http::Transport>, base: String) -> Self {
                Self { t, base, query: $crate::http::QueryOptions::new() }
            }

            /// Lists all items, following paging links.
            pub async fn list(&self) -> $crate::Result<Vec<$item>> {
                self.pages().collect_all().await
            }

            /// Returns a pager.
            pub fn pages(&self) -> $crate::http::Pager<$item> {
                $crate::graph::pager(&self.t, &self.base, &self.query)
            }
        }
        $crate::http::query::query_methods!($name);
    };
}

/// Adds `create` (POST of a definition to the collection) to a handle made by `collection!`.
macro_rules! creatable {
    ($name:ident => $item:ty) => {
        impl $name {
            /// Creates an item from a definition and returns it.
            pub async fn create<F: ::serde::Serialize>(
                &self,
                definition: &F,
            ) -> $crate::Result<$item> {
                let mut req =
                    $crate::http::RawRequest::new(::reqwest::Method::POST, self.base.as_str());
                req.body = $crate::http::Body::Json(serde_json::to_value(definition)?);
                self.t.send(&req).await?.json()
            }
        }
    };
}

/// Defines a handle over a single `$item` that can be read and deleted.
macro_rules! entity {
    ($(#[$m:meta])* $name:ident => $item:ty) => {
        $(#[$m])*
        #[derive(Debug, Clone)]
        pub struct $name {
            t: ::std::sync::Arc<$crate::http::Transport>,
            base: String,
            query: $crate::http::QueryOptions,
        }

        impl $name {
            pub(crate) fn new(t: ::std::sync::Arc<$crate::http::Transport>, base: String) -> Self {
                Self { t, base, query: $crate::http::QueryOptions::new() }
            }

            /// Fetches the entity.
            pub async fn get(&self) -> $crate::Result<$item> {
                $crate::graph::get_json(&self.t, &self.base, &self.query).await
            }

            /// Deletes the entity.
            pub async fn delete(&self) -> $crate::Result<()> {
                self.t
                    .send(&$crate::http::RawRequest::new(
                        ::reqwest::Method::DELETE,
                        self.base.as_str(),
                    ))
                    .await?;
                Ok(())
            }
        }
        $crate::http::query::query_methods!($name);
    };
}

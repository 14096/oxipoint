/// Common OData query options (`$select`, `$filter`, ...).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QueryOptions {
    pairs: Vec<(String, String)>,
}

impl QueryOptions {
    /// Creates an empty set of options.
    pub fn new() -> Self {
        Self::default()
    }

    fn set(&mut self, key: &str, value: String) {
        self.pairs.retain(|(k, _)| k != key);
        self.pairs.push((key.to_owned(), value));
    }

    /// Sets `$select` to the given comma separated fields.
    pub fn select<I, S>(mut self, fields: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let v = fields
            .into_iter()
            .map(|s| s.as_ref().to_owned())
            .collect::<Vec<_>>()
            .join(",");
        self.set("$select", v);
        self
    }

    /// Sets `$filter`.
    pub fn filter(mut self, expr: impl Into<String>) -> Self {
        self.set("$filter", expr.into());
        self
    }

    /// Sets `$expand`.
    pub fn expand(mut self, expr: impl Into<String>) -> Self {
        self.set("$expand", expr.into());
        self
    }

    /// Sets `$orderby`.
    pub fn order_by(mut self, expr: impl Into<String>) -> Self {
        self.set("$orderby", expr.into());
        self
    }

    /// Sets `$top`.
    pub fn top(mut self, n: u32) -> Self {
        self.set("$top", n.to_string());
        self
    }

    /// Sets `$search` (Graph only).
    pub fn search(mut self, text: impl Into<String>) -> Self {
        self.set("$search", text.into());
        self
    }

    /// Returns the options as key/value pairs.
    pub fn pairs(&self) -> &[(String, String)] {
        &self.pairs
    }
}

/// Adds the OData query methods (`select`, `filter`, ...) to a builder struct that has a
/// `query: QueryOptions` field.
#[allow(unused_macros)]
macro_rules! query_methods {
    ($t:ty) => {
        impl $t {
            /// Sets `$select`.
            pub fn select<I, S>(mut self, fields: I) -> Self
            where
                I: IntoIterator<Item = S>,
                S: AsRef<str>,
            {
                self.query = self.query.select(fields);
                self
            }
            /// Sets `$filter`.
            pub fn filter(mut self, expr: impl Into<String>) -> Self {
                self.query = self.query.filter(expr);
                self
            }
            /// Sets `$expand`.
            pub fn expand(mut self, expr: impl Into<String>) -> Self {
                self.query = self.query.expand(expr);
                self
            }
            /// Sets `$orderby`.
            pub fn order_by(mut self, expr: impl Into<String>) -> Self {
                self.query = self.query.order_by(expr);
                self
            }
            /// Sets `$top`.
            pub fn top(mut self, n: u32) -> Self {
                self.query = self.query.top(n);
                self
            }
            /// Sets `$search`.
            pub fn search(mut self, text: impl Into<String>) -> Self {
                self.query = self.query.search(text);
                self
            }
        }
    };
}
#[allow(unused_imports)]
pub(crate) use query_methods;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_pairs_and_overrides() {
        let q = QueryOptions::new()
            .select(["id", "name"])
            .top(5)
            .top(7)
            .filter("a eq 1");
        assert_eq!(
            q.pairs(),
            &[
                ("$select".to_string(), "id,name".to_string()),
                ("$top".to_string(), "7".to_string()),
                ("$filter".to_string(), "a eq 1".to_string())
            ]
        );
    }
}

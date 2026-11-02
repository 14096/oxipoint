//! Typed list item column values, shared by the SharePoint and Graph clients.
//!
//! Build values with [`Fields`] and pick the back end with [`Fields::for_sharepoint`] or
//! [`Fields::for_graph`]; the result serializes to the form that back end expects and can be
//! passed wherever a serializable set of column values is accepted.
//!
//! ```
//! use oxipoint::fields::Fields;
//! let f = Fields::new().text("Title", "Write docs").lookup("Project", 3);
//! let sp = serde_json::to_value(f.for_sharepoint()).unwrap();
//! let graph = serde_json::to_value(f.for_graph()).unwrap();
//! assert_eq!(sp["ProjectId"], 3);
//! assert_eq!(graph["ProjectLookupId"], "3");
//! ```
//!
//! Managed metadata values have a SharePoint form for a single term only, and none on Graph;
//! serializing an unsupported kind fails and nothing is sent.

use serde::{Serialize, Serializer, ser::Error as _};
use serde_json::{Map, Value, json};
use std::fmt;

use crate::{Error, Result};

/// One column value.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    /// Single line or multi line text.
    Text(String),
    /// Number or currency.
    Number(f64),
    /// Yes/No.
    Bool(bool),
    /// Date and time as ISO 8601 text.
    DateTime(String),
    /// A single choice.
    Choice(String),
    /// Several choices.
    MultiChoice(Vec<String>),
    /// Hyperlink or picture.
    Url {
        /// Address.
        address: String,
        /// Display text.
        description: Option<String>,
    },
    /// A lookup to the item with this id.
    Lookup(i64),
    /// A multi-value lookup.
    MultiLookup(Vec<i64>),
    /// A person or group, by id in the site's user information list.
    Person(i64),
    /// Several people or groups.
    MultiPerson(Vec<i64>),
    /// A managed metadata term.
    Taxonomy {
        /// Term label.
        label: String,
        /// Term GUID.
        term_guid: String,
    },
    /// Several managed metadata terms.
    MultiTaxonomy(Vec<(String, String)>),
}

/// The back end a value set is serialized for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// SharePoint REST (JSON light).
    SharePoint,
    /// Microsoft Graph.
    Graph,
}

/// A builder for a set of column values.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Fields {
    entries: Vec<(String, FieldValue)>,
}

macro_rules! adder {
    ($(#[$m:meta])* $name:ident($($arg:ident: $ty:ty),*) => $value:expr) => {
        $(#[$m])*
        pub fn $name(self, column: &str, $($arg: $ty),*) -> Self {
            self.push(column, $value)
        }
    };
}

impl Fields {
    /// An empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a value for `column` (its internal name), replacing an earlier one.
    pub fn push(mut self, column: &str, value: FieldValue) -> Self {
        self.entries.retain(|(c, _)| c != column);
        self.entries.push((column.to_owned(), value));
        self
    }

    adder!(/// Adds a text value.
        text(v: impl Into<String>) => FieldValue::Text(v.into()));
    adder!(/// Adds a number.
        number(v: f64) => FieldValue::Number(v));
    adder!(/// Adds a boolean.
        bool(v: bool) => FieldValue::Bool(v));
    adder!(/// Adds a date and time (ISO 8601 text).
        datetime(v: impl Into<String>) => FieldValue::DateTime(v.into()));
    adder!(/// Adds a single choice.
        choice(v: impl Into<String>) => FieldValue::Choice(v.into()));
    adder!(/// Adds several choices.
        multi_choice(v: Vec<String>) => FieldValue::MultiChoice(v));
    adder!(/// Adds a lookup by item id.
        lookup(id: i64) => FieldValue::Lookup(id));
    adder!(/// Adds a multi-value lookup.
        multi_lookup(ids: Vec<i64>) => FieldValue::MultiLookup(ids));
    adder!(/// Adds a person or group by user id.
        person(id: i64) => FieldValue::Person(id));
    adder!(/// Adds several people or groups.
        multi_person(ids: Vec<i64>) => FieldValue::MultiPerson(ids));
    adder!(/// Adds a managed metadata term.
        taxonomy(label: impl Into<String>, term_guid: impl Into<String>) =>
            FieldValue::Taxonomy { label: label.into(), term_guid: term_guid.into() });

    /// Adds a hyperlink.
    pub fn url(self, column: &str, address: impl Into<String>, description: Option<&str>) -> Self {
        self.push(
            column,
            FieldValue::Url {
                address: address.into(),
                description: description.map(str::to_owned),
            },
        )
    }

    /// Whether no value was added.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The values serialized for SharePoint REST.
    pub fn for_sharepoint(&self) -> TargetFields<'_> {
        TargetFields {
            fields: self,
            target: Target::SharePoint,
        }
    }

    /// The values serialized for Microsoft Graph.
    pub fn for_graph(&self) -> TargetFields<'_> {
        TargetFields {
            fields: self,
            target: Target::Graph,
        }
    }

    /// Encodes the values for `target`, failing for kinds the target cannot express.
    pub fn to_value(&self, target: Target) -> Result<Value> {
        let mut out = Map::new();
        for (column, value) in &self.entries {
            encode(column, value, target, &mut out).map_err(Error::Config)?;
        }
        Ok(Value::Object(out))
    }

    /// The values as the formatted text SharePoint's validated writes expect.
    pub fn form_values(&self) -> Result<Vec<(String, String)>> {
        self.entries
            .iter()
            .map(|(c, v)| Ok((c.clone(), form_text(c, v).map_err(Error::Config)?)))
            .collect()
    }
}

/// A [`Fields`] set bound to a back end; serializes to that back end's JSON form.
#[derive(Debug, Clone, Copy)]
pub struct TargetFields<'a> {
    fields: &'a Fields,
    target: Target,
}

impl Serialize for TargetFields<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        self.fields
            .to_value(self.target)
            .map_err(S::Error::custom)?
            .serialize(s)
    }
}

fn number(n: f64, column: &str) -> std::result::Result<Value, String> {
    serde_json::Number::from_f64(n)
        .map(Value::Number)
        .ok_or_else(|| format!("column {column}: number is not finite"))
}

fn collection(kind: &str, items: Vec<Value>) -> Value {
    json!({ "__metadata": { "type": format!("Collection({kind})") }, "results": items })
}

fn encode(
    column: &str,
    value: &FieldValue,
    target: Target,
    out: &mut Map<String, Value>,
) -> std::result::Result<(), String> {
    use FieldValue::*;
    let sp = target == Target::SharePoint;
    match value {
        Text(s) | Choice(s) | DateTime(s) => {
            out.insert(column.into(), s.clone().into());
        }
        Number(n) => {
            out.insert(column.into(), number(*n, column)?);
        }
        Bool(b) => {
            out.insert(column.into(), (*b).into());
        }
        MultiChoice(v) => {
            let items: Vec<Value> = v.iter().cloned().map(Value::String).collect();
            if sp {
                out.insert(column.into(), collection("Edm.String", items));
            } else {
                out.insert(
                    format!("{column}@odata.type"),
                    "Collection(Edm.String)".into(),
                );
                out.insert(column.into(), Value::Array(items));
            }
        }
        Url {
            address,
            description,
        } => {
            let mut o = Map::new();
            if sp {
                o.insert("__metadata".into(), json!({"type":"SP.FieldUrlValue"}));
            }
            o.insert("Url".into(), address.clone().into());
            o.insert(
                "Description".into(),
                description
                    .clone()
                    .unwrap_or_else(|| address.clone())
                    .into(),
            );
            out.insert(column.into(), Value::Object(o));
        }
        Lookup(id) | Person(id) => {
            if sp {
                out.insert(format!("{column}Id"), (*id).into());
            } else {
                out.insert(format!("{column}LookupId"), id.to_string().into());
            }
        }
        MultiLookup(ids) | MultiPerson(ids) => {
            if sp {
                let items = ids.iter().map(|i| Value::from(*i)).collect();
                out.insert(format!("{column}Id"), collection("Edm.Int32", items));
            } else {
                let key = format!("{column}LookupId");
                out.insert(format!("{key}@odata.type"), "Collection(Edm.String)".into());
                out.insert(
                    key,
                    Value::Array(ids.iter().map(|i| i.to_string().into()).collect()),
                );
            }
        }
        Taxonomy { label, term_guid } if sp => {
            out.insert(
                column.into(),
                json!({
                    "__metadata": {"type": "SP.Taxonomy.TaxonomyFieldValue"},
                    "Label": label, "TermGuid": term_guid, "WssId": -1
                }),
            );
        }
        Taxonomy { .. } | MultiTaxonomy(_) => {
            return Err(format!(
                "column {column}: managed metadata has no form for {}",
                if sp {
                    "multiple terms on SharePoint"
                } else {
                    "Graph"
                }
            ));
        }
    }
    Ok(())
}

fn form_text(column: &str, value: &FieldValue) -> std::result::Result<String, String> {
    use FieldValue::*;
    Ok(match value {
        Text(s) | Choice(s) | DateTime(s) => s.clone(),
        Number(n) => n.to_string(),
        Bool(b) => if *b { "1" } else { "0" }.to_owned(),
        MultiChoice(v) => format!(";#{};#", v.join(";#")),
        Url {
            address,
            description,
        } => format!("{address}, {}", description.as_deref().unwrap_or(address)),
        Lookup(id) => id.to_string(),
        Taxonomy { label, term_guid } => format!("{label}|{term_guid}"),
        MultiLookup(_) | Person(_) | MultiPerson(_) | MultiTaxonomy(_) => {
            return Err(format!(
                "column {column}: this kind cannot be sent as form text"
            ));
        }
    })
}

/// A parsed ISO 8601 date and time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateTime {
    /// Year.
    pub year: i32,
    /// Month, 1 to 12.
    pub month: u8,
    /// Day of month.
    pub day: u8,
    /// Hour.
    pub hour: u8,
    /// Minute.
    pub minute: u8,
    /// Second.
    pub second: u8,
    /// Offset from UTC in minutes (0 for `Z` or when absent).
    pub offset_minutes: i32,
}

impl DateTime {
    /// Parses `YYYY-MM-DDTHH:MM:SS` with optional fraction and `Z` or `±HH:MM` suffix.
    pub fn parse(s: &str) -> Option<Self> {
        let b = s.as_bytes();
        if b.len() < 19 || b[4] != b'-' || b[7] != b'-' || !matches!(b[10], b'T' | b't' | b' ') {
            return None;
        }
        if b[13] != b':' || b[16] != b':' {
            return None;
        }
        let num = |a: usize, z: usize| -> Option<i32> {
            let t = s.get(a..z)?;
            t.bytes()
                .all(|c| c.is_ascii_digit())
                .then(|| t.parse().ok())?
        };
        let (year, month, day) = (num(0, 4)?, num(5, 7)?, num(8, 10)?);
        let (hour, minute, second) = (num(11, 13)?, num(14, 16)?, num(17, 19)?);
        if !(1..=12).contains(&month)
            || !(1..=31).contains(&day)
            || hour > 23
            || minute > 59
            || second > 60
        {
            return None;
        }
        let mut i = 19;
        if b.get(i) == Some(&b'.') {
            i += 1;
            let start = i;
            while b.get(i).is_some_and(u8::is_ascii_digit) {
                i += 1;
            }
            if i == start {
                return None;
            }
        }
        let offset_minutes = match &s[i..] {
            "" | "Z" | "z" => 0,
            tz if tz.len() == 6
                && matches!(tz.as_bytes()[0], b'+' | b'-')
                && tz.as_bytes()[3] == b':' =>
            {
                let h: i32 = tz.get(1..3)?.parse().ok()?;
                let m: i32 = tz.get(4..6)?.parse().ok()?;
                if h > 23 || m > 59 {
                    return None;
                }
                let v = h * 60 + m;
                if tz.starts_with('-') { -v } else { v }
            }
            _ => return None,
        };
        Some(Self {
            year,
            month: month as u8,
            day: day as u8,
            hour: hour as u8,
            minute: minute as u8,
            second: second as u8,
            offset_minutes,
        })
    }

    /// Seconds since 1970-01-01T00:00:00Z.
    pub fn unix_seconds(&self) -> i64 {
        let y = i64::from(self.year) - i64::from(self.month <= 2);
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let m = i64::from(self.month);
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(self.day) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        let days = era * 146_097 + doe - 719_468;
        days * 86_400
            + i64::from(self.hour) * 3600
            + i64::from(self.minute) * 60
            + i64::from(self.second)
            - i64::from(self.offset_minutes) * 60
    }
}

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )?;
        if self.offset_minutes == 0 {
            write!(f, "Z")
        } else {
            let o = self.offset_minutes.abs();
            let sign = if self.offset_minutes < 0 { '-' } else { '+' };
            write!(f, "{sign}{:02}:{:02}", o / 60, o % 60)
        }
    }
}

/// A lookup value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lookup {
    /// Id of the looked-up item.
    pub id: i64,
    /// Display value, when returned.
    pub value: Option<String>,
}

/// A person or group value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    /// Id in the site's user information list.
    pub id: i64,
    /// Display name, when returned.
    pub name: Option<String>,
}

/// A hyperlink value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlValue {
    /// Address.
    pub address: String,
    /// Display text.
    pub description: Option<String>,
}

/// A managed metadata term value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Taxonomy {
    /// Term label.
    pub label: String,
    /// Term GUID, when returned.
    pub term_guid: Option<String>,
}

pub(crate) type Columns = Map<String, Value>;

pub(crate) fn text(m: &Columns, col: &str) -> Option<String> {
    m.get(col)?.as_str().map(str::to_owned)
}

pub(crate) fn number_of(m: &Columns, col: &str) -> Option<f64> {
    m.get(col)?.as_f64()
}

pub(crate) fn bool_of(m: &Columns, col: &str) -> Option<bool> {
    m.get(col)?.as_bool()
}

pub(crate) fn datetime(m: &Columns, col: &str) -> Option<DateTime> {
    DateTime::parse(m.get(col)?.as_str()?)
}

fn int(v: &Value) -> Option<i64> {
    v.as_i64().or_else(|| v.as_str()?.trim().parse().ok())
}

fn shown(o: &Columns) -> Option<String> {
    ["LookupValue", "Title", "Name"]
        .iter()
        .find_map(|k| o.get(*k)?.as_str().map(str::to_owned))
}

fn id_and_value(m: &Columns, col: &str) -> Option<(i64, Option<String>)> {
    let display = |m: &Columns| match m.get(col) {
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Object(o)) => shown(o),
        _ => None,
    };
    for key in [format!("{col}LookupId"), format!("{col}Id")] {
        if let Some(id) = m.get(&key).and_then(int) {
            return Some((id, display(m)));
        }
    }
    let o = m.get(col)?.as_object()?;
    let id = ["Id", "LookupId", "ID"]
        .iter()
        .find_map(|k| o.get(*k).and_then(int))?;
    Some((id, shown(o)))
}

pub(crate) fn lookup(m: &Columns, col: &str) -> Option<Lookup> {
    id_and_value(m, col).map(|(id, value)| Lookup { id, value })
}

pub(crate) fn person(m: &Columns, col: &str) -> Option<Person> {
    id_and_value(m, col).map(|(id, name)| Person { id, name })
}

pub(crate) fn url(m: &Columns, col: &str) -> Option<UrlValue> {
    let o = m.get(col)?.as_object()?;
    Some(UrlValue {
        address: o.get("Url")?.as_str()?.to_owned(),
        description: o
            .get("Description")
            .and_then(Value::as_str)
            .map(str::to_owned),
    })
}

pub(crate) fn taxonomy(m: &Columns, col: &str) -> Option<Taxonomy> {
    let o = m.get(col)?.as_object()?;
    Some(Taxonomy {
        label: o.get("Label")?.as_str()?.to_owned(),
        term_guid: o.get("TermGuid").and_then(Value::as_str).map(str::to_owned),
    })
}

/// Adds the typed readers to an item type; `$cols` yields its `Option<&Map<String, Value>>`.
#[allow(unused_macros)]
macro_rules! item_readers {
    ($ty:ty, $s:ident => $cols:expr) => {
        impl $ty {
            /// Reads a text column; `None` when missing or not text.
            pub fn text(&self, column: &str) -> Option<String> {
                let $s = self;
                $crate::fields::text($cols?, column)
            }
            /// Reads a number column; `None` when missing or not a number.
            pub fn number(&self, column: &str) -> Option<f64> {
                let $s = self;
                $crate::fields::number_of($cols?, column)
            }
            /// Reads a boolean column; `None` when missing or not a boolean.
            pub fn bool(&self, column: &str) -> Option<bool> {
                let $s = self;
                $crate::fields::bool_of($cols?, column)
            }
            /// Reads a date-time column holding ISO 8601 text.
            pub fn datetime(&self, column: &str) -> Option<$crate::fields::DateTime> {
                let $s = self;
                $crate::fields::datetime($cols?, column)
            }
            /// Reads a hyperlink column.
            pub fn url(&self, column: &str) -> Option<$crate::fields::UrlValue> {
                let $s = self;
                $crate::fields::url($cols?, column)
            }
            /// Reads a lookup column (`<Column>Id`, `<Column>LookupId` or an expanded object).
            pub fn lookup(&self, column: &str) -> Option<$crate::fields::Lookup> {
                let $s = self;
                $crate::fields::lookup($cols?, column)
            }
            /// Reads a person or group column.
            pub fn person(&self, column: &str) -> Option<$crate::fields::Person> {
                let $s = self;
                $crate::fields::person($cols?, column)
            }
            /// Reads a managed metadata column holding a single term.
            pub fn taxonomy(&self, column: &str) -> Option<$crate::fields::Taxonomy> {
                let $s = self;
                $crate::fields::taxonomy($cols?, column)
            }
        }
    };
}
#[allow(unused_imports)]
pub(crate) use item_readers;

#[cfg(test)]
mod tests {
    use super::*;

    fn both(f: &Fields) -> (Value, Value) {
        (
            serde_json::to_value(f.for_sharepoint()).unwrap(),
            serde_json::to_value(f.for_graph()).unwrap(),
        )
    }

    #[test]
    fn primitives_are_identical_on_both_targets() {
        let f = Fields::new()
            .text("Title", "a")
            .number("N", 2.5)
            .bool("Done", true)
            .datetime("Due", "2030-01-02T03:04:05Z")
            .choice("Status", "Open");
        let (sp, g) = both(&f);
        assert_eq!(sp, g);
        assert_eq!(
            sp,
            json!({"Title":"a","N":2.5,"Done":true,"Due":"2030-01-02T03:04:05Z","Status":"Open"})
        );
    }

    #[test]
    fn lookup_and_person_forms() {
        let f = Fields::new().lookup("Project", 3).person("Owner", 9);
        let (sp, g) = both(&f);
        assert_eq!(sp, json!({"ProjectId":3,"OwnerId":9}));
        assert_eq!(g, json!({"ProjectLookupId":"3","OwnerLookupId":"9"}));
    }

    #[test]
    fn multi_lookup_and_multi_person_forms() {
        let f = Fields::new()
            .multi_lookup("P", vec![1, 2])
            .multi_person("O", vec![4]);
        let (sp, g) = both(&f);
        assert_eq!(
            sp["PId"],
            json!({"__metadata":{"type":"Collection(Edm.Int32)"},"results":[1,2]})
        );
        assert_eq!(sp["OId"]["results"], json!([4]));
        assert_eq!(g["PLookupId@odata.type"], "Collection(Edm.String)");
        assert_eq!(g["PLookupId"], json!(["1", "2"]));
        assert_eq!(g["OLookupId"], json!(["4"]));
    }

    #[test]
    fn multi_choice_uses_collection_form() {
        let f = Fields::new().multi_choice("Tags", vec!["a".into(), "b".into()]);
        let (sp, g) = both(&f);
        assert_eq!(
            sp["Tags"],
            json!({"__metadata":{"type":"Collection(Edm.String)"},"results":["a","b"]})
        );
        assert_eq!(
            g,
            json!({"Tags@odata.type":"Collection(Edm.String)","Tags":["a","b"]})
        );
    }

    #[test]
    fn url_forms() {
        let f = Fields::new().url("Link", "https://x", Some("X"));
        let (sp, g) = both(&f);
        assert_eq!(
            sp["Link"],
            json!({"__metadata":{"type":"SP.FieldUrlValue"},"Url":"https://x","Description":"X"})
        );
        assert_eq!(g["Link"], json!({"Url":"https://x","Description":"X"}));
        let (_, g) = both(&Fields::new().url("Link", "https://x", None));
        assert_eq!(g["Link"]["Description"], "https://x");
    }

    #[test]
    fn taxonomy_single_on_sharepoint_only() {
        let f = Fields::new().taxonomy("Dept", "HR", "guid-1");
        let sp = serde_json::to_value(f.for_sharepoint()).unwrap();
        assert_eq!(sp["Dept"]["TermGuid"], "guid-1");
        assert_eq!(sp["Dept"]["WssId"], -1);
        assert!(serde_json::to_value(f.for_graph()).is_err());
    }

    #[test]
    fn unsupported_kinds_fail_and_name_the_column() {
        let f = Fields::new().push(
            "Dept",
            FieldValue::MultiTaxonomy(vec![("a".into(), "g".into())]),
        );
        for t in [Target::SharePoint, Target::Graph] {
            let e = f.to_value(t).unwrap_err();
            assert!(matches!(e, Error::Config(ref m) if m.contains("Dept")));
        }
        assert!(serde_json::to_value(f.for_sharepoint()).is_err());
        let nan = Fields::new().number("N", f64::NAN);
        assert!(nan.to_value(Target::Graph).is_err());
    }

    #[test]
    fn later_value_replaces_earlier() {
        let f = Fields::new().text("A", "1").text("A", "2");
        assert_eq!(f.to_value(Target::Graph).unwrap(), json!({"A":"2"}));
        assert!(!f.is_empty() && Fields::new().is_empty());
    }

    #[test]
    fn form_values_are_text() {
        let f = Fields::new()
            .text("Title", "t")
            .bool("Done", true)
            .number("N", 3.0)
            .lookup("P", 4)
            .multi_choice("Tags", vec!["a".into(), "b".into()])
            .taxonomy("Dept", "HR", "g");
        let v: std::collections::HashMap<_, _> = f.form_values().unwrap().into_iter().collect();
        assert_eq!(v["Done"], "1");
        assert_eq!(v["N"], "3");
        assert_eq!(v["P"], "4");
        assert_eq!(v["Tags"], ";#a;#b;#");
        assert_eq!(v["Dept"], "HR|g");
        assert!(Fields::new().person("O", 1).form_values().is_err());
    }

    fn cols(v: Value) -> Columns {
        v.as_object().unwrap().clone()
    }

    #[test]
    fn readers_return_none_for_missing_and_wrong_kind() {
        let m = cols(json!({"Title":"x","N":5,"Done":false}));
        assert_eq!(text(&m, "Title").as_deref(), Some("x"));
        assert_eq!(text(&m, "Missing"), None);
        assert_eq!(number_of(&m, "Title"), None);
        assert_eq!(text(&m, "N"), None);
        assert_eq!(number_of(&m, "N"), Some(5.0));
        assert_eq!(bool_of(&m, "Done"), Some(false));
        assert_eq!(bool_of(&m, "N"), None);
        assert!(datetime(&m, "Title").is_none());
        assert!(lookup(&m, "Title").is_none() && url(&m, "Title").is_none());
        assert!(person(&m, "Missing").is_none() && taxonomy(&m, "N").is_none());
    }

    #[test]
    fn lookup_and_person_readers_accept_both_back_ends() {
        let sp = cols(json!({"ProjectId":3,"Project":{"Id":3,"Title":"Apollo"}}));
        let l = lookup(&sp, "Project").unwrap();
        assert_eq!((l.id, l.value.as_deref()), (3, Some("Apollo")));
        let g = cols(json!({"ProjectLookupId":"7","Project":"Gemini"}));
        let l = lookup(&g, "Project").unwrap();
        assert_eq!((l.id, l.value.as_deref()), (7, Some("Gemini")));
        let expanded = cols(json!({"Author":{"Id":9,"Title":"Ann"}}));
        let p = person(&expanded, "Author").unwrap();
        assert_eq!((p.id, p.name.as_deref()), (9, Some("Ann")));
        assert_eq!(
            person(&cols(json!({"AuthorLookupId":"4"})), "Author")
                .unwrap()
                .id,
            4
        );
    }

    #[test]
    fn url_and_taxonomy_readers() {
        let m = cols(
            json!({"Link":{"Url":"https://x","Description":"X"},"Dept":{"Label":"HR","TermGuid":"g","WssId":-1}}),
        );
        let u = url(&m, "Link").unwrap();
        assert_eq!(
            (u.address.as_str(), u.description.as_deref()),
            ("https://x", Some("X"))
        );
        let t = taxonomy(&m, "Dept").unwrap();
        assert_eq!(
            (t.label.as_str(), t.term_guid.as_deref()),
            ("HR", Some("g"))
        );
    }

    #[test]
    fn iso_date_times_parse() {
        let d = DateTime::parse("2024-02-29T10:30:15Z").unwrap();
        assert_eq!(
            (d.year, d.month, d.day, d.hour, d.minute, d.second),
            (2024, 2, 29, 10, 30, 15)
        );
        assert_eq!(d.unix_seconds(), 1_709_202_615);
        assert_eq!(d.to_string(), "2024-02-29T10:30:15Z");
        let z = DateTime::parse("1970-01-01T00:00:00.123+01:00").unwrap();
        assert_eq!(z.unix_seconds(), -3600);
        assert_eq!(z.to_string(), "1970-01-01T00:00:00+01:00");
        assert_eq!(
            DateTime::parse("2020-01-01T00:00:00")
                .unwrap()
                .offset_minutes,
            0
        );
        for bad in [
            "",
            "yesterday",
            "2020-13-01T00:00:00Z",
            "2020-01-01T25:00:00Z",
            "2020-01-01T00:00:00+1",
            "2020-01-01T00:00:00.Z",
        ] {
            assert!(DateTime::parse(bad).is_none(), "{bad}");
        }
        let m = cols(json!({"Due":"2030-01-02T03:04:05Z"}));
        assert_eq!(datetime(&m, "Due").unwrap().year, 2030);
    }
}

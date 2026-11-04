//! Builds OData `$filter` expressions with correct quoting.
//!
//! String values are quoted and their single quotes doubled, so user input cannot change the
//! meaning of a filter. Use the [`filter!`](crate::filter!) macro or the [`Filter`] builder and
//! pass the result to any `filter` method.
//!
//! ```
//! use oxipoint::filter;
//! let name = "O'Brien";
//! let f = filter!(Status == "Open" && (Priority >= 3 || Urgent == true) && Author == (name));
//! assert_eq!(
//!     f.as_str(),
//!     "Status eq 'Open' and (Priority ge 3 or Urgent eq true) and Author eq 'O''Brien'"
//! );
//! ```

use std::fmt;

/// A value that can appear on the right hand side of a comparison.
pub trait ODataValue {
    /// The OData literal for this value.
    fn to_odata(&self) -> String;
}

impl ODataValue for str {
    fn to_odata(&self) -> String {
        format!("'{}'", self.replace('\'', "''"))
    }
}

impl ODataValue for String {
    fn to_odata(&self) -> String {
        self.as_str().to_odata()
    }
}

impl ODataValue for bool {
    fn to_odata(&self) -> String {
        self.to_string()
    }
}

impl<T: ODataValue + ?Sized> ODataValue for &T {
    fn to_odata(&self) -> String {
        (**self).to_odata()
    }
}

macro_rules! numeric {
    ($($t:ty),*) => {$(
        impl ODataValue for $t {
            fn to_odata(&self) -> String {
                self.to_string()
            }
        }
    )*};
}
numeric!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Leaf,
    And,
    Or,
}

/// A `$filter` expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    text: String,
    kind: Kind,
}

impl Filter {
    /// A comparison with an explicit OData operator (`eq`, `ne`, `gt`, `ge`, `lt`, `le`).
    pub fn cmp(column: &str, op: &str, value: impl ODataValue) -> Self {
        Self {
            text: format!("{column} {op} {}", value.to_odata()),
            kind: Kind::Leaf,
        }
    }

    /// `column eq value`
    pub fn eq(column: &str, value: impl ODataValue) -> Self {
        Self::cmp(column, "eq", value)
    }

    /// `column ne value`
    pub fn ne(column: &str, value: impl ODataValue) -> Self {
        Self::cmp(column, "ne", value)
    }

    /// `column gt value`
    pub fn gt(column: &str, value: impl ODataValue) -> Self {
        Self::cmp(column, "gt", value)
    }

    /// `column ge value`
    pub fn ge(column: &str, value: impl ODataValue) -> Self {
        Self::cmp(column, "ge", value)
    }

    /// `column lt value`
    pub fn lt(column: &str, value: impl ODataValue) -> Self {
        Self::cmp(column, "lt", value)
    }

    /// `column le value`
    pub fn le(column: &str, value: impl ODataValue) -> Self {
        Self::cmp(column, "le", value)
    }

    /// `startswith(column, 'prefix')`
    pub fn starts_with(column: &str, prefix: &str) -> Self {
        Self {
            text: format!("startswith({column},{})", prefix.to_odata()),
            kind: Kind::Leaf,
        }
    }

    /// Both expressions must hold.
    pub fn and(self, other: Filter) -> Self {
        Self {
            text: format!("{} and {}", self.operand(Kind::Or), other.operand(Kind::Or)),
            kind: Kind::And,
        }
    }

    /// Either expression must hold.
    pub fn or(self, other: Filter) -> Self {
        Self {
            text: format!("{} or {}", self.text, other.text),
            kind: Kind::Or,
        }
    }

    /// The expression text.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    fn operand(&self, wrap: Kind) -> String {
        if self.kind == wrap {
            format!("({})", self.text)
        } else {
            self.text.clone()
        }
    }
}

impl fmt::Display for Filter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

impl From<Filter> for String {
    fn from(f: Filter) -> Self {
        f.text
    }
}

#[doc(hidden)]
pub fn and_acc(acc: Option<Filter>, term: Filter) -> Option<Filter> {
    Some(match acc {
        Some(a) => a.and(term),
        None => term,
    })
}

#[doc(hidden)]
pub fn or_acc(or: Option<Filter>, and: Option<Filter>, term: Filter) -> Option<Filter> {
    let and = and_acc(and, term).expect("always Some");
    Some(match or {
        Some(o) => o.or(and),
        None => and,
    })
}

#[doc(hidden)]
pub fn finish(or: Option<Filter>, and: Option<Filter>, term: Filter) -> Filter {
    or_acc(or, and, term).expect("always Some")
}

/// Builds a [`Filter`] from Rust-like syntax.
///
/// * Columns are bare identifiers (`Status`) or string literals (`"Author/Title"`).
/// * Comparisons: `==`, `!=`, `>`, `>=`, `<`, `<=`.
/// * Values are a single literal (`"text"`, `3`, `true`) or a parenthesized Rust expression
///   (`(name)`, `(-1)`); strings are always quoted and escaped.
/// * `&&` binds tighter than `||`, and parentheses group, as in Rust.
///
/// ```
/// let f = oxipoint::filter!(Done == false || Priority > 2 && "Author/Title" == "Ann");
/// assert_eq!(f.as_str(), "Done eq false or Priority gt 2 and Author/Title eq 'Ann'");
/// ```
#[macro_export]
macro_rules! filter {
    (@leaf $c:expr, ==, $v:expr) => { $crate::filter::Filter::eq($c, $v) };
    (@leaf $c:expr, !=, $v:expr) => { $crate::filter::Filter::ne($c, $v) };
    (@leaf $c:expr, >=, $v:expr) => { $crate::filter::Filter::ge($c, $v) };
    (@leaf $c:expr, <=, $v:expr) => { $crate::filter::Filter::le($c, $v) };
    (@leaf $c:expr, >, $v:expr) => { $crate::filter::Filter::gt($c, $v) };
    (@leaf $c:expr, <, $v:expr) => { $crate::filter::Filter::lt($c, $v) };

    (@s $o:expr, $a:expr; ($($g:tt)+) $($rest:tt)*) => {
        $crate::filter!(@op $o, $a, $crate::filter!($($g)+); $($rest)*)
    };
    (@s $o:expr, $a:expr; $c:ident $op:tt $v:tt $($rest:tt)*) => {
        $crate::filter!(@op $o, $a, $crate::filter!(@leaf stringify!($c), $op, $v); $($rest)*)
    };
    (@s $o:expr, $a:expr; $c:literal $op:tt $v:tt $($rest:tt)*) => {
        $crate::filter!(@op $o, $a, $crate::filter!(@leaf $c, $op, $v); $($rest)*)
    };

    (@op $o:expr, $a:expr, $t:expr;) => { $crate::filter::finish($o, $a, $t) };
    (@op $o:expr, $a:expr, $t:expr; && $($rest:tt)+) => {
        $crate::filter!(@s $o, $crate::filter::and_acc($a, $t); $($rest)+)
    };
    (@op $o:expr, $a:expr, $t:expr; || $($rest:tt)+) => {
        $crate::filter!(@s $crate::filter::or_acc($o, $a, $t), None; $($rest)+)
    };

    ($($t:tt)+) => { $crate::filter!(@s None, None; $($t)+) };
}

#[cfg(test)]
mod tests {
    use super::Filter;

    #[test]
    fn escapes_quotes() {
        assert_eq!(Filter::eq("A", "it's").as_str(), "A eq 'it''s'");
        assert_eq!(
            Filter::eq("A", "' or 1 eq 1 or A eq '").as_str(),
            "A eq ''' or 1 eq 1 or A eq '''"
        );
    }

    #[test]
    fn precedence_matches_rust() {
        let f = crate::filter!(A == 1 && B == 2 || C == 3 && D == 4);
        assert_eq!(f.as_str(), "A eq 1 and B eq 2 or C eq 3 and D eq 4");
        let f = crate::filter!(A == 1 && (B == 2 || C == 3));
        assert_eq!(f.as_str(), "A eq 1 and (B eq 2 or C eq 3)");
        let f = crate::filter!((A == 1 || B == 2) && C == 3);
        assert_eq!(f.as_str(), "(A eq 1 or B eq 2) and C eq 3");
    }

    #[test]
    fn values_and_operators() {
        let n = 5;
        let s = String::from("x'y");
        let f = crate::filter!(A != (n) && B <= (-1) && C < 2.5 && D == (&s) && "E/F" >= "z");
        assert_eq!(
            f.as_str(),
            "A ne 5 and B le -1 and C lt 2.5 and D eq 'x''y' and E/F ge 'z'"
        );
        assert_eq!(crate::filter!(A == true).as_str(), "A eq true");
    }

    #[test]
    fn starts_with_and_conversion() {
        let s: String = Filter::starts_with("Title", "a'b").into();
        assert_eq!(s, "startswith(Title,'a''b')");
    }
}

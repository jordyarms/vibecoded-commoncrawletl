use std::fmt;

/// A single term in an N-Quads statement.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Term {
    Iri(String),
    BlankNode(String),
    Literal {
        value: String,
        datatype: Option<String>,
        language: Option<String>,
    },
}

impl Term {
    pub fn as_iri(&self) -> Option<&str> {
        match self {
            Term::Iri(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_str_value(&self) -> &str {
        match self {
            Term::Iri(s) | Term::BlankNode(s) => s,
            Term::Literal { value, .. } => value,
        }
    }

    pub fn is_blank_node(&self) -> bool {
        matches!(self, Term::BlankNode(_))
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Term::Iri(iri) => write!(f, "<{iri}>"),
            Term::BlankNode(id) => write!(f, "_:{id}"),
            Term::Literal {
                value,
                datatype,
                language,
            } => {
                write!(f, "\"{value}\"")?;
                if let Some(lang) = language {
                    write!(f, "@{lang}")?;
                } else if let Some(dt) = datatype {
                    write!(f, "^^<{dt}>")?;
                }
                Ok(())
            }
        }
    }
}

/// A single N-Quads statement: subject predicate object graph .
#[derive(Debug, Clone)]
pub struct Quad {
    pub subject: Term,
    pub predicate: Term,
    pub object: Term,
    pub graph: Option<Term>,
}

impl Quad {
    /// Extract the domain from the graph IRI (the source URL in WDC data).
    pub fn graph_domain(&self) -> Option<String> {
        let iri = self.graph.as_ref()?.as_iri()?;
        url::Url::parse(iri).ok().and_then(|u| u.host_str().map(|h| h.to_lowercase()))
    }

    /// Get the local name of the predicate IRI (after last # or /).
    pub fn predicate_local(&self) -> Option<&str> {
        let iri = self.predicate.as_iri()?;
        iri.rfind('#')
            .or_else(|| iri.rfind('/'))
            .map(|i| &iri[i + 1..])
    }
}

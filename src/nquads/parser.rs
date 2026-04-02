use std::collections::HashMap;

use crate::nquads::types::{Quad, Term};

/// Parse a single N-Quads line into a Quad.
///
/// N-Quads format: `subject predicate object graphLabel? .`
pub fn parse_line(line: &str) -> Result<Quad, String> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return Err("empty or comment line".into());
    }

    let mut parser = NQuadsParser::new(line);
    let subject = parser.parse_term()?;
    parser.skip_whitespace();
    let predicate = parser.parse_term()?;
    parser.skip_whitespace();
    let object = parser.parse_term()?;
    parser.skip_whitespace();

    let graph = if parser.peek() == Some('.') {
        None
    } else if parser.remaining().is_empty() {
        None
    } else {
        let g = parser.parse_term()?;
        Some(g)
    };

    Ok(Quad {
        subject,
        predicate,
        object,
        graph,
    })
}

struct NQuadsParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> NQuadsParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn remaining(&self) -> &str {
        &self.input[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() {
            let ch = self.input.as_bytes()[self.pos];
            if ch == b' ' || ch == b'\t' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn parse_term(&mut self) -> Result<Term, String> {
        match self.peek() {
            Some('<') => self.parse_iri(),
            Some('_') => self.parse_blank_node(),
            Some('"') => self.parse_literal(),
            Some(ch) => Err(format!("unexpected character '{}' at position {}", ch, self.pos)),
            None => Err("unexpected end of input".into()),
        }
    }

    fn parse_iri(&mut self) -> Result<Term, String> {
        self.pos += 1; // skip '<'
        let start = self.pos;
        while self.pos < self.input.len() {
            if self.input.as_bytes()[self.pos] == b'>' {
                let iri = &self.input[start..self.pos];
                self.pos += 1; // skip '>'
                return Ok(Term::Iri(unescape(iri)));
            }
            self.pos += 1;
        }
        Err("unterminated IRI".into())
    }

    fn parse_blank_node(&mut self) -> Result<Term, String> {
        if !self.remaining().starts_with("_:") {
            return Err("expected blank node '_:'".into());
        }
        self.pos += 2; // skip '_:'
        let start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.input.as_bytes()[self.pos];
            if ch == b' ' || ch == b'\t' || ch == b'.' {
                break;
            }
            self.pos += 1;
        }
        let id = &self.input[start..self.pos];
        if id.is_empty() {
            return Err("empty blank node identifier".into());
        }
        Ok(Term::BlankNode(id.to_string()))
    }

    fn parse_literal(&mut self) -> Result<Term, String> {
        self.pos += 1; // skip opening '"'
        let mut value = String::new();
        let mut escaped = false;

        while self.pos < self.input.len() {
            let ch = self.input.as_bytes()[self.pos];
            if escaped {
                match ch {
                    b'"' => value.push('"'),
                    b'\\' => value.push('\\'),
                    b'n' => value.push('\n'),
                    b'r' => value.push('\r'),
                    b't' => value.push('\t'),
                    b'u' => {
                        self.pos += 1;
                        let hex = self
                            .input
                            .get(self.pos..self.pos + 4)
                            .ok_or("incomplete \\u escape")?;
                        let cp = u32::from_str_radix(hex, 16)
                            .map_err(|e| format!("invalid \\u escape: {e}"))?;
                        let ch = char::from_u32(cp).ok_or("invalid unicode codepoint")?;
                        value.push(ch);
                        self.pos += 3; // will be incremented by 1 below
                    }
                    b'U' => {
                        self.pos += 1;
                        let hex = self
                            .input
                            .get(self.pos..self.pos + 8)
                            .ok_or("incomplete \\U escape")?;
                        let cp = u32::from_str_radix(hex, 16)
                            .map_err(|e| format!("invalid \\U escape: {e}"))?;
                        let ch = char::from_u32(cp).ok_or("invalid unicode codepoint")?;
                        value.push(ch);
                        self.pos += 7; // will be incremented by 1 below
                    }
                    _ => {
                        value.push('\\');
                        value.push(ch as char);
                    }
                }
                escaped = false;
            } else if ch == b'\\' {
                escaped = true;
            } else if ch == b'"' {
                self.pos += 1; // skip closing '"'
                // Check for language tag or datatype
                let (datatype, language) = self.parse_literal_suffix()?;
                return Ok(Term::Literal {
                    value,
                    datatype,
                    language,
                });
            } else {
                value.push(ch as char);
            }
            self.pos += 1;
        }
        Err("unterminated string literal".into())
    }

    fn parse_literal_suffix(&mut self) -> Result<(Option<String>, Option<String>), String> {
        if self.remaining().starts_with("^^") {
            self.pos += 2; // skip '^^'
            if self.peek() == Some('<') {
                let term = self.parse_iri()?;
                if let Term::Iri(dt) = term {
                    return Ok((Some(dt), None));
                }
            }
            Err("expected IRI after ^^".into())
        } else if self.remaining().starts_with('@') {
            self.pos += 1; // skip '@'
            let start = self.pos;
            while self.pos < self.input.len() {
                let ch = self.input.as_bytes()[self.pos];
                if ch == b' ' || ch == b'\t' || ch == b'.' {
                    break;
                }
                self.pos += 1;
            }
            let lang = &self.input[start..self.pos];
            Ok((None, Some(lang.to_string())))
        } else {
            Ok((None, None))
        }
    }
}

fn unescape(s: &str) -> String {
    if !s.contains('\\') {
        return s.to_string();
    }
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).collect();
                    if let Ok(cp) = u32::from_str_radix(&hex, 16) {
                        if let Some(c) = char::from_u32(cp) {
                            result.push(c);
                            continue;
                        }
                    }
                    result.push_str("\\u");
                    result.push_str(&hex);
                }
                Some('U') => {
                    let hex: String = chars.by_ref().take(8).collect();
                    if let Ok(cp) = u32::from_str_radix(&hex, 16) {
                        if let Some(c) = char::from_u32(cp) {
                            result.push(c);
                            continue;
                        }
                    }
                    result.push_str("\\U");
                    result.push_str(&hex);
                }
                Some(c) => {
                    result.push('\\');
                    result.push(c);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(ch);
        }
    }
    result
}

/// Groups quads by subject, using run-length detection with a capped spillover buffer.
///
/// WDC data is naturally clustered by subject, so most groups are captured in sequence.
/// The spillover HashMap handles out-of-order subjects up to a cap.
pub struct SubjectGrouper {
    current_subject: Option<Term>,
    current_group: Vec<Quad>,
    spillover: HashMap<String, Vec<Quad>>,
    spillover_cap: usize,
}

impl SubjectGrouper {
    pub fn new(spillover_cap: usize) -> Self {
        Self {
            current_subject: None,
            current_group: Vec::new(),
            spillover: HashMap::new(),
            spillover_cap,
        }
    }

    /// Push a quad into the grouper. Returns completed groups if any.
    pub fn push(&mut self, quad: Quad) -> Vec<Vec<Quad>> {
        let mut completed = Vec::new();
        let subject_key = quad.subject.as_str_value().to_string();

        match &self.current_subject {
            Some(current) if current.as_str_value() == subject_key => {
                self.current_group.push(quad);
            }
            _ => {
                // Subject changed — flush current group
                if let Some(_prev_subject) = self.current_subject.take() {
                    if !self.current_group.is_empty() {
                        completed.push(std::mem::take(&mut self.current_group));
                    }
                }

                // Check spillover
                if let Some(mut group) = self.spillover.remove(&subject_key) {
                    group.push(quad);
                    self.current_subject = Some(group[0].subject.clone());
                    self.current_group = group;
                } else {
                    self.current_subject = Some(quad.subject.clone());
                    self.current_group = vec![quad];
                }
            }
        }

        // Evict spillover if over cap
        if self.spillover.len() >= self.spillover_cap {
            let keys: Vec<String> = self.spillover.keys().cloned().collect();
            for key in keys {
                if let Some(group) = self.spillover.remove(&key) {
                    completed.push(group);
                }
                if self.spillover.len() < self.spillover_cap / 2 {
                    break;
                }
            }
        }

        completed
    }

    /// Flush all remaining groups.
    pub fn flush(mut self) -> Vec<Vec<Quad>> {
        let mut completed = Vec::new();
        if !self.current_group.is_empty() {
            completed.push(std::mem::take(&mut self.current_group));
        }
        for (_key, group) in self.spillover.drain() {
            completed.push(group);
        }
        completed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_quad() {
        let line = r#"<http://example.org/s> <http://example.org/p> "hello" <http://example.org/g> ."#;
        let quad = parse_line(line).unwrap();
        assert_eq!(quad.subject, Term::Iri("http://example.org/s".into()));
        assert_eq!(quad.predicate, Term::Iri("http://example.org/p".into()));
        assert_eq!(
            quad.object,
            Term::Literal {
                value: "hello".into(),
                datatype: None,
                language: None
            }
        );
        assert_eq!(
            quad.graph,
            Some(Term::Iri("http://example.org/g".into()))
        );
    }

    #[test]
    fn test_parse_typed_literal() {
        let line = r#"<http://ex.org/s> <http://ex.org/p> "2024-01-01"^^<http://www.w3.org/2001/XMLSchema#date> <http://ex.org/g> ."#;
        let quad = parse_line(line).unwrap();
        if let Term::Literal { datatype, .. } = &quad.object {
            assert_eq!(
                datatype.as_deref(),
                Some("http://www.w3.org/2001/XMLSchema#date")
            );
        } else {
            panic!("expected literal");
        }
    }

    #[test]
    fn test_parse_language_literal() {
        let line = r#"<http://ex.org/s> <http://ex.org/p> "bonjour"@fr <http://ex.org/g> ."#;
        let quad = parse_line(line).unwrap();
        if let Term::Literal { language, .. } = &quad.object {
            assert_eq!(language.as_deref(), Some("fr"));
        } else {
            panic!("expected literal");
        }
    }

    #[test]
    fn test_parse_blank_node() {
        let line = r#"_:b0 <http://ex.org/p> _:b1 <http://ex.org/g> ."#;
        let quad = parse_line(line).unwrap();
        assert_eq!(quad.subject, Term::BlankNode("b0".into()));
        assert_eq!(quad.object, Term::BlankNode("b1".into()));
    }

    #[test]
    fn test_parse_escaped_quotes() {
        let line = r#"<http://ex.org/s> <http://ex.org/p> "say \"hello\"" <http://ex.org/g> ."#;
        let quad = parse_line(line).unwrap();
        if let Term::Literal { value, .. } = &quad.object {
            assert_eq!(value, r#"say "hello""#);
        } else {
            panic!("expected literal");
        }
    }

    #[test]
    fn test_graph_domain() {
        let line = r#"<http://ex.org/s> <http://ex.org/p> "x" <https://www.toronto.ca/events/123> ."#;
        let quad = parse_line(line).unwrap();
        assert_eq!(quad.graph_domain().as_deref(), Some("www.toronto.ca"));
    }

    #[test]
    fn test_predicate_local() {
        let line = r#"<http://ex.org/s> <http://schema.org/name> "x" <http://ex.org/g> ."#;
        let quad = parse_line(line).unwrap();
        assert_eq!(quad.predicate_local(), Some("name"));
    }

    #[test]
    fn test_subject_grouper() {
        let mut grouper = SubjectGrouper::new(100);
        let make_quad = |s: &str| Quad {
            subject: Term::Iri(s.into()),
            predicate: Term::Iri("http://ex.org/p".into()),
            object: Term::Literal {
                value: "v".into(),
                datatype: None,
                language: None,
            },
            graph: None,
        };

        // Same subject — no output yet
        let out = grouper.push(make_quad("http://ex.org/a"));
        assert!(out.is_empty());
        let out = grouper.push(make_quad("http://ex.org/a"));
        assert!(out.is_empty());

        // New subject — flushes previous group
        let out = grouper.push(make_quad("http://ex.org/b"));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].len(), 2);

        // Flush remaining
        let out = grouper.flush();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].len(), 1);
    }
}

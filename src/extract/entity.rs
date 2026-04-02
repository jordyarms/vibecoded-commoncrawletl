use std::collections::HashMap;

use crate::extract::types::{AddressInfo, ExtractedEvent, LocationInfo};
use crate::nquads::types::{Quad, Term};

/// Reconstruct an ExtractedEvent from a group of quads sharing the same subject.
/// Uses `blank_nodes` to resolve references to location/address/organizer blank nodes.
pub fn extract_event(
    quads: &[Quad],
    blank_nodes: &HashMap<String, Vec<Quad>>,
    part_number: u32,
) -> Option<ExtractedEvent> {
    if quads.is_empty() {
        return None;
    }

    // Check if this is an Event type
    let is_event = quads.iter().any(|q| {
        q.predicate_local() == Some("type")
            && q.object
                .as_iri()
                .is_some_and(|iri| iri.contains("schema.org") && iri.contains("Event"))
    });

    if !is_event {
        return None;
    }

    let domain = quads
        .first()
        .and_then(|q| q.graph_domain())
        .unwrap_or_default();
    let source_url = quads
        .first()
        .and_then(|q| q.graph.as_ref()?.as_iri().map(String::from))
        .unwrap_or_default();

    let mut event = ExtractedEvent {
        name: None,
        description: None,
        start_date: None,
        end_date: None,
        url: None,
        event_type: None,
        location: None,
        organizer: None,
        domain,
        source_url,
        part_number,
    };

    for quad in quads {
        let pred = match quad.predicate_local() {
            Some(p) => p,
            None => continue,
        };

        match pred {
            "name" => event.name = Some(quad.object.as_str_value().to_string()),
            "description" => event.description = Some(quad.object.as_str_value().to_string()),
            "startDate" => event.start_date = Some(quad.object.as_str_value().to_string()),
            "endDate" => event.end_date = Some(quad.object.as_str_value().to_string()),
            "url" => event.url = Some(quad.object.as_str_value().to_string()),
            "location" => {
                event.location = resolve_location(&quad.object, blank_nodes);
            }
            "organizer" => {
                event.organizer = resolve_name(&quad.object, blank_nodes);
            }
            "type" => {
                if let Some(iri) = quad.object.as_iri() {
                    if let Some(local) = iri.rfind('/').map(|i| &iri[i + 1..]) {
                        event.event_type = Some(local.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    // Only return if we have at least a name or URL
    if event.name.is_some() || event.url.is_some() {
        Some(event)
    } else {
        None
    }
}

fn resolve_location(term: &Term, blank_nodes: &HashMap<String, Vec<Quad>>) -> Option<LocationInfo> {
    let quads = resolve_blank(term, blank_nodes)?;

    let mut loc = LocationInfo {
        name: None,
        address: None,
        latitude: None,
        longitude: None,
    };

    for q in quads {
        let pred = match q.predicate_local() {
            Some(p) => p,
            None => continue,
        };
        match pred {
            "name" => loc.name = Some(q.object.as_str_value().to_string()),
            "address" => loc.address = resolve_address(&q.object, blank_nodes),
            "latitude" | "geo" => {
                if let Ok(v) = q.object.as_str_value().parse::<f64>() {
                    loc.latitude = Some(v);
                }
            }
            "longitude" => {
                if let Ok(v) = q.object.as_str_value().parse::<f64>() {
                    loc.longitude = Some(v);
                }
            }
            _ => {}
        }
    }

    // If it looks like a Place with an address directly, also check for address fields
    if loc.address.is_none() {
        let addr = extract_address_from_quads(quads);
        if addr.locality.is_some() || addr.postal_code.is_some() {
            loc.address = Some(addr);
        }
    }

    Some(loc)
}

fn resolve_address(
    term: &Term,
    blank_nodes: &HashMap<String, Vec<Quad>>,
) -> Option<AddressInfo> {
    let quads = resolve_blank(term, blank_nodes)?;
    let addr = extract_address_from_quads(quads);
    Some(addr)
}

fn extract_address_from_quads(quads: &[Quad]) -> AddressInfo {
    let mut addr = AddressInfo {
        street: None,
        locality: None,
        region: None,
        postal_code: None,
        country: None,
    };

    for q in quads {
        let pred = match q.predicate_local() {
            Some(p) => p,
            None => continue,
        };
        match pred {
            "streetAddress" => addr.street = Some(q.object.as_str_value().to_string()),
            "addressLocality" => addr.locality = Some(q.object.as_str_value().to_string()),
            "addressRegion" => addr.region = Some(q.object.as_str_value().to_string()),
            "postalCode" => addr.postal_code = Some(q.object.as_str_value().to_string()),
            "addressCountry" => addr.country = Some(q.object.as_str_value().to_string()),
            _ => {}
        }
    }

    addr
}

fn resolve_name(term: &Term, blank_nodes: &HashMap<String, Vec<Quad>>) -> Option<String> {
    // If it's a literal, use directly
    if let Term::Literal { value, .. } = term {
        return Some(value.clone());
    }

    let quads = resolve_blank(term, blank_nodes)?;
    for q in quads {
        if q.predicate_local() == Some("name") {
            return Some(q.object.as_str_value().to_string());
        }
    }
    None
}

fn resolve_blank<'a>(
    term: &Term,
    blank_nodes: &'a HashMap<String, Vec<Quad>>,
) -> Option<&'a Vec<Quad>> {
    if let Term::BlankNode(id) = term {
        blank_nodes.get(id)
    } else {
        None
    }
}

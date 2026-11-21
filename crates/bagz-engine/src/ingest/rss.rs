//! RSS 2.0 job feeds. Aggregators often title items `Company: Role`; the
//! parser splits that form and falls back to the feed host as company.

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::error::{Error, Result};
use crate::model::{Posting, Source, Workplace};
use crate::text::{html_to_text, iso_date, rfc822_date, truncate_chars, DESCRIPTION_CAP};

/// Host part of a feed URL, used as the board name.
pub fn host(feed_url: &str) -> String {
    let rest = feed_url
        .split_once("://")
        .map(|(_, r)| r)
        .unwrap_or(feed_url);
    rest.split(['/', '?', '#'])
        .next()
        .unwrap_or(rest)
        .trim_start_matches("www.")
        .to_string()
}

#[derive(Default)]
struct Item {
    title: String,
    link: String,
    guid: String,
    pub_date: String,
    description: String,
    region: String,
    category: String,
    kind: String,
    company: String,
}

pub fn parse(body: &str, feed_url: &str) -> Result<Vec<Posting>> {
    let board = host(feed_url);
    let mut reader = Reader::from_str(body);
    reader.config_mut().trim_text(true);
    let mut out = Vec::new();
    let mut item: Option<Item> = None;
    let mut field: Option<String> = None;
    let mut saw_channel = false;
    loop {
        let ev = reader
            .read_event()
            .map_err(|e| Error::parse("rss", e.to_string()))?;
        match ev {
            Event::Start(e) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                match name.as_str() {
                    "channel" | "rss" | "feed" => saw_channel = true,
                    "item" => item = Some(Item::default()),
                    _ if item.is_some() => field = Some(name),
                    _ => {}
                }
            }
            Event::End(e) => {
                let name = e.local_name();
                if name.as_ref() == b"item" {
                    if let Some(it) = item.take() {
                        if let Some(p) = to_posting(it, &board) {
                            out.push(p);
                        }
                    }
                }
                field = None;
            }
            Event::Text(t) => {
                if let (Some(it), Some(f)) = (item.as_mut(), field.as_deref()) {
                    let s = t
                        .unescape()
                        .map_err(|e| Error::parse("rss", e.to_string()))?;
                    push_field(it, f, &s);
                }
            }
            Event::CData(c) => {
                if let (Some(it), Some(f)) = (item.as_mut(), field.as_deref()) {
                    let s = String::from_utf8_lossy(&c.into_inner()).to_string();
                    push_field(it, f, &s);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if !saw_channel {
        return Err(Error::parse("rss", "no <rss> or <channel> element"));
    }
    Ok(out)
}

fn push_field(it: &mut Item, field: &str, s: &str) {
    let slot = match field {
        "title" => &mut it.title,
        "link" => &mut it.link,
        "guid" => &mut it.guid,
        "pubDate" | "date" | "published" => &mut it.pub_date,
        "description" | "encoded" | "summary" => &mut it.description,
        "region" | "location" => &mut it.region,
        "category" => &mut it.category,
        "type" => &mut it.kind,
        "company" | "creator" | "author" => &mut it.company,
        _ => return,
    };
    if !slot.is_empty() {
        slot.push(' ');
    }
    slot.push_str(s);
}

fn to_posting(it: Item, board: &str) -> Option<Posting> {
    let raw_title = crate::text::collapse_spaces(&it.title);
    if raw_title.is_empty() {
        return None;
    }
    let (company, title) = match (it.company.trim(), raw_title.split_once(": ")) {
        (c, _) if !c.is_empty() => (c.to_string(), raw_title.clone()),
        (_, Some((c, t))) => (c.trim().to_string(), t.trim().to_string()),
        _ => (board.to_string(), raw_title.clone()),
    };
    let link = it.link.trim().to_string();
    let native = if it.guid.trim().is_empty() {
        link.clone()
    } else {
        it.guid.trim().to_string()
    };
    if native.is_empty() {
        return None;
    }
    let location = crate::text::collapse_spaces(&it.region);
    let mut workplace = Workplace::from_label(&location);
    if workplace == Workplace::Unknown {
        workplace = Workplace::from_label(&raw_title);
    }
    let description = truncate_chars(&html_to_text(&it.description), DESCRIPTION_CAP);
    let posted = rfc822_date(&it.pub_date).or_else(|| iso_date(&it.pub_date));
    Some(Posting {
        id: Posting::make_id(Source::Rss, board, &native),
        source: Source::Rss,
        board: board.to_string(),
        company,
        title,
        location,
        workplace,
        department: (!it.category.trim().is_empty()).then(|| it.category.trim().to_string()),
        employment_type: (!it.kind.trim().is_empty()).then(|| it.kind.trim().to_string()),
        url: link,
        apply_url: None,
        posted,
        salary: super::salary_from_text(&description),
        description,
    })
}

//! ATS-safe `.docx` export, with no dependencies.
//!
//! ATS parsers drop or scramble text in tables, text boxes, columns,
//! headers, footers and images. This writer emits none of those: one
//! column of paragraphs, standard headings (`Summary`, `Skills`,
//! `Experience`, `Education`), a core font, and bullets as plain `•` text.
//! The zip uses the stored method and a fixed timestamp, so the same input
//! always gives the same bytes.

use std::path::Path;

use crate::agents::cover_letter::CoverLetter;
use crate::agents::tailor::Resume;
use crate::approval::Approved;
use crate::error::{Error, Result};
use crate::profile::Profile;

/// Paragraph kinds the writer supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Title,
    Heading,
    Normal,
    Bullet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Para {
    pub style: Style,
    pub text: String,
}

impl Para {
    pub fn new(style: Style, text: impl Into<String>) -> Self {
        Self {
            style,
            text: text.into(),
        }
    }
}

/// Escapes text for XML and drops control characters XML 1.0 forbids.
pub fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\t' | '\n' | '\r' => out.push(' '),
            c if (c as u32) < 0x20 => {}
            c => out.push(c),
        }
    }
    out
}

fn para_xml(p: &Para) -> String {
    let (size, bold, before, after) = match p.style {
        Style::Title => (32, true, 0, 60),
        Style::Heading => (24, true, 200, 60),
        Style::Normal => (22, false, 0, 60),
        Style::Bullet => (22, false, 0, 40),
    };
    let text = match p.style {
        Style::Bullet => format!("• {}", p.text),
        _ => p.text.clone(),
    };
    let b = if bold { "<w:b/>" } else { "" };
    let ind = if p.style == Style::Bullet {
        r#"<w:ind w:left="360" w:hanging="180"/>"#
    } else {
        ""
    };
    format!(
        r#"<w:p><w:pPr><w:spacing w:before="{before}" w:after="{after}"/>{ind}</w:pPr><w:r><w:rPr><w:rFonts w:ascii="Calibri" w:hAnsi="Calibri" w:cs="Calibri"/>{b}<w:sz w:val="{size}"/></w:rPr><w:t xml:space="preserve">{}</w:t></w:r></w:p>"#,
        xml_escape(&text)
    )
}

/// The `word/document.xml` part.
pub fn document_xml(paras: &[Para]) -> String {
    let body: String = paras.iter().map(para_xml).collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{body}<w:sectPr><w:pgSz w:w="12240" w:h="15840"/><w:pgMar w:top="1080" w:right="1080" w:bottom="1080" w:left="1080" w:header="0" w:footer="0" w:gutter="0"/><w:cols w:space="720"/></w:sectPr></w:body></w:document>"#
    )
}

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/></Types>"#;

const RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/></Relationships>"#;

fn core_xml(title: &str, author: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>{}</dc:title><dc:creator>{}</dc:creator></cp:coreProperties>"#,
        xml_escape(title),
        xml_escape(author)
    )
}

/// Builds a `.docx` file from paragraphs.
pub fn build(paras: &[Para], title: &str, author: &str) -> Vec<u8> {
    let mut z = ZipWriter::default();
    z.add("[Content_Types].xml", CONTENT_TYPES.as_bytes());
    z.add("_rels/.rels", RELS.as_bytes());
    z.add("docProps/core.xml", core_xml(title, author).as_bytes());
    z.add("word/document.xml", document_xml(paras).as_bytes());
    z.finish()
}

/// Résumé paragraphs in ATS reading order.
pub fn resume_paras(r: &Resume) -> Vec<Para> {
    let mut p = vec![Para::new(Style::Title, &r.name)];
    if !r.contact.is_empty() {
        p.push(Para::new(Style::Normal, r.contact.join(" | ")));
    }
    if !r.headline.is_empty() {
        p.push(Para::new(Style::Normal, &r.headline));
    }
    if !r.summary.is_empty() {
        p.push(Para::new(Style::Heading, "Summary"));
        p.push(Para::new(Style::Normal, &r.summary));
    }
    if !r.skills.is_empty() {
        p.push(Para::new(Style::Heading, "Skills"));
        p.push(Para::new(Style::Normal, r.skills.join(", ")));
    }
    if !r.experience.is_empty() {
        p.push(Para::new(Style::Heading, "Experience"));
        for e in &r.experience {
            p.push(Para::new(
                Style::Normal,
                format!("{}, {} ({} to {})", e.title, e.company, e.start, e.end),
            ));
            for b in &e.bullets {
                p.push(Para::new(Style::Bullet, b));
            }
        }
    }
    if !r.education.is_empty() {
        p.push(Para::new(Style::Heading, "Education"));
        for e in &r.education {
            p.push(Para::new(Style::Normal, e));
        }
    }
    p
}

pub fn resume_bytes(r: &Resume) -> Vec<u8> {
    build(&resume_paras(r), &format!("{} résumé", r.name), &r.name)
}

pub fn letter_paras(l: &CoverLetter, profile: &Profile) -> Vec<Para> {
    let mut p = vec![
        Para::new(Style::Title, profile.full_name()),
        Para::new(
            Style::Normal,
            format!("{} | {}", profile.email, profile.phone),
        ),
    ];
    for chunk in l.body.split("\n\n") {
        let t = crate::text::collapse_spaces(chunk);
        if !t.is_empty() {
            p.push(Para::new(Style::Normal, t));
        }
    }
    p
}

/// Writes an approved résumé. An unapproved one cannot reach this call.
pub fn export_resume(r: &Approved<Resume>, path: impl AsRef<Path>) -> Result<()> {
    std::fs::write(path, resume_bytes(r.payload())).map_err(Error::from)
}

/// Writes an approved cover letter.
pub fn export_letter(
    l: &Approved<CoverLetter>,
    profile: &Profile,
    path: impl AsRef<Path>,
) -> Result<()> {
    let bytes = build(
        &letter_paras(l.payload(), profile),
        &format!("Cover letter, {}", l.payload().role),
        &profile.full_name(),
    );
    std::fs::write(path, bytes).map_err(Error::from)
}

/// CRC-32 (IEEE), as zip needs it.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Minimal zip writer: stored entries, fixed 1980-01-01 timestamp.
#[derive(Default)]
pub struct ZipWriter {
    buf: Vec<u8>,
    central: Vec<u8>,
    count: u16,
}

impl ZipWriter {
    pub fn add(&mut self, name: &str, data: &[u8]) {
        let offset = self.buf.len() as u32;
        let crc = crc32(data);
        let size = data.len() as u32;
        let n = name.as_bytes();
        // local header
        self.buf.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        self.buf.extend_from_slice(&20u16.to_le_bytes()); // version needed
        self.buf.extend_from_slice(&0x0800u16.to_le_bytes()); // utf-8 names
        self.buf.extend_from_slice(&0u16.to_le_bytes()); // stored
        self.buf.extend_from_slice(&0u16.to_le_bytes()); // time
        self.buf.extend_from_slice(&0x0021u16.to_le_bytes()); // 1980-01-01
        self.buf.extend_from_slice(&crc.to_le_bytes());
        self.buf.extend_from_slice(&size.to_le_bytes());
        self.buf.extend_from_slice(&size.to_le_bytes());
        self.buf.extend_from_slice(&(n.len() as u16).to_le_bytes());
        self.buf.extend_from_slice(&0u16.to_le_bytes());
        self.buf.extend_from_slice(n);
        self.buf.extend_from_slice(data);
        // central directory entry
        let c = &mut self.central;
        c.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        c.extend_from_slice(&20u16.to_le_bytes()); // made by
        c.extend_from_slice(&20u16.to_le_bytes()); // needed
        c.extend_from_slice(&0x0800u16.to_le_bytes());
        c.extend_from_slice(&0u16.to_le_bytes());
        c.extend_from_slice(&0u16.to_le_bytes());
        c.extend_from_slice(&0x0021u16.to_le_bytes());
        c.extend_from_slice(&crc.to_le_bytes());
        c.extend_from_slice(&size.to_le_bytes());
        c.extend_from_slice(&size.to_le_bytes());
        c.extend_from_slice(&(n.len() as u16).to_le_bytes());
        c.extend_from_slice(&[0; 8]); // extra, comment, disk
        c.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
        c.extend_from_slice(&0u32.to_le_bytes()); // external attrs
        c.extend_from_slice(&offset.to_le_bytes());
        c.extend_from_slice(n);
        self.count += 1;
    }

    pub fn finish(mut self) -> Vec<u8> {
        let cd_offset = self.buf.len() as u32;
        let cd_size = self.central.len() as u32;
        self.buf.extend_from_slice(&self.central);
        self.buf.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        self.buf.extend_from_slice(&[0; 4]);
        self.buf.extend_from_slice(&self.count.to_le_bytes());
        self.buf.extend_from_slice(&self.count.to_le_bytes());
        self.buf.extend_from_slice(&cd_size.to_le_bytes());
        self.buf.extend_from_slice(&cd_offset.to_le_bytes());
        self.buf.extend_from_slice(&0u16.to_le_bytes());
        self.buf
    }
}

fn u16_at(b: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?))
}
fn u32_at(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?))
}

/// Reads the stored entries of a zip and checks each CRC. Used by tests
/// and by `bagz docx-check` to confirm a file parses.
pub fn read_entries(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>> {
    let bad = |m: &str| Error::Guard(format!("bad zip: {m}"));
    let mut out = Vec::new();
    let mut i = 0;
    while u32_at(bytes, i) == Some(0x0403_4b50) {
        let method = u16_at(bytes, i + 8).ok_or_else(|| bad("header"))?;
        if method != 0 {
            return Err(bad("only stored entries are supported"));
        }
        let crc = u32_at(bytes, i + 14).ok_or_else(|| bad("crc"))?;
        let size = u32_at(bytes, i + 18).ok_or_else(|| bad("size"))? as usize;
        let nlen = u16_at(bytes, i + 26).ok_or_else(|| bad("name"))? as usize;
        let xlen = u16_at(bytes, i + 28).ok_or_else(|| bad("extra"))? as usize;
        let name_start = i + 30;
        let data_start = name_start + nlen + xlen;
        let name = bytes
            .get(name_start..name_start + nlen)
            .ok_or_else(|| bad("name bytes"))?;
        let data = bytes
            .get(data_start..data_start + size)
            .ok_or_else(|| bad("data bytes"))?;
        if crc32(data) != crc {
            return Err(bad("crc mismatch"));
        }
        out.push((String::from_utf8_lossy(name).to_string(), data.to_vec()));
        i = data_start + size;
    }
    if out.is_empty() {
        return Err(bad("no entries"));
    }
    Ok(out)
}

/// Text of every paragraph in `word/document.xml`, in order, the way an
/// ATS text extractor sees it.
pub fn extract_text(bytes: &[u8]) -> Result<Vec<String>> {
    let entries = read_entries(bytes)?;
    let doc = entries
        .iter()
        .find(|(n, _)| n == "word/document.xml")
        .ok_or_else(|| Error::Guard("no word/document.xml".into()))?;
    let xml = String::from_utf8_lossy(&doc.1);
    let mut paras = Vec::new();
    for p in xml.split("<w:p>").skip(1) {
        let mut text = String::new();
        for seg in p.split("<w:t").skip(1) {
            if let Some(start) = seg.find('>') {
                if let Some(end) = seg.find("</w:t>") {
                    text.push_str(&crate::text::decode_entities(&seg[start + 1..end]));
                }
            }
        }
        paras.push(text);
    }
    Ok(paras)
}

/// Structural checks for ATS safety: no tables, text boxes, images,
/// headers, footers, or multi-column sections.
pub fn ats_violations(bytes: &[u8]) -> Result<Vec<&'static str>> {
    let entries = read_entries(bytes)?;
    let mut v = Vec::new();
    for (name, data) in &entries {
        if name.starts_with("word/header") || name.starts_with("word/footer") {
            v.push("header or footer part");
        }
        if name.starts_with("word/media/") {
            v.push("embedded image");
        }
        if name == "word/document.xml" {
            let xml = String::from_utf8_lossy(data);
            for (tag, why) in [
                ("<w:tbl>", "table"),
                ("<w:txbxContent", "text box"),
                ("<w:drawing", "drawing"),
                ("<w:pict", "picture"),
                ("<w:cols w:num=\"", "multiple columns"),
            ] {
                if xml.contains(tag) {
                    v.push(why);
                }
            }
        }
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_known_vectors() {
        assert_eq!(crc32(b""), 0);
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(
            crc32(b"The quick brown fox jumps over the lazy dog"),
            0x414F_A339
        );
    }

    #[test]
    fn xml_escape_covers_specials_and_controls() {
        assert_eq!(
            xml_escape("a<b>&\"'\u{1}\tz"),
            "a&lt;b&gt;&amp;&quot;&apos; z"
        );
    }

    #[test]
    fn zip_round_trip() {
        let mut z = ZipWriter::default();
        z.add("a.txt", b"hello");
        z.add("dir/b.txt", b"");
        let bytes = z.finish();
        let e = read_entries(&bytes).unwrap();
        assert_eq!(e.len(), 2);
        assert_eq!(e[0], ("a.txt".to_string(), b"hello".to_vec()));
        assert_eq!(e[1].0, "dir/b.txt");
        assert_eq!(
            &bytes[bytes.len() - 22..bytes.len() - 18],
            &[0x50, 0x4b, 0x05, 0x06]
        );
    }

    #[test]
    fn corrupt_zip_is_refused() {
        let mut z = ZipWriter::default();
        z.add("a.txt", b"hello");
        let mut bytes = z.finish();
        bytes[30 + 5] ^= 0xff; // flip a data byte
        assert!(read_entries(&bytes).is_err());
        assert!(read_entries(b"not a zip").is_err());
    }
}

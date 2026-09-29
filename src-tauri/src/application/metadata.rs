use std::{fs, path::Path, time::Duration};

use serde::Deserialize;

use crate::{
    application::{library, reader},
    domain::paper::{MetadataUpdate, Paper},
};

const CROSSREF_API: &str = "https://api.crossref.org/works";

#[derive(Default, Debug, Clone)]
struct Candidate {
    title: Option<String>,
    authors: Vec<String>,
    year: Option<i32>,
    doi: Option<String>,
    source_url: Option<String>,
    venue: Option<String>,
    publisher: Option<String>,
}

#[derive(Deserialize)]
struct CrossrefEnvelope<T> {
    message: T,
}

#[derive(Deserialize)]
struct CrossrefList {
    #[serde(default)]
    items: Vec<CrossrefWork>,
}

#[derive(Deserialize, Default)]
struct CrossrefWork {
    #[serde(rename = "DOI")]
    doi: Option<String>,
    #[serde(default)]
    title: Vec<String>,
    #[serde(default)]
    author: Vec<CrossrefAuthor>,
    #[serde(rename = "container-title", default)]
    container_title: Vec<String>,
    publisher: Option<String>,
    resource: Option<CrossrefResource>,
    #[serde(rename = "URL")]
    url: Option<String>,
    #[serde(default)]
    link: Vec<CrossrefLink>,
    #[serde(rename = "published-print")]
    published_print: Option<CrossrefDate>,
    #[serde(rename = "published-online")]
    published_online: Option<CrossrefDate>,
    issued: Option<CrossrefDate>,
}

#[derive(Deserialize, Default)]
struct CrossrefAuthor {
    given: Option<String>,
    family: Option<String>,
    name: Option<String>,
}

#[derive(Deserialize)]
struct CrossrefResource {
    primary: Option<CrossrefPrimary>,
}
#[derive(Deserialize)]
struct CrossrefPrimary {
    #[serde(rename = "URL")]
    url: Option<String>,
}
#[derive(Deserialize)]
struct CrossrefLink {
    #[serde(rename = "URL")]
    url: Option<String>,
    #[serde(rename = "content-type")]
    content_type: Option<String>,
}
#[derive(Deserialize)]
struct CrossrefDate {
    #[serde(rename = "date-parts", default)]
    date_parts: Vec<Vec<i32>>,
}

pub fn enrich_paper(data_dir: &Path, paper_id: &str) -> Result<Paper, String> {
    let current = library::list_papers(data_dir)?
        .into_iter()
        .find(|paper| paper.id == paper_id)
        .ok_or("找不到这篇文献")?;
    let opened = reader::open_document(data_dir, paper_id)?;
    let bytes = fs::read(&opened.path).map_err(|e| format!("读取 PDF 元数据失败：{e}"))?;
    let embedded = embedded_candidate(&bytes);
    let lookup_title = embedded.title.as_deref().unwrap_or(&current.title);
    let lookup_doi = current.doi.as_deref().or(embedded.doi.as_deref());
    let remote = lookup_crossref(lookup_doi, lookup_title)?;
    let merged = merge(&current, embedded, remote);

    if same_metadata(&current, &merged) {
        return Ok(current);
    }
    library::update_metadata(
        data_dir,
        MetadataUpdate {
            id: current.id.clone(),
            expected_revision: current.revision,
            title: merged.title.unwrap_or(current.title),
            authors: merged.authors,
            year: merged.year,
            doi: merged.doi,
            source_url: merged.source_url,
            venue: merged.venue,
            publisher: merged.publisher,
        },
    )
}

fn lookup_crossref(doi: Option<&str>, title: &str) -> Result<Option<Candidate>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Reflo/0.1 (https://github.com/zzzhj514/Reflo)")
        .build()
        .map_err(|e| format!("创建元数据请求失败：{e}"))?;
    let work = if let Some(doi) = doi {
        let mut url = url::Url::parse(CROSSREF_API).map_err(|e| e.to_string())?;
        url.path_segments_mut()
            .map_err(|_| "Crossref 地址无效")?
            .push(doi);
        let response = client
            .get(url)
            .send()
            .map_err(|e| format!("Crossref 检索失败：{e}"))?
            .error_for_status()
            .map_err(|e| format!("Crossref 未找到该 DOI：{e}"))?;
        response
            .json::<CrossrefEnvelope<CrossrefWork>>()
            .map_err(|e| format!("Crossref 返回内容无法解析：{e}"))?
            .message
    } else {
        if normalized_tokens(title).len() < 2 {
            return Ok(None);
        }
        let mut url = url::Url::parse(CROSSREF_API).map_err(|e| e.to_string())?;
        url.query_pairs_mut()
            .append_pair("query.bibliographic", title)
            .append_pair("rows", "3");
        let response = client
            .get(url)
            .send()
            .map_err(|e| format!("Crossref 检索失败：{e}"))?
            .error_for_status()
            .map_err(|e| format!("Crossref 检索失败：{e}"))?;
        let list = response
            .json::<CrossrefEnvelope<CrossrefList>>()
            .map_err(|e| format!("Crossref 返回内容无法解析：{e}"))?
            .message;
        match list.items.into_iter().find(|work| {
            work.title
                .first()
                .is_some_and(|candidate| title_similarity(title, candidate) >= 0.72)
        }) {
            Some(work) => work,
            None => return Ok(None),
        }
    };
    Ok(Some(candidate_from_work(work)))
}

fn candidate_from_work(work: CrossrefWork) -> Candidate {
    let doi = clean(work.doi);
    let source_url = work
        .link
        .iter()
        .find(|link| link.content_type.as_deref() == Some("application/pdf"))
        .and_then(|link| clean(link.url.clone()))
        .or_else(|| {
            work.resource
                .and_then(|resource| resource.primary)
                .and_then(|primary| clean(primary.url))
        })
        .or_else(|| clean(work.url))
        .or_else(|| doi.as_ref().map(|value| format!("https://doi.org/{value}")));
    let year = [&work.published_print, &work.published_online, &work.issued]
        .into_iter()
        .flatten()
        .find_map(|date| date.date_parts.first()?.first().copied());
    Candidate {
        title: work.title.into_iter().find_map(|value| clean(Some(value))),
        authors: work
            .author
            .into_iter()
            .filter_map(|author| {
                clean(author.name).or_else(|| {
                    clean(Some(format!(
                        "{} {}",
                        author.given.unwrap_or_default(),
                        author.family.unwrap_or_default()
                    )))
                })
            })
            .collect(),
        year,
        doi,
        source_url,
        venue: work
            .container_title
            .into_iter()
            .find_map(|value| clean(Some(value))),
        publisher: clean(work.publisher),
    }
}

fn embedded_candidate(bytes: &[u8]) -> Candidate {
    let text = String::from_utf8_lossy(&bytes[..bytes.len().min(32 * 1024 * 1024)]);
    Candidate {
        title: xml_value(&text, "dc:title").or_else(|| xml_value(&text, "pdf:Title")),
        authors: xml_section(&text, "dc:creator")
            .map(|section| xml_values(section, "rdf:li"))
            .unwrap_or_default(),
        year: xml_value(&text, "prism:publicationDate")
            .and_then(|value| value.get(..4)?.parse().ok()),
        doi: extract_doi(&text),
        venue: xml_value(&text, "prism:publicationName"),
        ..Candidate::default()
    }
}

fn extract_doi(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    for index in 0..bytes.len().saturating_sub(7) {
        if bytes[index] != b'1'
            || bytes.get(index + 1) != Some(&b'0')
            || bytes.get(index + 2) != Some(&b'.')
        {
            continue;
        }
        let mut cursor = index + 3;
        let digits_start = cursor;
        while bytes.get(cursor).is_some_and(u8::is_ascii_digit) && cursor - digits_start < 9 {
            cursor += 1;
        }
        if !(4..=9).contains(&(cursor - digits_start)) || bytes.get(cursor) != Some(&b'/') {
            continue;
        }
        cursor += 1;
        let suffix_start = cursor;
        while let Some(byte) = bytes.get(cursor) {
            if byte.is_ascii_alphanumeric() || b"-._;()/:".contains(byte) {
                cursor += 1;
            } else {
                break;
            }
            if cursor - index >= 200 {
                break;
            }
        }
        if cursor == suffix_start {
            continue;
        }
        let value = lower[index..cursor]
            .trim_end_matches(['.', ',', ';', ':', ')'])
            .to_owned();
        return Some(value);
    }
    None
}

fn xml_value(text: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = text.find(&open)? + open.len();
    let raw = &text[start..text[start..].find(&close)? + start];
    let value = if let Some(li) = raw.find("<rdf:li") {
        let after = &raw[li..];
        let content = after.find('>')? + 1;
        &after[content..after[content..].find("</rdf:li>")? + content]
    } else {
        raw
    };
    clean(Some(decode_xml(value)))
}

fn xml_values(text: &str, tag: &str) -> Vec<String> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut rest = text;
    let mut values = Vec::new();
    while let Some(start) = rest.find(&open) {
        rest = &rest[start + open.len()..];
        let Some(content) = rest.find('>') else { break };
        rest = &rest[content + 1..];
        let Some(end) = rest.find(&close) else { break };
        if let Some(value) = clean(Some(decode_xml(&rest[..end]))) {
            values.push(value);
        }
        rest = &rest[end + close.len()..];
    }
    values
}

fn xml_section<'a>(text: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = text.find(&open)? + open.len();
    let end = text[start..].find(&close)? + start;
    Some(&text[start..end])
}

fn decode_xml(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}
fn clean(value: Option<String>) -> Option<String> {
    value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
}

fn merge(current: &Paper, embedded: Candidate, remote: Option<Candidate>) -> Candidate {
    let remote = remote.unwrap_or_default();
    Candidate {
        title: remote
            .title
            .or(embedded.title)
            .or_else(|| Some(current.title.clone())),
        authors: if current.authors.is_empty() {
            if remote.authors.is_empty() {
                embedded.authors
            } else {
                remote.authors
            }
        } else {
            current.authors.clone()
        },
        year: current.year.or(remote.year).or(embedded.year),
        doi: current.doi.clone().or(remote.doi).or(embedded.doi),
        source_url: current.source_url.clone().or(remote.source_url),
        venue: current.venue.clone().or(remote.venue).or(embedded.venue),
        publisher: current.publisher.clone().or(remote.publisher),
    }
}

fn same_metadata(paper: &Paper, candidate: &Candidate) -> bool {
    candidate.title.as_deref() == Some(&paper.title)
        && candidate.authors == paper.authors
        && candidate.year == paper.year
        && candidate.doi == paper.doi
        && candidate.source_url == paper.source_url
        && candidate.venue == paper.venue
        && candidate.publisher == paper.publisher
}

fn normalized_tokens(value: &str) -> Vec<String> {
    value
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|token| token.len() > 1)
        .map(str::to_owned)
        .collect()
}

fn title_similarity(left: &str, right: &str) -> f64 {
    let left = normalized_tokens(left);
    let right = normalized_tokens(right);
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let common = left.iter().filter(|token| right.contains(token)).count();
    (2 * common) as f64 / (left.len() + right.len()) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_doi_and_xmp_values() {
        let pdf = br#"%PDF <dc:title><rdf:Alt><rdf:li xml:lang="x-default">Attention &amp; Memory</rdf:li></rdf:Alt></dc:title><prism:doi>10.1234/ABC.567</prism:doi><prism:publicationName>Test Journal</prism:publicationName>"#;
        let candidate = embedded_candidate(pdf);
        assert_eq!(candidate.title.as_deref(), Some("Attention & Memory"));
        assert_eq!(candidate.doi.as_deref(), Some("10.1234/abc.567"));
        assert_eq!(candidate.venue.as_deref(), Some("Test Journal"));
    }

    #[test]
    fn rejects_unrelated_title_results() {
        assert!(title_similarity("Attention is all you need", "Attention is all you need") > 0.9);
        assert!(
            title_similarity(
                "Attention is all you need",
                "Protein folding with diffusion"
            ) < 0.3
        );
    }
}

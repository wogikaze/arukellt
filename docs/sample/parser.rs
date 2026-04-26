//! Gloss markup parser — generates an [`Event`] stream from Gloss Markdown.
//!
//! Gloss extends standard Markdown with five named features:
//! - **Ruby** (`[base/reading]`) — phonetic annotation above text
//! - **Anno** (`{base/note1/note2/...}`) — semantic annotation below text
//! - **Nest** (`---` / `;;;`) — explicit section-close markers
//! - **Math** (`$…$` / `$$…$$`) — KaTeX math; brackets inside are not parsed as Ruby/Anno
//! - **Lint** — parser warnings collected in [`Parser::warnings`] as [`Warning`] values

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;

// ── Warning codes ────────────────────────────────────────────────────────────

pub mod codes {
    pub const RUBY_KATAKANA_HIRAGANA:  &str = "ruby-katakana-hiragana";
    pub const RUBY_EMPTY_BASE:         &str = "ruby-empty-base";
    pub const RUBY_EMPTY_READING:      &str = "ruby-empty-reading";
    pub const RUBY_SELF_REFERENTIAL:   &str = "ruby-self-referential";
    pub const RUBY_MALFORMED:          &str = "ruby-malformed";
    pub const ANNO_LOOKS_LIKE_RUBY:    &str = "anno-looks-like-ruby";
    pub const ANNO_MALFORMED:          &str = "anno-malformed";
    pub const ANNO_EMPTY_BASE:         &str = "anno-empty-base";
    pub const KANJI_NO_RUBY:           &str = "kanji-no-ruby";
    pub const MATH_UNCLOSED_DISPLAY:   &str = "math-unclosed-display";
    pub const MATH_UNCLOSED_INLINE:    &str = "math-unclosed-inline";
    pub const FOOTNOTE_UNDEFINED_REF:  &str = "footnote-undefined-ref";
    pub const FOOTNOTE_UNUSED_DEF:     &str = "footnote-unused-def";
    pub const CARD_NON_HTTP:           &str = "card-non-http";
    pub const CARD_MALFORMED:          &str = "card-malformed";
    pub const CARD_UNKNOWN_TYPE:       &str = "card-unknown-type";
    pub const RUBY_KANA_BASE:          &str = "ruby-kana-base";
    pub const RUBY_KANJI_READING:      &str = "ruby-kanji-reading";
}

// ── Front matter ─────────────────────────────────────────────────────────────

/// A single `key: value` field from a YAML-style front matter block.
#[derive(Debug, Clone, PartialEq)]
pub struct FrontMatterField<'a> {
    pub key: &'a str,
    /// Raw value string as written (may be `"quoted"`, `['array']`, or bare).
    pub raw: &'a str,
}

/// Extract a front matter block from the very start of `text`.
/// The block must begin with `---\n` and end with a line containing only `---`.
/// Returns `(fields, remainder)` where `remainder` is the text after the closing `---`.
fn extract_front_matter<'a>(text: &'a str) -> Option<(Vec<FrontMatterField<'a>>, &'a str)> {
    if !text.starts_with("---\n") && !text.starts_with("---\r\n") {
        return None;
    }
    let after_open = text.find('\n').unwrap() + 1;
    let rest = &text[after_open..];

    // Find closing `---` line: `\n---\n`, `\n---\r\n`, `\n---` at EOF
    let close = rest.find("\n---\n").map(|p| (p, p + 5))
        .or_else(|| rest.find("\n---\r\n").map(|p| (p, p + 6)))
        .or_else(|| {
            if rest.ends_with("\n---") { Some((rest.len() - 4, rest.len())) } else { None }
        })?;

    let (fm_end, content_start) = close;
    let fm_str = &rest[..fm_end];
    let remainder = if content_start < rest.len() { &rest[content_start..] } else { "" };

    let mut fields = Vec::new();
    for line in fm_str.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        if let Some(colon) = line.find(':') {
            let key = line[..colon].trim();
            let raw = line[colon + 1..].trim();
            if !key.is_empty() {
                fields.push(FrontMatterField { key, raw });
            }
        }
    }
    Some((fields, remainder))
}

// ── Warning struct ────────────────────────────────────────────────────────────

/// A lint/parse warning with source position information.
#[derive(Debug, Clone)]
pub struct Warning {
    /// Machine-readable code (see [`codes`]).
    pub code: &'static str,
    /// Human-readable description.
    pub message: String,
    /// Source filename, or empty string if unknown.
    pub source: String,
    /// 1-based line number in the source file.
    pub line: u32,
    /// 1-based character (Unicode scalar) column in the source line.
    pub col: u32,
}

// ── Position context ─────────────────────────────────────────────────────────

/// Holds the original input and precomputed line-start offsets so that any
/// `&str` sub-slice of the input can be mapped to `(line, col)` via pointer
/// arithmetic in O(log n).
struct ParseCtx<'a> {
    input: &'a str,
    source: &'a str,
    /// Byte offset of the first byte of each line (index 0 → line 1).
    line_starts: Vec<usize>,
}

impl<'a> ParseCtx<'a> {
    fn new(input: &'a str, source: &'a str) -> Self {
        let mut starts = alloc::vec![0usize];
        let bytes = input.as_bytes();
        for i in 0..bytes.len() {
            if bytes[i] == b'\n' {
                starts.push(i + 1);
            }
        }
        ParseCtx { input, source, line_starts: starts }
    }

    /// Convert a sub-slice of `self.input` to `(1-based line, 1-based char col)`.
    ///
    /// If `s` is not a sub-slice of the input (e.g. a synthesised string),
    /// falls back to (1, 1).
    fn pos(&self, s: &str) -> (u32, u32) {
        let input_start = self.input.as_ptr() as usize;
        let s_start = s.as_ptr() as usize;
        if s_start < input_start || s_start > input_start + self.input.len() {
            return (1, 1);
        }
        let offset = s_start - input_start;
        // Find last line_start ≤ offset
        let line_idx = self.line_starts.partition_point(|&ls| ls <= offset);
        let line = line_idx as u32;            // 1-based
        let line_start = self.line_starts[line_idx.saturating_sub(1)];
        let col = self.input[line_start..offset].chars().count() as u32 + 1;
        (line, col)
    }

    fn warn(&self, warnings: &mut Vec<Warning>, code: &'static str, msg: String, at: &str) {
        let (line, col) = self.pos(at);
        warnings.push(Warning {
            code,
            message: msg,
            source: self.source.to_string(),
            line,
            col,
        });
    }
}

// ── AST types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Alignment { None, Left, Center, Right }

#[derive(Debug, Clone, PartialEq)]
pub enum Tag<'a> {
    Paragraph,
    Heading(u32),
    Section(u32),
    /// `[base/reading]` — reading stored for the End event.
    Ruby(&'a str),
    /// `{base/note1/note2/…}` — notes stored for the End event.
    Anno(Vec<&'a str>),
    AnnoNote,
    List(bool),
    Item,
    Code,
    CodeBlock(&'a str, &'a str),    // (lang, filename)
    FootnoteSection,
    FootnoteItem(u32),
    Blockquote,
    Table(Vec<Alignment>),
    TableHead,
    TableRow,
    TableCell(Alignment),
    Strong,
    Emphasis,
    Strikethrough,
    Link(&'a str),
    Image(&'a str, &'a str),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event<'a> {
    Start(Tag<'a>),
    End(Tag<'a>),
    Text(&'a str),
    MathDisplay(&'a str),
    MathInline(&'a str),
    SoftBreak,
    HardBreak,
    Rule,
    CardLink(&'a str),
    FootnoteRef(u32),
    /// Stable content-hash ID for the immediately following block element.
    /// Emitted by the parser before Paragraph / Heading / CodeBlock / etc.
    /// so the HTML renderer can attach `data-bid` attributes.
    BlockId(u64),
    /// Front matter fields from the leading `---` block.
    /// Always the first event when front matter is present.
    FrontMatter(Vec<FrontMatterField<'a>>),
}

// ── Public parser ─────────────────────────────────────────────────────────────

pub struct Parser<'a> {
    events: alloc::vec::IntoIter<Event<'a>>,
    pub warnings: Vec<Warning>,
}

impl<'a> Parser<'a> {
    /// Parse with an empty source label (warnings will show `source = ""`).
    pub fn new(text: &'a str) -> Self {
        Self::new_with_source(text, "")
    }

    /// Parse with a named source file label for warning messages.
    pub fn new_with_source(text: &'a str, source: &'a str) -> Self {
        let ctx = ParseCtx::new(text, source);
        let mut events: Vec<Event<'a>> = Vec::new();
        let mut warnings = Vec::new();

        // Strip front matter first; ParseCtx is built from the full text so
        // pointer-arithmetic line numbers remain correct for the remainder.
        let content = if let Some((fields, remainder)) = extract_front_matter(text) {
            events.push(Event::FrontMatter(fields));
            remainder
        } else {
            text
        };

        let lines: Vec<&str> = content.lines().collect();
        let fn_defs = collect_fn_defs(&lines);
        let mut fn_refs: Vec<&str> = Vec::new();
        parse_blocks(&lines, &mut events, &mut warnings, &ctx, true, &fn_defs, &mut fn_refs);
        emit_fn_section(&fn_defs, &fn_refs, &mut events, &mut warnings, &ctx);
        Parser { events: events.into_iter(), warnings }
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = Event<'a>;
    fn next(&mut self) -> Option<Self::Item> { self.events.next() }
}

// ── FNV-1a hash (no_std compatible) ─────────────────────────────────────────

pub fn fnv1a(s: &str) -> u64 {
    let mut h: u64 = 14695981039346656037;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

// ── Source block splitting ────────────────────────────────────────────────────

/// Split `input` into source blocks separated by blank lines, treating fenced
/// code blocks as atomic units.  Each returned slice is a sub-slice of `input`.
pub fn split_source_blocks(input: &str) -> Vec<&str> {
    let mut result: Vec<&str> = Vec::new();
    let mut block_start: Option<usize> = None;  // byte offset in `input`
    let mut block_end: usize = 0;
    let mut in_fence = false;
    let mut pos = 0usize;

    for line in input.split('\n') {
        let line_end = pos + line.len();  // exclusive, not counting \n
        let trimmed = line.trim_start();

        if trimmed.starts_with("```") {
            in_fence = !in_fence;
        }

        if !in_fence && trimmed.is_empty() {
            if let Some(start) = block_start {
                result.push(&input[start..block_end]);
                block_start = None;
            }
        } else {
            if block_start.is_none() { block_start = Some(pos); }
            block_end = line_end;
        }

        pos = line_end + 1;  // +1 for the \n character
    }

    if let Some(start) = block_start {
        result.push(&input[start..block_end]);
    }
    result
}

// ── Unicode helpers ───────────────────────────────────────────────────────────

fn is_kanji(c: char) -> bool {
    matches!(c,
        '\u{4E00}'..='\u{9FFF}' |
        '\u{3400}'..='\u{4DBF}' |
        '\u{20000}'..='\u{2A6DF}' |
        '\u{2A700}'..='\u{2B73F}' |
        '\u{2B740}'..='\u{2B81F}' |
        '\u{2B820}'..='\u{2CEAF}' |
        '\u{2CEB0}'..='\u{2EBEF}' |
        '\u{30000}'..='\u{3134F}' |
        '\u{F900}'..='\u{FAFF}' |
        '\u{2F800}'..='\u{2FA1F}' |
        '\u{3005}'
    )
}

fn contains_kanji(s: &str) -> bool { s.chars().any(is_kanji) }

fn is_purely_katakana(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| matches!(c,
        '\u{30A0}'..='\u{30FF}' |
        '\u{31F0}'..='\u{31FF}' |
        '\u{FF65}'..='\u{FF9F}'
    ))
}

fn is_purely_hiragana(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| matches!(c,
        '\u{3040}'..='\u{309F}' |
        '\u{30FC}'
    ))
}

/// True if `s` contains at least one hiragana or katakana character.
fn has_kana(s: &str) -> bool {
    s.chars().any(|c| matches!(c,
        '\u{3040}'..='\u{309F}' |   // Hiragana
        '\u{30A0}'..='\u{30FF}' |   // Katakana
        '\u{31F0}'..='\u{31FF}' |   // Katakana Phonetic Extensions
        '\u{FF65}'..='\u{FF9F}'     // Halfwidth Katakana
    ))
}

/// True if `s` contains at least one CJK ideograph (kanji).
fn has_kanji(s: &str) -> bool {
    s.chars().any(|c| matches!(c,
        '\u{3400}'..='\u{4DBF}' |   // CJK Extension A
        '\u{4E00}'..='\u{9FFF}' |   // CJK Unified Ideographs
        '\u{F900}'..='\u{FAFF}' |   // CJK Compatibility Ideographs
        '\u{20000}'..='\u{2A6DF}'   // CJK Extension B
    ))
}

fn is_purely_kana_or_punct(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| {
        matches!(c,
            '\u{3040}'..='\u{309F}' |
            '\u{30A0}'..='\u{30FF}' |
            '\u{31F0}'..='\u{31FF}' |
            '\u{FF65}'..='\u{FF9F}' |
            '\u{3000}'..='\u{303F}' |
            '\u{FE30}'..='\u{FE4F}' |
            '\u{FF00}'..='\u{FF60}' |
            '\u{FFE0}'..='\u{FFE6}' |
            '\u{02CA}' | '\u{02C7}' | '\u{02CB}' | '\u{02D9}' |
            '\u{31A0}'..='\u{31BF}' | '\u{3100}'..='\u{312F}' |
            '\u{AC00}'..='\u{D7AF}' |
            '\u{1100}'..='\u{11FF}' |
            '\u{3130}'..='\u{318F}' |
            '\u{0100}'..='\u{024F}' |
            '\u{1E00}'..='\u{1EFF}' |
            ' '
        )
    })
}

// ── Footnote helpers ──────────────────────────────────────────────────────────

fn collect_fn_defs<'a>(lines: &[&'a str]) -> Vec<(&'a str, &'a str)> {
    let mut defs: Vec<(&'a str, &'a str)> = Vec::new();
    for &line in lines {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("[^") {
            if let Some(colon_idx) = rest.find("]: ") {
                let id = &rest[..colon_idx];
                if !id.is_empty() && !id.contains(' ') && !defs.iter().any(|(did, _)| *did == id) {
                    defs.push((id, &rest[colon_idx + 3..]));
                }
            }
        }
    }
    defs
}

fn emit_fn_section<'a>(
    fn_defs: &[(&'a str, &'a str)],
    fn_refs: &[&'a str],
    events: &mut Vec<Event<'a>>,
    warnings: &mut Vec<Warning>,
    ctx: &ParseCtx<'a>,
) {
    for (id, content) in fn_defs {
        if !fn_refs.contains(id) {
            // Position: point at the definition line content
            ctx.warn(warnings, codes::FOOTNOTE_UNUSED_DEF,
                format!("Footnote '[^{}]' is defined but never referenced.", id),
                content);
        }
    }
    if fn_refs.is_empty() { return; }
    events.push(Event::Start(Tag::FootnoteSection));
    for (idx, &id) in fn_refs.iter().enumerate() {
        let num = (idx + 1) as u32;
        if let Some(&(_, content)) = fn_defs.iter().find(|(did, _)| *did == id) {
            events.push(Event::Start(Tag::FootnoteItem(num)));
            let mut nested_refs: Vec<&'a str> = Vec::new();
            parse_inline(content, events, warnings, ctx, false, fn_defs, &mut nested_refs);
            events.push(Event::End(Tag::FootnoteItem(num)));
        }
    }
    events.push(Event::End(Tag::FootnoteSection));
}

// ── Block parser ──────────────────────────────────────────────────────────────

fn parse_blocks<'a>(
    lines: &[&'a str],
    events: &mut Vec<Event<'a>>,
    warnings: &mut Vec<Warning>,
    ctx: &ParseCtx<'a>,
    root: bool,
    fn_defs: &[(&'a str, &'a str)],
    fn_refs: &mut Vec<&'a str>,
) {
    let mut i = 0;
    let mut section_stack: Vec<u32> = Vec::new();

    let pop_section = |events: &mut Vec<Event<'a>>, stack: &mut Vec<u32>| {
        if let Some(level) = stack.pop() {
            events.push(Event::End(Tag::Section(level)));
        }
    };

    let close_sections_until = |events: &mut Vec<Event<'a>>, stack: &mut Vec<u32>, level: u32| {
        while let Some(&top) = stack.last() {
            if top >= level { pop_section(events, stack); } else { break; }
        }
    };

    while i < lines.len() {
        let line = lines[i];
        let tline = line.trim_start();

        // Blank
        if tline.is_empty() { i += 1; continue; }

        // Footnote definition — skip (rendered in footnote section)
        if tline.starts_with("[^") && tline.contains("]: ") { i += 1; continue; }

        // Card link: @[type](url)
        if tline.starts_with("@[") {
            if let Some(bracket_end) = tline[2..].find(']') {
                let type_name = &tline[2..2 + bracket_end];
                let after = &tline[2 + bracket_end + 1..];
                if type_name == "card" {
                    if after.starts_with('(') && after.ends_with(')') {
                        let url = &after[1..after.len() - 1];
                        if !url.starts_with("http://") && !url.starts_with("https://") {
                            ctx.warn(warnings, codes::CARD_NON_HTTP,
                                format!("Card link URL '{}' should start with http:// or https://", url),
                                url);
                        }
                        events.push(Event::CardLink(url));
                    } else {
                        ctx.warn(warnings, codes::CARD_MALFORMED,
                            format!("Malformed @[card] syntax near '{}': expected @[card](URL).",
                                &tline[..tline.len().min(40)]),
                            tline);
                    }
                } else {
                    ctx.warn(warnings, codes::CARD_UNKNOWN_TYPE,
                        format!("Unknown embed type '{}' in '@[{}]': only 'card' is supported.",
                            type_name, type_name),
                        tline);
                }
            }
            i += 1;
            continue;
        }

        // Code fence
        if tline.starts_with("```") {
            let info = tline[3..].trim();
            let (lang, filename) = if let Some(colon) = info.find(':') {
                (&info[..colon], &info[colon + 1..])
            } else {
                (info, "")
            };
            events.push(Event::BlockId(fnv1a(tline)));
            events.push(Event::Start(Tag::CodeBlock(lang, filename)));
            i += 1;
            while i < lines.len() && !lines[i].trim_start().starts_with("```") {
                events.push(Event::Text(lines[i]));
                events.push(Event::Text("\n"));
                i += 1;
            }
            if i < lines.len() { i += 1; }
            events.push(Event::End(Tag::CodeBlock(lang, filename)));
            continue;
        }

        // Heading
        if tline.starts_with('#') {
            let bytes = tline.as_bytes();
            let mut level = 0;
            while level < bytes.len() && bytes[level] == b'#' { level += 1; }
            if level > 0 && level <= 6 && (level == bytes.len() || bytes[level] == b' ') {
                if root {
                    close_sections_until(events, &mut section_stack, level as u32);
                    events.push(Event::Start(Tag::Section(level as u32)));
                    section_stack.push(level as u32);
                }
                events.push(Event::BlockId(fnv1a(tline)));
                events.push(Event::Start(Tag::Heading(level as u32)));
                parse_inline(tline[level..].trim(), events, warnings, ctx, false, fn_defs, fn_refs);
                events.push(Event::End(Tag::Heading(level as u32)));
                i += 1;
                continue;
            }
        }

        // Thematic break → close one section + emit <hr/>
        if line.starts_with("---") && line.chars().all(|c| c == '-') {
            if root && line.len() == 3 { pop_section(events, &mut section_stack); }
            events.push(Event::Rule);
            i += 1;
            continue;
        }

        // Section close (;;;)
        if line.starts_with(";;;") {
            if root {
                let count = line.matches(";;;").count();
                for _ in 0..count { pop_section(events, &mut section_stack); }
            }
            i += 1;
            continue;
        }

        // Blockquote
        if line.starts_with('>') {
            let mut bq_lines = Vec::new();
            let mut j = i;
            while j < lines.len() {
                let ln = lines[j];
                if ln.starts_with('>') {
                    let mut content = &ln[1..];
                    if content.starts_with(' ') { content = &content[1..]; }
                    bq_lines.push(content);
                    j += 1;
                } else if ln.trim().is_empty() && j > i && j + 1 < lines.len() && lines[j + 1].starts_with('>') {
                    bq_lines.push("");
                    j += 1;
                } else {
                    break;
                }
            }
            // hash = hash of all source lines combined
            let src_hash = {
                let mut h: u64 = 14695981039346656037;
                for ln in &bq_lines { for b in ln.bytes() { h ^= b as u64; h = h.wrapping_mul(1099511628211); } }
                h
            };
            events.push(Event::BlockId(src_hash));
            events.push(Event::Start(Tag::Blockquote));
            parse_blocks(&bq_lines, events, warnings, ctx, false, fn_defs, fn_refs);
            events.push(Event::End(Tag::Blockquote));
            i = j;
            continue;
        }

        // Table
        let is_table_line = |l: &str| l.trim_start().starts_with('|');
        if is_table_line(line) && i + 1 < lines.len() && is_table_line(lines[i + 1]) {
            let sep_line = lines[i + 1].trim();
            if sep_line.contains("-|") || sep_line.contains("|-") {
                let parse_cells = |l: &'a str| -> Vec<&'a str> {
                    let t = l.trim();
                    let t = if t.starts_with('|') { &t[1..] } else { t };
                    let t = if t.ends_with('|') { &t[..t.len()-1] } else { t };
                    t.split('|').map(|s| s.trim()).collect()
                };
                let head = parse_cells(line);
                let sep  = parse_cells(lines[i + 1]);
                let aligns: Vec<Alignment> = sep.iter().map(|s| {
                    let s = s.trim();
                    let left = s.starts_with(':');
                    let right = s.ends_with(':');
                    if left && right { Alignment::Center }
                    else if left    { Alignment::Left }
                    else if right   { Alignment::Right }
                    else            { Alignment::None }
                }).collect();
                events.push(Event::BlockId(fnv1a(line)));
                events.push(Event::Start(Tag::Table(aligns.clone())));
                events.push(Event::Start(Tag::TableHead));
                events.push(Event::Start(Tag::TableRow));
                for (ci, cell) in head.iter().enumerate() {
                    let a = aligns.get(ci).cloned().unwrap_or(Alignment::None);
                    events.push(Event::Start(Tag::TableCell(a.clone())));
                    parse_inline(cell, events, warnings, ctx, false, fn_defs, fn_refs);
                    events.push(Event::End(Tag::TableCell(a)));
                }
                events.push(Event::End(Tag::TableRow));
                events.push(Event::End(Tag::TableHead));
                let mut j = i + 2;
                while j < lines.len() && is_table_line(lines[j]) {
                    events.push(Event::Start(Tag::TableRow));
                    let row = parse_cells(lines[j]);
                    for (ci, cell) in row.iter().enumerate() {
                        let a = aligns.get(ci).cloned().unwrap_or(Alignment::None);
                        events.push(Event::Start(Tag::TableCell(a.clone())));
                        parse_inline(cell, events, warnings, ctx, false, fn_defs, fn_refs);
                        events.push(Event::End(Tag::TableCell(a)));
                    }
                    events.push(Event::End(Tag::TableRow));
                    j += 1;
                }
                events.push(Event::End(Tag::Table(aligns)));
                i = j;
                continue;
            }
        }

        // Ordered / Unordered list
        let is_ul = tline.starts_with("- ") || tline.starts_with("* ");
        let dig_count = tline.chars().take_while(|c| c.is_ascii_digit()).count();
        let is_ol = dig_count > 0 && tline[dig_count..].starts_with(". ");

        if is_ul || is_ol {
            events.push(Event::BlockId(fnv1a(tline)));
            events.push(Event::Start(Tag::List(is_ol)));
            let mut j = i;
            while j < lines.len() {
                let l2 = lines[j].trim_start();
                let is_ul2 = l2.starts_with("- ") || l2.starts_with("* ");
                let d2 = l2.chars().take_while(|c| c.is_ascii_digit()).count();
                let is_ol2 = d2 > 0 && l2[d2..].starts_with(". ");
                if (is_ol && is_ol2) || (!is_ol && is_ul2) {
                    let content = if is_ul2 { &l2[2..] } else { &l2[d2 + 2..] };
                    events.push(Event::Start(Tag::Item));
                    parse_inline(content, events, warnings, ctx, false, fn_defs, fn_refs);
                    events.push(Event::End(Tag::Item));
                    j += 1;
                } else { break; }
            }
            events.push(Event::End(Tag::List(is_ol)));
            i = j;
            continue;
        }

        // Paragraph: collect consecutive non-block lines
        let mut para: Vec<&'a str> = Vec::new();
        let mut j = i;
        while j < lines.len() {
            let ln = lines[j];
            let t = ln.trim_start();
            if t.is_empty()
                || t.starts_with("```")
                || t.starts_with('#')
                || (ln.starts_with("---") && ln.chars().all(|c| c == '-'))
                || ln.starts_with(";;;")
                || ln.starts_with('>')
                || is_table_line(ln)
                || t.starts_with("- ")
                || t.starts_with("* ")
                || (t.chars().take_while(|c| c.is_ascii_digit()).count() > 0
                    && t[t.chars().take_while(|c| c.is_ascii_digit()).count()..].starts_with(". "))
                || t.starts_with("@[")
                || (t.starts_with("[^") && t.contains("]: "))
            { break; }
            para.push(ln);
            j += 1;
        }

        if !para.is_empty() {
            // Hash paragraph content for incremental rendering
            let para_hash = {
                let mut h: u64 = 14695981039346656037;
                for ln in &para { for b in ln.bytes() { h ^= b as u64; h = h.wrapping_mul(1099511628211); } }
                h
            };
            events.push(Event::BlockId(para_hash));
            events.push(Event::Start(Tag::Paragraph));
            for (pidx, pline) in para.iter().enumerate() {
                parse_inline(pline, events, warnings, ctx, false, fn_defs, fn_refs);
                if pidx < para.len() - 1 { events.push(Event::HardBreak); }
            }
            events.push(Event::End(Tag::Paragraph));
        }
        i = j;
    }

    if root {
        while !section_stack.is_empty() { pop_section(events, &mut section_stack); }
    }
}

// ── Inline parser ─────────────────────────────────────────────────────────────

fn parse_inline<'a>(
    mut text: &'a str,
    events: &mut Vec<Event<'a>>,
    warnings: &mut Vec<Warning>,
    ctx: &ParseCtx<'a>,
    in_annotation: bool,
    fn_defs: &[(&'a str, &'a str)],
    fn_refs: &mut Vec<&'a str>,
) {
    while !text.is_empty() {
        // $$ math display
        if text.starts_with("$$") {
            if let Some(end) = text[2..].find("$$") {
                events.push(Event::MathDisplay(&text[2..2 + end]));
                text = &text[2 + end + 2..];
                continue;
            } else {
                ctx.warn(warnings, codes::MATH_UNCLOSED_DISPLAY,
                    "Unclosed '$$' math block: no matching '$$' found.".to_string(), text);
            }
        }
        // $ math inline
        if text.starts_with('$') && !text.starts_with("$$") {
            if let Some(end) = text[1..].find('$') {
                events.push(Event::MathInline(&text[1..1 + end]));
                text = &text[1 + end + 1..];
                continue;
            } else {
                ctx.warn(warnings, codes::MATH_UNCLOSED_INLINE,
                    "Unclosed '$' math expression: no matching '$' found.".to_string(), text);
            }
        }
        // `code`
        if text.starts_with('`') {
            if let Some(end) = text[1..].find('`') {
                events.push(Event::Start(Tag::Code));
                events.push(Event::Text(&text[1..1 + end]));
                events.push(Event::End(Tag::Code));
                text = &text[1 + end + 1..];
                continue;
            }
        }
        // ~~strike~~
        if text.starts_with("~~") {
            if let Some(end) = text[2..].find("~~") {
                events.push(Event::Start(Tag::Strikethrough));
                parse_inline(&text[2..2 + end], events, warnings, ctx, in_annotation, fn_defs, fn_refs);
                events.push(Event::End(Tag::Strikethrough));
                text = &text[2 + end + 2..];
                continue;
            }
        }
        // **bold**
        if text.starts_with("**") {
            if let Some(end) = text[2..].find("**") {
                events.push(Event::Start(Tag::Strong));
                parse_inline(&text[2..2 + end], events, warnings, ctx, in_annotation, fn_defs, fn_refs);
                events.push(Event::End(Tag::Strong));
                text = &text[2 + end + 2..];
                continue;
            }
        }
        // *em*
        if text.starts_with('*') && !text.starts_with("**") {
            if let Some(end) = text[1..].find('*') {
                events.push(Event::Start(Tag::Emphasis));
                parse_inline(&text[1..1 + end], events, warnings, ctx, in_annotation, fn_defs, fn_refs);
                events.push(Event::End(Tag::Emphasis));
                text = &text[1 + end + 1..];
                continue;
            }
        }
        // \n → hard break
        if text.starts_with("\\n") {
            events.push(Event::HardBreak);
            text = &text[2..];
            continue;
        }
        // Escape: \X → literal X
        if text.starts_with('\\') && text.len() >= 2 {
            let ch = text[1..].chars().next().unwrap();
            let len = ch.len_utf8();
            events.push(Event::Text(&text[1..1 + len]));
            text = &text[1 + len..];
            continue;
        }
        // ![alt](src) image
        if text.starts_with("![") {
            let mut bracket = 0;
            let mut close_alt = None;
            for (idx, c) in text[1..].char_indices() {
                if c == '[' { bracket += 1; }
                else if c == ']' { bracket -= 1; if bracket == 0 { close_alt = Some(idx + 1); break; } }
            }
            if let Some(ca) = close_alt {
                if text.len() > ca + 1 && text.as_bytes()[ca + 1] == b'(' {
                    if let Some(cp) = text[ca + 2..].find(')') {
                        let close_src = ca + 2 + cp;
                        let alt = &text[2..ca];
                        let src = &text[ca + 2..close_src];
                        events.push(Event::Start(Tag::Image(src, alt)));
                        events.push(Event::End(Tag::Image(src, alt)));
                        text = &text[close_src + 1..];
                        continue;
                    }
                }
            }
        }
        // Footnote reference: [^id]
        if text.starts_with("[^") {
            if let Some(bracket_end) = text[2..].find(']') {
                let id = &text[2..2 + bracket_end];
                if !id.is_empty() && !id.contains(' ') {
                    let total_len = 2 + bracket_end + 1;
                    if fn_defs.iter().any(|(did, _)| *did == id) {
                        let num = if let Some(pos) = fn_refs.iter().position(|r| *r == id) {
                            (pos + 1) as u32
                        } else {
                            fn_refs.push(id);
                            fn_refs.len() as u32
                        };
                        events.push(Event::FootnoteRef(num));
                    } else {
                        ctx.warn(warnings, codes::FOOTNOTE_UNDEFINED_REF,
                            format!("Footnote reference '[^{}]' has no matching definition.", id),
                            &text[..total_len]);
                        events.push(Event::Text(&text[..total_len]));
                    }
                    text = &text[total_len..];
                    continue;
                }
            }
        }

        // [content](url) link  or  [base/reading] ruby
        if text.starts_with('[') {
            let mut bracket = 0;
            let mut close_bracket = None;
            for (idx, c) in text.char_indices() {
                if c == '[' { bracket += 1; }
                else if c == ']' { bracket -= 1; if bracket == 0 { close_bracket = Some(idx); break; } }
            }
            if let Some(cb) = close_bracket {
                let content = &text[1..cb];
                // [text](url) link
                if text.len() > cb + 1 && text.as_bytes()[cb + 1] == b'(' {
                    if let Some(cp) = text[cb + 2..].find(')') {
                        let close_paren = cb + 2 + cp;
                        let href = &text[cb + 2..close_paren];
                        events.push(Event::Start(Tag::Link(href)));
                        parse_inline(content, events, warnings, ctx, in_annotation, fn_defs, fn_refs);
                        events.push(Event::End(Tag::Link(href)));
                        text = &text[close_paren + 1..];
                        continue;
                    }
                }
                // [base/ruby]: find first '/' at bracket-depth 0
                let slash_idx = {
                    let mut blk = 0i32;
                    let mut found = None;
                    for (idx, c) in content.char_indices() {
                        if c == '[' { blk += 1; }
                        else if c == ']' { blk -= 1; }
                        else if c == '/' && blk == 0 { found = Some(idx); break; }
                    }
                    found
                };
                if let Some(slash) = slash_idx {
                    let base  = &content[..slash];
                    let ruby  = &content[slash + 1..];
                    let token = &text[..cb + 1]; // whole [base/ruby] for position
                    // ── Lint checks ───────────────────────────────────────────
                    if base.is_empty() {
                        ctx.warn(warnings, codes::RUBY_EMPTY_BASE,
                            format!("Ruby '[{}]': base text is empty.", content), token);
                    }
                    if ruby.is_empty() {
                        ctx.warn(warnings, codes::RUBY_EMPTY_READING,
                            format!("Ruby '[{}]': reading is empty.", content), token);
                    }
                    if !base.is_empty() && !ruby.is_empty() && base == ruby {
                        ctx.warn(warnings, codes::RUBY_SELF_REFERENTIAL,
                            format!("Ruby '[{}/{}]': base and reading are identical.", base, ruby), token);
                    }
                    if is_purely_katakana(base) && is_purely_hiragana(ruby) {
                        ctx.warn(warnings, codes::RUBY_KATAKANA_HIRAGANA,
                            format!(
                                "Ruby '[{}/{}]': katakana base with hiragana reading. \
                                 Katakana is already phonetic; use romanization or the original script instead.",
                                base, ruby),
                            token);
                    } else if !base.is_empty() && has_kana(base) {
                        // More general kana-in-base check; skipped when the more specific
                        // ruby-katakana-hiragana already fired to avoid double-warning.
                        ctx.warn(warnings, codes::RUBY_KANA_BASE,
                            format!(
                                "Ruby '[{}/{}]': base text contains kana. \
                                 Ruby is intended for kanji; kana bases are usually unnecessary.",
                                base, ruby),
                            token);
                    }
                    if !ruby.is_empty() && has_kanji(ruby) {
                        ctx.warn(warnings, codes::RUBY_KANJI_READING,
                            format!(
                                "Ruby '[{}/{}]': reading contains kanji. \
                                 Readings should be in kana or romanization, not kanji.",
                                base, ruby),
                            token);
                    }
                    events.push(Event::Start(Tag::Ruby(ruby)));
                    parse_inline(base, events, warnings, ctx, true, fn_defs, fn_refs);
                    events.push(Event::End(Tag::Ruby(ruby)));
                    text = &text[cb + 1..];
                    continue;
                }
                // Has ']' but no '/' — check for broken pattern
                if content.contains('/') {
                    ctx.warn(warnings, codes::RUBY_MALFORMED,
                        format!("Possibly malformed ruby syntax '[{}]': has '/' but nested brackets prevent parsing.",
                            &content[..content.len().min(30)]),
                        &text[..cb + 1]);
                }
            } else {
                // No matching ']'
                if text[1..].contains('/') || text[1..].contains(']') {
                    let snippet: String = text.chars().take(30).collect();
                    ctx.warn(warnings, codes::RUBY_MALFORMED,
                        format!("Possibly malformed ruby syntax: '[' with no matching ']' near '{}'.", snippet),
                        text);
                }
            }
        }

        // {base/note1/note2…}
        if text.starts_with('{') {
            let mut bracket = 0;
            let mut close_brace = None;
            for (idx, c) in text.char_indices() {
                if c == '{' { bracket += 1; }
                else if c == '}' { bracket -= 1; if bracket == 0 { close_brace = Some(idx); break; } }
            }
            if let Some(end) = close_brace {
                let content = &text[1..end];
                // split by '/' at bracket-depth 0
                let mut parts: Vec<&str> = Vec::new();
                let mut last = 0;
                let mut blk = 0i32;
                for (idx, c) in content.char_indices() {
                    if c == '[' { blk += 1; }
                    else if c == ']' { blk -= 1; }
                    else if c == '/' && blk == 0 { parts.push(&content[last..idx]); last = idx + 1; }
                }
                parts.push(&content[last..]);
                let token = &text[..end + 1]; // whole {…} for position

                if parts.len() >= 2 {
                    let base  = parts[0];
                    let notes = &parts[1..];

                    // ── Lint checks ───────────────────────────────────────────
                    if base.is_empty() {
                        ctx.warn(warnings, codes::ANNO_EMPTY_BASE,
                            format!("Anno '{}': base text is empty.", content), token);
                    }
                    // {漢字/かな} — single purely-kana note looks like ruby
                    if notes.len() == 1 && contains_kanji(base) && is_purely_kana_or_punct(notes[0]) {
                        ctx.warn(warnings, codes::ANNO_LOOKS_LIKE_RUBY,
                            format!("Anno '{{{}/{}}}' looks like a Ruby reading. Did you mean '[{}/{}]'?",
                                base, notes[0], base, notes[0]),
                            token);
                    }
                    let notes_owned: Vec<&str> = notes.to_vec();
                    events.push(Event::Start(Tag::Anno(notes_owned.clone())));
                    parse_inline(base, events, warnings, ctx, true, fn_defs, fn_refs);
                    for note in notes_owned.iter() {
                        events.push(Event::Start(Tag::AnnoNote));
                        parse_inline(note, events, warnings, ctx, true, fn_defs, fn_refs);
                        events.push(Event::End(Tag::AnnoNote));
                    }
                    events.push(Event::End(Tag::Anno(notes_owned)));
                    text = &text[end + 1..];
                    continue;
                }
                // {text} with no '/' — fall through to plain text
            } else {
                let snippet: String = text.chars().take(30).collect();
                ctx.warn(warnings, codes::ANNO_MALFORMED,
                    format!("Possibly malformed anno syntax: '{{' with no matching '}}' near '{}'.", snippet),
                    text);
            }
        }

        // Plain text up to next special character
        let next_special = text
            .find(|c| matches!(c, '$' | '[' | '{' | '`' | '\\' | '*' | '~' | '!'))
            .unwrap_or(text.len());

        if next_special == 0 {
            let ch = text.chars().next().unwrap();
            let len = ch.len_utf8();
            let t = &text[..len];
            if !in_annotation && contains_kanji(t) {
                ctx.warn(warnings, codes::KANJI_NO_RUBY,
                    format!("Kanji without ruby: '{}'", t.trim()), t);
            }
            events.push(Event::Text(t));
            text = &text[len..];
        } else {
            let t = &text[..next_special];
            if !in_annotation && contains_kanji(t) {
                ctx.warn(warnings, codes::KANJI_NO_RUBY,
                    format!("Kanji without ruby: '{}'", t.trim()), t);
            }
            events.push(Event::Text(t));
            text = &text[next_special..];
        }
    }
}
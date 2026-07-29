use chrono::{SecondsFormat, Utc};
use printpdf::{
    DateTime, Mm, Op, ParsedFont, PdfDocument, PdfFontHandle, PdfPage, PdfSaveOptions, Point, Pt,
    TextItem,
};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::fs::{self, File};
use std::io::{Seek, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

const SOURCE_SERIF_REGULAR: &[u8] = include_bytes!("../assets/fonts/SourceSerif4-Regular.ttf");
const SOURCE_SERIF_BOLD: &[u8] = include_bytes!("../assets/fonts/SourceSerif4-Bold.ttf");
const SOURCE_SERIF_ITALIC: &[u8] = include_bytes!("../assets/fonts/SourceSerif4-It.ttf");
const SOURCE_SERIF_BOLD_ITALIC: &[u8] = include_bytes!("../assets/fonts/SourceSerif4-BoldIt.ttf");

const LETTER_WIDTH_MM: f32 = 215.9;
const LETTER_HEIGHT_MM: f32 = 279.4;
const PAGE_WIDTH_PT: f32 = 612.0;
const PAGE_HEIGHT_PT: f32 = 792.0;
const PAGE_MARGIN_PT: f32 = 72.0;
const BODY_SIZE_PT: f32 = 12.0;
const BODY_LINE_HEIGHT_PT: f32 = 24.0;

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
  <Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
  <Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/>
</Types>"#;

const PACKAGE_RELATIONSHIPS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/>
</Relationships>"#;

const DOCUMENT_RELATIONSHIPS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
</Relationships>"#;

const APP_PROPERTIES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes">
  <Application>Writing Environment</Application>
  <AppVersion>0.5</AppVersion>
</Properties>"#;

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:docDefaults>
    <w:rPrDefault><w:rPr><w:rFonts w:ascii="Times New Roman" w:hAnsi="Times New Roman" w:eastAsia="Times New Roman" w:cs="Times New Roman"/><w:sz w:val="24"/><w:szCs w:val="24"/></w:rPr></w:rPrDefault>
    <w:pPrDefault><w:pPr><w:spacing w:after="0" w:line="480" w:lineRule="auto"/></w:pPr></w:pPrDefault>
  </w:docDefaults>
  <w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/><w:pPr><w:spacing w:after="0" w:line="480" w:lineRule="auto"/></w:pPr></w:style>
  <w:style w:type="paragraph" w:styleId="BodyText"><w:name w:val="Manuscript Body"/><w:basedOn w:val="Normal"/><w:next w:val="BodyText"/><w:qFormat/><w:pPr><w:ind w:firstLine="720"/></w:pPr></w:style>
  <w:style w:type="paragraph" w:styleId="Title"><w:name w:val="Manuscript Title"/><w:basedOn w:val="Normal"/><w:next w:val="BodyText"/><w:qFormat/><w:pPr><w:keepNext/><w:jc w:val="center"/><w:spacing w:after="480"/></w:pPr><w:rPr><w:b/><w:sz w:val="28"/><w:szCs w:val="28"/></w:rPr></w:style>
  <w:style w:type="paragraph" w:styleId="Byline"><w:name w:val="Author Byline"/><w:basedOn w:val="Normal"/><w:next w:val="BodyText"/><w:qFormat/><w:pPr><w:jc w:val="center"/><w:spacing w:before="0" w:after="240"/></w:pPr><w:rPr><w:i/><w:sz w:val="22"/><w:szCs w:val="22"/></w:rPr></w:style>
  <w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="Heading 1"/><w:basedOn w:val="Normal"/><w:next w:val="BodyText"/><w:qFormat/><w:pPr><w:keepNext/><w:keepLines/><w:jc w:val="center"/><w:spacing w:before="480" w:after="240"/></w:pPr><w:rPr><w:b/><w:sz w:val="26"/><w:szCs w:val="26"/></w:rPr></w:style>
  <w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="Heading 2"/><w:basedOn w:val="Normal"/><w:next w:val="BodyText"/><w:qFormat/><w:pPr><w:keepNext/><w:keepLines/><w:spacing w:before="360" w:after="120"/></w:pPr><w:rPr><w:b/></w:rPr></w:style>
  <w:style w:type="paragraph" w:styleId="Heading3"><w:name w:val="Heading 3"/><w:basedOn w:val="Normal"/><w:next w:val="BodyText"/><w:qFormat/><w:pPr><w:keepNext/><w:keepLines/><w:spacing w:before="240"/></w:pPr><w:rPr><w:b/><w:i/></w:rPr></w:style>
  <w:style w:type="paragraph" w:styleId="Quotation"><w:name w:val="Quotation"/><w:basedOn w:val="Normal"/><w:next w:val="BodyText"/><w:qFormat/><w:pPr><w:ind w:left="720" w:right="720"/></w:pPr><w:rPr><w:i/></w:rPr></w:style>
  <w:style w:type="paragraph" w:styleId="ListParagraph"><w:name w:val="List Paragraph"/><w:basedOn w:val="Normal"/><w:next w:val="ListParagraph"/><w:qFormat/></w:style>
  <w:style w:type="paragraph" w:styleId="SceneBreak"><w:name w:val="Scene Break"/><w:basedOn w:val="Normal"/><w:next w:val="BodyText"/><w:qFormat/><w:pPr><w:jc w:val="center"/><w:spacing w:before="240" w:after="240"/></w:pPr></w:style>
  <w:style w:type="paragraph" w:styleId="CodeBlock"><w:name w:val="Code Block"/><w:basedOn w:val="Normal"/><w:next w:val="BodyText"/><w:qFormat/><w:pPr><w:ind w:left="720"/><w:spacing w:after="120" w:line="240" w:lineRule="auto"/></w:pPr><w:rPr><w:rFonts w:ascii="Courier New" w:hAnsi="Courier New"/><w:sz w:val="20"/><w:szCs w:val="20"/></w:rPr></w:style>
  <w:style w:type="character" w:styleId="CodeChar"><w:name w:val="Inline Code"/><w:basedOn w:val="DefaultParagraphFont"/><w:rPr><w:rFonts w:ascii="Courier New" w:hAnsi="Courier New"/><w:sz w:val="21"/><w:szCs w:val="21"/></w:rPr></w:style>
</w:styles>"#;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct InlineStyle {
    bold: u8,
    italic: u8,
    strike: u8,
    code: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Run {
    text: String,
    style: InlineStyle,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ParagraphKind {
    Body,
    Title,
    Byline,
    Heading(u8),
    Quote,
    List { depth: usize },
    SceneBreak,
    Code,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Paragraph {
    kind: ParagraphKind,
    runs: Vec<Run>,
    page_break_before: bool,
}

impl Paragraph {
    fn new(kind: ParagraphKind) -> Self {
        Self {
            kind,
            runs: Vec::new(),
            page_break_before: false,
        }
    }

    fn push(&mut self, text: &str, style: InlineStyle) {
        if text.is_empty() {
            return;
        }
        if let Some(last) = self.runs.last_mut().filter(|run| run.style == style) {
            last.text.push_str(text);
        } else {
            self.runs.push(Run {
                text: text.to_string(),
                style,
            });
        }
    }

    fn plain_text(&self) -> String {
        self.runs.iter().map(|run| run.text.as_str()).collect()
    }

    fn has_text(&self) -> bool {
        self.runs.iter().any(|run| !run.text.trim().is_empty())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExportSection {
    pub title: String,
    pub markdown: String,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ExportOptions {
    pub title_page: bool,
    pub page_breaks: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExportMetadata {
    pub author: String,
    pub language: String,
}

#[derive(Clone, Copy, Debug)]
struct ListState {
    ordered: bool,
    next: u64,
}

#[derive(Debug, Default)]
struct MarkdownBuilder {
    paragraphs: Vec<Paragraph>,
    current: Option<Paragraph>,
    inline: InlineStyle,
    quote_depth: usize,
    lists: Vec<ListState>,
    in_item: bool,
    item_marker: Option<String>,
}

impl MarkdownBuilder {
    fn inferred_kind(&self) -> ParagraphKind {
        if !self.lists.is_empty() && self.in_item {
            ParagraphKind::List {
                depth: self.lists.len(),
            }
        } else if self.quote_depth > 0 {
            ParagraphKind::Quote
        } else {
            ParagraphKind::Body
        }
    }

    fn start(&mut self, kind: ParagraphKind) {
        self.finish();
        let mut paragraph = Paragraph::new(kind);
        if matches!(paragraph.kind, ParagraphKind::List { .. }) {
            if let Some(marker) = self.item_marker.take() {
                paragraph.push(&marker, InlineStyle::default());
            }
        }
        self.current = Some(paragraph);
    }

    fn ensure_current(&mut self) {
        if self.current.is_none() {
            let kind = self.inferred_kind();
            self.start(kind);
        }
    }

    fn push(&mut self, text: &str) {
        self.ensure_current();
        if let Some(current) = self.current.as_mut() {
            current.push(text, self.inline);
        }
    }

    fn push_with_style(&mut self, text: &str, style: InlineStyle) {
        self.ensure_current();
        if let Some(current) = self.current.as_mut() {
            current.push(text, style);
        }
    }

    fn finish(&mut self) {
        if let Some(paragraph) = self.current.take() {
            if paragraph.has_text() || matches!(paragraph.kind, ParagraphKind::SceneBreak) {
                self.paragraphs.push(paragraph);
            }
        }
    }

    fn start_item(&mut self) {
        self.in_item = true;
        self.item_marker = self.lists.last_mut().map(|list| {
            if list.ordered {
                let marker = format!("{}. ", list.next);
                list.next = list.next.saturating_add(1);
                marker
            } else {
                "• ".to_string()
            }
        });
    }
}

pub fn export_sheet_docx(path: &str, title: &str, markdown: &str) -> Result<String, String> {
    export_document_docx(
        path,
        title,
        &[ExportSection {
            title: title.to_string(),
            markdown: markdown.to_string(),
        }],
        ExportOptions::default(),
        &ExportMetadata::default(),
    )
}

pub fn export_document_docx(
    path: &str,
    title: &str,
    sections: &[ExportSection],
    options: ExportOptions,
    metadata: &ExportMetadata,
) -> Result<String, String> {
    let target = validate_destination(path)?;
    let parent = target
        .parent()
        .ok_or_else(|| "The export destination has no parent folder.".to_string())?;
    let mut temporary = NamedTempFile::new_in(parent)
        .map_err(|error| format!("Cannot create the temporary Word document: {error}"))?;

    write_docx_document(temporary.as_file_mut(), title, sections, options, metadata)?;
    if let Ok(metadata) = fs::metadata(&target) {
        temporary
            .as_file_mut()
            .set_permissions(metadata.permissions())
            .map_err(|error| {
                format!("Cannot preserve the existing document permissions: {error}")
            })?;
    }
    temporary
        .as_file_mut()
        .sync_all()
        .map_err(|error| format!("Cannot flush the Word document: {error}"))?;
    temporary
        .persist(&target)
        .map_err(|error| format!("Cannot save the Word document: {}", error.error))?;

    #[cfg(unix)]
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("Cannot flush the export folder: {error}"))?;

    Ok(target.to_string_lossy().into_owned())
}

pub fn export_sheet_pdf(path: &str, title: &str, markdown: &str) -> Result<String, String> {
    export_document_pdf(
        path,
        title,
        &[ExportSection {
            title: title.to_string(),
            markdown: markdown.to_string(),
        }],
        ExportOptions::default(),
        &ExportMetadata::default(),
    )
}

pub fn export_document_pdf(
    path: &str,
    title: &str,
    sections: &[ExportSection],
    options: ExportOptions,
    metadata: &ExportMetadata,
) -> Result<String, String> {
    let target = validate_export_destination(path, "pdf", "PDF")?;
    let bytes = pdf_document_bytes(title, sections, options, metadata)?;
    persist_bytes(&target, &bytes, "PDF document")
}

pub fn export_sheet_epub(path: &str, title: &str, markdown: &str) -> Result<String, String> {
    export_document_epub(
        path,
        title,
        &[ExportSection {
            title: title.to_string(),
            markdown: markdown.to_string(),
        }],
        ExportOptions::default(),
        &ExportMetadata::default(),
    )
}

pub fn export_document_epub(
    path: &str,
    title: &str,
    sections: &[ExportSection],
    options: ExportOptions,
    metadata: &ExportMetadata,
) -> Result<String, String> {
    let target = validate_export_destination(path, "epub", "EPUB")?;
    let parent = target
        .parent()
        .ok_or_else(|| "The export destination has no parent folder.".to_string())?;
    let mut temporary = NamedTempFile::new_in(parent)
        .map_err(|error| format!("Cannot create the temporary EPUB: {error}"))?;
    write_epub_document(temporary.as_file_mut(), title, sections, options, metadata)?;
    persist_temporary(temporary, &target, "EPUB")
}

fn persist_bytes(target: &Path, bytes: &[u8], label: &str) -> Result<String, String> {
    let parent = target
        .parent()
        .ok_or_else(|| "The export destination has no parent folder.".to_string())?;
    let mut temporary = NamedTempFile::new_in(parent)
        .map_err(|error| format!("Cannot create the temporary {label}: {error}"))?;
    temporary
        .write_all(bytes)
        .map_err(|error| format!("Cannot write the {label}: {error}"))?;
    persist_temporary(temporary, target, label)
}

fn persist_temporary(
    mut temporary: NamedTempFile,
    target: &Path,
    label: &str,
) -> Result<String, String> {
    let parent = target
        .parent()
        .ok_or_else(|| "The export destination has no parent folder.".to_string())?;
    if let Ok(metadata) = fs::metadata(target) {
        temporary
            .as_file_mut()
            .set_permissions(metadata.permissions())
            .map_err(|error| {
                format!("Cannot preserve the existing {label} permissions: {error}")
            })?;
    }
    temporary
        .as_file_mut()
        .sync_all()
        .map_err(|error| format!("Cannot flush the {label}: {error}"))?;
    temporary
        .persist(target)
        .map_err(|error| format!("Cannot save the {label}: {}", error.error))?;

    #[cfg(unix)]
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("Cannot flush the export folder: {error}"))?;

    Ok(target.to_string_lossy().into_owned())
}

fn validate_export_destination(
    path: &str,
    extension: &str,
    label: &str,
) -> Result<PathBuf, String> {
    let target = PathBuf::from(path);
    if !target.is_absolute() {
        return Err(format!("Choose an absolute destination for the {label}."));
    }
    if !target
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
    {
        return Err(format!("The export destination must end in .{extension}."));
    }
    let parent = target
        .parent()
        .ok_or_else(|| "The export destination has no parent folder.".to_string())?;
    if !parent.is_dir() {
        return Err("The export destination folder does not exist.".into());
    }
    if target.is_dir() {
        return Err("The export destination is a folder, not a file.".into());
    }
    Ok(target)
}

fn validate_destination(path: &str) -> Result<PathBuf, String> {
    validate_export_destination(path, "docx", "Word document")
}

#[cfg(test)]
fn write_docx<W: Write + Seek>(writer: W, title: &str, markdown: &str) -> Result<(), String> {
    write_docx_document(
        writer,
        title,
        &[ExportSection {
            title: title.to_string(),
            markdown: markdown.to_string(),
        }],
        ExportOptions::default(),
        &ExportMetadata::default(),
    )
}

fn write_docx_document<W: Write + Seek>(
    writer: W,
    title: &str,
    sections: &[ExportSection],
    options: ExportOptions,
    metadata: &ExportMetadata,
) -> Result<(), String> {
    let title = normalized_title(title);
    let paragraphs = document_paragraphs(title, sections, options, &metadata.author)?;
    let document = document_xml(&paragraphs);
    let core_properties = core_properties_xml(title, metadata);
    let styles = styles_xml(&metadata.language);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(0o644);
    let mut archive = ZipWriter::new(writer);

    for (name, contents) in [
        ("[Content_Types].xml", CONTENT_TYPES),
        ("_rels/.rels", PACKAGE_RELATIONSHIPS),
        ("docProps/app.xml", APP_PROPERTIES),
        ("docProps/core.xml", core_properties.as_str()),
        ("word/document.xml", document.as_str()),
        ("word/styles.xml", styles.as_str()),
        ("word/_rels/document.xml.rels", DOCUMENT_RELATIONSHIPS),
    ] {
        archive
            .start_file(name, options)
            .map_err(|error| format!("Cannot add {name} to the Word document: {error}"))?;
        archive
            .write_all(contents.as_bytes())
            .map_err(|error| format!("Cannot write {name} to the Word document: {error}"))?;
    }

    archive
        .finish()
        .map_err(|error| format!("Cannot finish the Word document: {error}"))?;
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PdfFace {
    Regular,
    Bold,
    Italic,
    BoldItalic,
}

struct PdfFonts {
    regular: ParsedFont,
    bold: ParsedFont,
    italic: ParsedFont,
    bold_italic: ParsedFont,
    regular_handle: PdfFontHandle,
    bold_handle: PdfFontHandle,
    italic_handle: PdfFontHandle,
    bold_italic_handle: PdfFontHandle,
}

impl PdfFonts {
    fn load(document: &mut PdfDocument) -> Result<Self, String> {
        let regular = parse_pdf_font(SOURCE_SERIF_REGULAR, "regular")?;
        let bold = parse_pdf_font(SOURCE_SERIF_BOLD, "bold")?;
        let italic = parse_pdf_font(SOURCE_SERIF_ITALIC, "italic")?;
        let bold_italic = parse_pdf_font(SOURCE_SERIF_BOLD_ITALIC, "bold italic")?;
        let regular_handle = PdfFontHandle::External(document.add_font(&regular));
        let bold_handle = PdfFontHandle::External(document.add_font(&bold));
        let italic_handle = PdfFontHandle::External(document.add_font(&italic));
        let bold_italic_handle = PdfFontHandle::External(document.add_font(&bold_italic));
        Ok(Self {
            regular,
            bold,
            italic,
            bold_italic,
            regular_handle,
            bold_handle,
            italic_handle,
            bold_italic_handle,
        })
    }

    fn parsed(&self, face: PdfFace) -> &ParsedFont {
        match face {
            PdfFace::Regular => &self.regular,
            PdfFace::Bold => &self.bold,
            PdfFace::Italic => &self.italic,
            PdfFace::BoldItalic => &self.bold_italic,
        }
    }

    fn handle(&self, face: PdfFace) -> PdfFontHandle {
        match face {
            PdfFace::Regular => self.regular_handle.clone(),
            PdfFace::Bold => self.bold_handle.clone(),
            PdfFace::Italic => self.italic_handle.clone(),
            PdfFace::BoldItalic => self.bold_italic_handle.clone(),
        }
    }
}

#[derive(Clone, Debug)]
struct PdfCharacter {
    value: char,
    face: PdfFace,
}

#[derive(Clone, Copy)]
enum PdfAlignment {
    Left,
    Center,
}

#[derive(Clone, Copy)]
struct PdfParagraphStyle {
    size: f32,
    line_height: f32,
    before: f32,
    after: f32,
    left: f32,
    right: f32,
    first_indent: f32,
    alignment: PdfAlignment,
    bold: bool,
    italic: bool,
}

impl PdfParagraphStyle {
    fn for_paragraph(paragraph: &Paragraph) -> Self {
        match paragraph.kind {
            ParagraphKind::Title => Self {
                size: 16.0,
                line_height: 22.0,
                before: 0.0,
                after: 24.0,
                left: 0.0,
                right: 0.0,
                first_indent: 0.0,
                alignment: PdfAlignment::Center,
                bold: true,
                italic: false,
            },
            ParagraphKind::Byline => Self {
                size: 11.0,
                line_height: 18.0,
                before: 0.0,
                after: 18.0,
                left: 0.0,
                right: 0.0,
                first_indent: 0.0,
                alignment: PdfAlignment::Center,
                bold: false,
                italic: true,
            },
            ParagraphKind::Heading(1) => Self {
                size: 15.0,
                line_height: 21.0,
                before: 24.0,
                after: 12.0,
                left: 0.0,
                right: 0.0,
                first_indent: 0.0,
                alignment: PdfAlignment::Center,
                bold: true,
                italic: false,
            },
            ParagraphKind::Heading(2) => Self {
                size: 12.0,
                line_height: 18.0,
                before: 18.0,
                after: 6.0,
                left: 0.0,
                right: 0.0,
                first_indent: 0.0,
                alignment: PdfAlignment::Left,
                bold: true,
                italic: false,
            },
            ParagraphKind::Heading(_) => Self {
                size: 12.0,
                line_height: 18.0,
                before: 12.0,
                after: 3.0,
                left: 0.0,
                right: 0.0,
                first_indent: 0.0,
                alignment: PdfAlignment::Left,
                bold: true,
                italic: true,
            },
            ParagraphKind::Quote => Self {
                size: BODY_SIZE_PT,
                line_height: BODY_LINE_HEIGHT_PT,
                before: 0.0,
                after: 0.0,
                left: 36.0,
                right: 36.0,
                first_indent: 0.0,
                alignment: PdfAlignment::Left,
                bold: false,
                italic: true,
            },
            ParagraphKind::List { depth } => Self {
                size: BODY_SIZE_PT,
                line_height: BODY_LINE_HEIGHT_PT,
                before: 0.0,
                after: 0.0,
                left: 36.0 + depth.saturating_sub(1) as f32 * 18.0,
                right: 0.0,
                first_indent: 0.0,
                alignment: PdfAlignment::Left,
                bold: false,
                italic: false,
            },
            ParagraphKind::SceneBreak => Self {
                size: BODY_SIZE_PT,
                line_height: BODY_LINE_HEIGHT_PT,
                before: 12.0,
                after: 12.0,
                left: 0.0,
                right: 0.0,
                first_indent: 0.0,
                alignment: PdfAlignment::Center,
                bold: false,
                italic: false,
            },
            ParagraphKind::Code => Self {
                size: 10.0,
                line_height: 14.0,
                before: 6.0,
                after: 6.0,
                left: 36.0,
                right: 18.0,
                first_indent: 0.0,
                alignment: PdfAlignment::Left,
                bold: false,
                italic: false,
            },
            ParagraphKind::Body => Self {
                size: BODY_SIZE_PT,
                line_height: BODY_LINE_HEIGHT_PT,
                before: 0.0,
                after: 0.0,
                left: 0.0,
                right: 0.0,
                first_indent: 36.0,
                alignment: PdfAlignment::Left,
                bold: false,
                italic: false,
            },
        }
    }
}

struct PdfLayout<'a> {
    fonts: &'a PdfFonts,
    pages: Vec<Vec<Op>>,
    y: f32,
}

impl<'a> PdfLayout<'a> {
    fn new(fonts: &'a PdfFonts) -> Self {
        Self {
            fonts,
            pages: vec![Vec::new()],
            y: PAGE_HEIGHT_PT - PAGE_MARGIN_PT,
        }
    }

    fn new_page(&mut self) {
        self.pages.push(Vec::new());
        self.y = PAGE_HEIGHT_PT - PAGE_MARGIN_PT;
    }

    fn ensure_height(&mut self, height: f32) {
        if self.y - height < PAGE_MARGIN_PT {
            self.new_page();
        }
    }

    fn add_paragraph(&mut self, paragraph: &Paragraph) {
        if paragraph.page_break_before
            && self
                .pages
                .last()
                .is_some_and(|operations| !operations.is_empty())
        {
            self.new_page();
        }
        let style = PdfParagraphStyle::for_paragraph(paragraph);
        let characters = pdf_characters(paragraph, style);
        let available = PAGE_WIDTH_PT - PAGE_MARGIN_PT * 2.0 - style.left - style.right;
        let lines = wrap_pdf_characters(
            &characters,
            available,
            style.first_indent,
            style.size,
            self.fonts,
        );
        if lines.is_empty() {
            return;
        }

        self.ensure_height(style.before + style.line_height);
        self.y -= style.before;
        for (index, line) in lines.iter().enumerate() {
            self.ensure_height(style.line_height);
            let indent = if index == 0 { style.first_indent } else { 0.0 };
            let line_width = pdf_characters_width(line, style.size, self.fonts);
            let x = match style.alignment {
                PdfAlignment::Left => PAGE_MARGIN_PT + style.left + indent,
                PdfAlignment::Center => {
                    PAGE_MARGIN_PT + style.left + (available - line_width) / 2.0
                }
            };
            self.write_line(line, x, self.y, style.size);
            self.y -= style.line_height;
        }
        self.y -= style.after;
    }

    fn write_line(&mut self, line: &[PdfCharacter], mut x: f32, y: f32, size: f32) {
        let mut start = 0;
        while start < line.len() {
            let face = line[start].face;
            let mut end = start + 1;
            while end < line.len() && line[end].face == face {
                end += 1;
            }
            let text = line[start..end]
                .iter()
                .map(|character| character.value)
                .collect::<String>();
            let width = pdf_text_width(self.fonts.parsed(face), &text, size);
            self.pages.last_mut().expect("PDF page exists").extend([
                Op::StartTextSection,
                Op::SetTextCursor {
                    pos: Point { x: Pt(x), y: Pt(y) },
                },
                Op::SetFont {
                    font: self.fonts.handle(face),
                    size: Pt(size),
                },
                Op::ShowText {
                    items: vec![TextItem::Text(text)],
                },
                Op::EndTextSection,
            ]);
            x += width;
            start = end;
        }
    }
}

fn parse_pdf_font(bytes: &[u8], label: &str) -> Result<ParsedFont, String> {
    let mut warnings = Vec::new();
    ParsedFont::from_bytes(bytes, 0, &mut warnings)
        .ok_or_else(|| format!("Cannot load the bundled {label} export font."))
}

#[cfg(test)]
fn pdf_bytes(title: &str, markdown: &str) -> Result<Vec<u8>, String> {
    pdf_document_bytes(
        title,
        &[ExportSection {
            title: title.to_string(),
            markdown: markdown.to_string(),
        }],
        ExportOptions::default(),
        &ExportMetadata::default(),
    )
}

fn pdf_document_bytes(
    title: &str,
    sections: &[ExportSection],
    options: ExportOptions,
    metadata: &ExportMetadata,
) -> Result<Vec<u8>, String> {
    let title = normalized_title(title);
    let paragraphs = document_paragraphs(title, sections, options, &metadata.author)?;
    let mut document = PdfDocument::new(title);
    let now = DateTime::now();
    document.metadata.info.creation_date = now;
    document.metadata.info.modification_date = now;
    document.metadata.info.metadata_date = now;
    document.metadata.info.creator = "Writing Environment".into();
    document.metadata.info.producer = "Writing Environment".into();
    document.metadata.info.author = normalized_author(&metadata.author).to_string();
    let language = normalized_language(&metadata.language);
    if language != "und" {
        document.metadata.info.subject = format!("Language: {language}");
    }
    document.metadata.info.identifier = Uuid::new_v4().to_string();
    let fonts = PdfFonts::load(&mut document)?;
    let mut layout = PdfLayout::new(&fonts);
    for paragraph in &paragraphs {
        layout.add_paragraph(paragraph);
    }
    let pages = layout
        .pages
        .into_iter()
        .map(|operations| PdfPage::new(Mm(LETTER_WIDTH_MM), Mm(LETTER_HEIGHT_MM), operations))
        .collect();
    let mut warnings = Vec::new();
    let bytes = document
        .with_pages(pages)
        .save(&PdfSaveOptions::default(), &mut warnings);
    if bytes.is_empty() {
        return Err("The PDF renderer produced an empty document.".into());
    }
    Ok(bytes)
}

fn pdf_characters(paragraph: &Paragraph, paragraph_style: PdfParagraphStyle) -> Vec<PdfCharacter> {
    paragraph
        .runs
        .iter()
        .flat_map(|run| {
            let bold = paragraph_style.bold || run.style.bold > 0;
            let italic = paragraph_style.italic || run.style.italic > 0;
            let face = match (bold, italic) {
                (false, false) => PdfFace::Regular,
                (true, false) => PdfFace::Bold,
                (false, true) => PdfFace::Italic,
                (true, true) => PdfFace::BoldItalic,
            };
            run.text
                .chars()
                .map(move |value| PdfCharacter { value, face })
        })
        .collect()
}

fn wrap_pdf_characters(
    characters: &[PdfCharacter],
    available: f32,
    first_indent: f32,
    size: f32,
    fonts: &PdfFonts,
) -> Vec<Vec<PdfCharacter>> {
    let mut lines = Vec::new();
    let mut start = 0;
    while start < characters.len() {
        while start < characters.len() && characters[start].value == ' ' {
            start += 1;
        }
        if start >= characters.len() {
            break;
        }
        let limit = available - if lines.is_empty() { first_indent } else { 0.0 };
        let mut width = 0.0;
        let mut index = start;
        let mut last_break = None;
        let mut forced_break = false;
        while index < characters.len() {
            let value = characters[index].value;
            if value == '\n' {
                forced_break = true;
                break;
            }
            let character_width =
                pdf_character_width(fonts.parsed(characters[index].face), value, size);
            if width + character_width > limit && index > start {
                break;
            }
            width += character_width;
            index += 1;
            if value.is_whitespace() {
                last_break = Some(index);
            }
            if width > limit {
                break;
            }
        }

        let mut end = index;
        if !forced_break && index < characters.len() && characters[index].value != '\n' {
            if let Some(break_at) = last_break.filter(|break_at| *break_at > start) {
                end = break_at;
            }
        }
        if end == start {
            end = (start + 1).min(characters.len());
        }
        let mut visible_end = end;
        while visible_end > start && characters[visible_end - 1].value.is_whitespace() {
            visible_end -= 1;
        }
        if visible_end > start {
            lines.push(characters[start..visible_end].to_vec());
        } else {
            lines.push(Vec::new());
        }
        start = end;
        if forced_break && start < characters.len() && characters[start].value == '\n' {
            start += 1;
        }
    }
    if lines.is_empty() && !characters.is_empty() {
        lines.push(characters.to_vec());
    }
    lines
}

fn pdf_characters_width(characters: &[PdfCharacter], size: f32, fonts: &PdfFonts) -> f32 {
    characters
        .iter()
        .map(|character| pdf_character_width(fonts.parsed(character.face), character.value, size))
        .sum()
}

fn pdf_text_width(font: &ParsedFont, text: &str, size: f32) -> f32 {
    text.chars()
        .map(|character| pdf_character_width(font, character, size))
        .sum()
}

fn pdf_character_width(font: &ParsedFont, character: char, size: f32) -> f32 {
    let units = font
        .codepoint_to_glyph
        .get(&(character as u32))
        .and_then(|glyph| font.glyph_widths.get(glyph))
        .copied()
        .unwrap_or(font.units_per_em / 2);
    units as f32 / font.units_per_em.max(1) as f32 * size
}

#[cfg(test)]
fn write_epub<W: Write + Seek>(writer: W, title: &str, markdown: &str) -> Result<(), String> {
    write_epub_document(
        writer,
        title,
        &[ExportSection {
            title: title.to_string(),
            markdown: markdown.to_string(),
        }],
        ExportOptions::default(),
        &ExportMetadata::default(),
    )
}

fn write_epub_document<W: Write + Seek>(
    writer: W,
    title: &str,
    sections: &[ExportSection],
    options: ExportOptions,
    metadata: &ExportMetadata,
) -> Result<(), String> {
    let title = normalized_title(title);
    if sections.is_empty() {
        return Err("Choose at least one sheet to export.".into());
    }
    let identifier = format!("urn:uuid:{}", Uuid::new_v4());
    let modified = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let chapters = sections
        .iter()
        .enumerate()
        .map(|(index, section)| {
            let section_title = normalized_title(&section.title);
            let paragraphs = section_paragraphs(section_title, &section.markdown, true);
            (
                format!("chapter-{}.xhtml", index + 1),
                section_title.to_string(),
                epub_chapter_xhtml(
                    section_title,
                    &paragraphs,
                    normalized_language(&metadata.language),
                ),
            )
        })
        .collect::<Vec<_>>();
    let navigation = epub_navigation_xhtml(
        title,
        &chapters,
        options.title_page,
        normalized_language(&metadata.language),
    );
    let package = epub_package_xml(
        title,
        &identifier,
        &modified,
        &chapters,
        options.title_page,
        metadata,
    );
    let zip_options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(0o644);
    let mut archive = ZipWriter::new(writer);

    archive
        .start_file("mimetype", zip_options)
        .map_err(|error| format!("Cannot start the EPUB package: {error}"))?;
    archive
        .write_all(b"application/epub+zip")
        .map_err(|error| format!("Cannot write the EPUB media type: {error}"))?;

    for (name, contents) in [
        ("META-INF/container.xml", EPUB_CONTAINER),
        ("OEBPS/content.opf", package.as_str()),
        ("OEBPS/nav.xhtml", navigation.as_str()),
        ("OEBPS/styles/book.css", EPUB_CSS),
    ] {
        archive
            .start_file(name, zip_options)
            .map_err(|error| format!("Cannot add {name} to the EPUB: {error}"))?;
        archive
            .write_all(contents.as_bytes())
            .map_err(|error| format!("Cannot write {name} to the EPUB: {error}"))?;
    }
    if options.title_page {
        let title_page = epub_title_page_xhtml(title, metadata);
        archive
            .start_file("OEBPS/text/title.xhtml", zip_options)
            .map_err(|error| format!("Cannot add the title page to the EPUB: {error}"))?;
        archive
            .write_all(title_page.as_bytes())
            .map_err(|error| format!("Cannot write the EPUB title page: {error}"))?;
    }
    for (file_name, _, contents) in &chapters {
        let path = format!("OEBPS/text/{file_name}");
        archive
            .start_file(&path, zip_options)
            .map_err(|error| format!("Cannot add {path} to the EPUB: {error}"))?;
        archive
            .write_all(contents.as_bytes())
            .map_err(|error| format!("Cannot write {path} to the EPUB: {error}"))?;
    }
    archive
        .finish()
        .map_err(|error| format!("Cannot finish the EPUB: {error}"))?;
    Ok(())
}

const EPUB_CONTAINER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;

const EPUB_CSS: &str = r#"html { color: #1d1b19; background: #fff; }
body { max-width: 38em; margin: 0 auto; padding: 5%; font-family: serif; font-size: 1em; line-height: 1.55; }
h1.title { margin: 1.5em 0 2em; text-align: center; font-size: 1.55em; }
p.byline { margin: -1.25em 0 2em; text-align: center; text-indent: 0; font-style: italic; }
h1 { margin: 2em 0 1em; text-align: center; font-size: 1.45em; }
h2 { margin: 1.8em 0 0.65em; font-size: 1.2em; }
h3, h4, h5, h6 { margin: 1.5em 0 0.5em; font-size: 1em; font-style: italic; }
p { margin: 0; text-indent: 1.5em; }
blockquote { margin: 1em 2em; font-style: italic; }
blockquote p, p.list { text-indent: 0; }
p.list { margin-left: 1.5em; }
.scene-break { margin: 1.25em 0; text-align: center; letter-spacing: 0.3em; }
pre { margin: 1em 2em; white-space: pre-wrap; font-family: monospace; font-size: 0.9em; line-height: 1.35; }
code { font-family: monospace; }
nav ol { padding-left: 1.5em; }
a { color: inherit; }"#;

fn epub_package_xml(
    title: &str,
    identifier: &str,
    modified: &str,
    chapters: &[(String, String, String)],
    title_page: bool,
    metadata: &ExportMetadata,
) -> String {
    let title_item = if title_page {
        "    <item id=\"title-page\" href=\"text/title.xhtml\" media-type=\"application/xhtml+xml\"/>\n"
    } else {
        ""
    };
    let chapter_items = chapters
        .iter()
        .enumerate()
        .map(|(index, (file_name, _, _))| {
            format!(
                "    <item id=\"chapter-{}\" href=\"text/{}\" media-type=\"application/xhtml+xml\"/>\n",
                index + 1,
                xml_escape(file_name)
            )
        })
        .collect::<String>();
    let title_spine = if title_page {
        "    <itemref idref=\"title-page\"/>\n"
    } else {
        ""
    };
    let chapter_spine = chapters
        .iter()
        .enumerate()
        .map(|(index, _)| format!("    <itemref idref=\"chapter-{}\"/>\n", index + 1))
        .collect::<String>();
    let author = normalized_author(&metadata.author);
    let creator = if author.is_empty() {
        String::new()
    } else {
        format!("    <dc:creator>{}</dc:creator>\n", xml_escape(author))
    };
    let language = normalized_language(&metadata.language);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="book-id" xml:lang="{language}">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="book-id">{identifier}</dc:identifier>
    <dc:title>{title}</dc:title>
    <dc:language>{language}</dc:language>
{creator}    <meta property="dcterms:modified">{modified}</meta>
  </metadata>
  <manifest>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
    <item id="style" href="styles/book.css" media-type="text/css"/>
{title_item}{chapter_items}  </manifest>
  <spine>
{title_spine}{chapter_spine}  </spine>
</package>"#,
        identifier = xml_escape(identifier),
        title = xml_escape(title),
        modified = xml_escape(modified),
        language = xml_escape(language),
        creator = creator,
        title_item = title_item,
        chapter_items = chapter_items,
        title_spine = title_spine,
        chapter_spine = chapter_spine,
    )
}

fn epub_navigation_xhtml(
    title: &str,
    chapters: &[(String, String, String)],
    title_page: bool,
    language: &str,
) -> String {
    let mut entries = String::new();
    if title_page {
        entries.push_str(&format!(
            r#"<li><a href="text/title.xhtml">{}</a></li>"#,
            xml_escape(title)
        ));
    }
    for (file_name, chapter_title, _) in chapters {
        entries.push_str(&format!(
            r#"<li><a href="text/{}">{}</a></li>"#,
            xml_escape(file_name),
            xml_escape(chapter_title)
        ));
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" xml:lang="{language}" lang="{language}">
<head><title>Contents</title><link rel="stylesheet" type="text/css" href="styles/book.css"/></head>
<body><nav epub:type="toc" id="toc"><h1>Contents</h1><ol>{entries}</ol></nav></body>
</html>"#,
        entries = entries,
        language = xml_escape(language)
    )
}

fn epub_title_page_xhtml(title: &str, metadata: &ExportMetadata) -> String {
    let author = normalized_author(&metadata.author);
    let byline = if author.is_empty() {
        String::new()
    } else {
        format!(r#"<p class="byline">{}</p>"#, xml_escape(author))
    };
    let language = normalized_language(&metadata.language);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" xml:lang="{language}" lang="{language}">
<head><title>{title}</title><link rel="stylesheet" type="text/css" href="../styles/book.css"/></head>
<body><section epub:type="titlepage"><h1 class="title">{title}</h1>{byline}</section></body>
</html>"#,
        title = xml_escape(title),
        language = xml_escape(language),
        byline = byline,
    )
}

fn epub_chapter_xhtml(title: &str, paragraphs: &[Paragraph], language: &str) -> String {
    let mut body = String::new();
    for paragraph in paragraphs {
        body.push_str(&epub_paragraph_xhtml(paragraph));
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" xml:lang="{language}" lang="{language}">
<head><title>{title}</title><link rel="stylesheet" type="text/css" href="../styles/book.css"/></head>
<body><section epub:type="chapter">{body}</section></body>
</html>"#,
        title = xml_escape(title),
        language = xml_escape(language)
    )
}

fn epub_paragraph_xhtml(paragraph: &Paragraph) -> String {
    let contents = paragraph
        .runs
        .iter()
        .map(epub_run_xhtml)
        .collect::<String>();
    match paragraph.kind {
        ParagraphKind::Title => format!(r#"<h1 class="title">{contents}</h1>"#),
        ParagraphKind::Byline => format!(r#"<p class="byline">{contents}</p>"#),
        ParagraphKind::Heading(level) => {
            let level = level.clamp(1, 6);
            format!("<h{level}>{contents}</h{level}>")
        }
        ParagraphKind::Quote => format!("<blockquote><p>{contents}</p></blockquote>"),
        ParagraphKind::List { depth } => {
            format!(r#"<p class="list depth-{depth}">{contents}</p>"#)
        }
        ParagraphKind::SceneBreak => {
            format!(r#"<div class="scene-break" aria-label="Scene break">{contents}</div>"#)
        }
        ParagraphKind::Code => format!(
            "<pre><code>{}</code></pre>",
            epub_text(&paragraph.plain_text())
        ),
        ParagraphKind::Body => format!("<p>{contents}</p>"),
    }
}

fn epub_run_xhtml(run: &Run) -> String {
    let mut contents = epub_text(&run.text);
    if run.style.code {
        contents = format!("<code>{contents}</code>");
    }
    if run.style.italic > 0 {
        contents = format!("<em>{contents}</em>");
    }
    if run.style.bold > 0 {
        contents = format!("<strong>{contents}</strong>");
    }
    if run.style.strike > 0 {
        contents = format!("<del>{contents}</del>");
    }
    contents
}

fn epub_text(value: &str) -> String {
    xml_escape(value).replace('\n', "<br />")
}

fn normalized_title(title: &str) -> &str {
    let title = title.trim();
    if title.is_empty() {
        "Untitled"
    } else {
        title
    }
}

fn manuscript_paragraphs(title: &str, markdown: &str) -> Vec<Paragraph> {
    section_paragraphs(title, markdown, true)
}

fn document_paragraphs(
    title: &str,
    sections: &[ExportSection],
    options: ExportOptions,
    author: &str,
) -> Result<Vec<Paragraph>, String> {
    if sections.is_empty() {
        return Err("Choose at least one sheet to export.".into());
    }
    if sections.len() == 1 && !options.title_page {
        return Ok(manuscript_paragraphs(
            normalized_title(&sections[0].title),
            &sections[0].markdown,
        ));
    }

    let mut paragraphs = Vec::new();
    if options.title_page {
        let mut title_paragraph = Paragraph::new(ParagraphKind::Title);
        title_paragraph.push(normalized_title(title), InlineStyle::default());
        paragraphs.push(title_paragraph);
        let author = normalized_author(author);
        if !author.is_empty() {
            let mut byline = Paragraph::new(ParagraphKind::Byline);
            byline.push(author, InlineStyle::default());
            paragraphs.push(byline);
        }
    }
    for (index, section) in sections.iter().enumerate() {
        let section_title = normalized_title(&section.title);
        let mut section_paragraphs = section_paragraphs(section_title, &section.markdown, false);
        let mut heading = Paragraph::new(ParagraphKind::Heading(1));
        heading.page_break_before = options.title_page || (options.page_breaks && index > 0);
        heading.push(section_title, InlineStyle::default());
        paragraphs.push(heading);
        paragraphs.append(&mut section_paragraphs);
    }
    Ok(paragraphs)
}

fn section_paragraphs(title: &str, markdown: &str, include_title: bool) -> Vec<Paragraph> {
    let mut builder = MarkdownBuilder::default();
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    for event in Parser::new_ext(markdown_body(markdown), options) {
        match event {
            Event::Start(Tag::Paragraph) => builder.start(builder.inferred_kind()),
            Event::End(TagEnd::Paragraph) => builder.finish(),
            Event::Start(Tag::Heading { level, .. }) => {
                builder.start(ParagraphKind::Heading(heading_number(level)))
            }
            Event::End(TagEnd::Heading(_)) => builder.finish(),
            Event::Start(Tag::BlockQuote(_)) => builder.quote_depth += 1,
            Event::End(TagEnd::BlockQuote(_)) => {
                builder.finish();
                builder.quote_depth = builder.quote_depth.saturating_sub(1);
            }
            Event::Start(Tag::CodeBlock(_)) => {
                builder.start(ParagraphKind::Code);
                builder.inline.code = true;
            }
            Event::End(TagEnd::CodeBlock) => {
                builder.inline.code = false;
                builder.finish();
            }
            Event::Start(Tag::List(start)) => builder.lists.push(ListState {
                ordered: start.is_some(),
                next: start.unwrap_or(1),
            }),
            Event::End(TagEnd::List(_)) => {
                builder.finish();
                builder.lists.pop();
            }
            Event::Start(Tag::Item) => builder.start_item(),
            Event::End(TagEnd::Item) => {
                builder.finish();
                builder.in_item = false;
                builder.item_marker = None;
            }
            Event::Start(Tag::Emphasis) => builder.inline.italic += 1,
            Event::End(TagEnd::Emphasis) => {
                builder.inline.italic = builder.inline.italic.saturating_sub(1)
            }
            Event::Start(Tag::Strong) => builder.inline.bold += 1,
            Event::End(TagEnd::Strong) => {
                builder.inline.bold = builder.inline.bold.saturating_sub(1)
            }
            Event::Start(Tag::Strikethrough) => builder.inline.strike += 1,
            Event::End(TagEnd::Strikethrough) => {
                builder.inline.strike = builder.inline.strike.saturating_sub(1)
            }
            Event::Text(text) => builder.push(&text),
            Event::Code(text) => {
                let mut style = builder.inline;
                style.code = true;
                builder.push_with_style(&text, style);
            }
            Event::SoftBreak => builder.push(" "),
            Event::HardBreak => builder.push("\n"),
            Event::Rule => {
                builder.finish();
                let mut paragraph = Paragraph::new(ParagraphKind::SceneBreak);
                paragraph.push("* * *", InlineStyle::default());
                builder.paragraphs.push(paragraph);
            }
            Event::TaskListMarker(checked) => builder.push(if checked { "☒ " } else { "☐ " }),
            Event::InlineMath(text) | Event::DisplayMath(text) => {
                let mut style = builder.inline;
                style.code = true;
                builder.push_with_style(&text, style);
            }
            Event::FootnoteReference(label) => builder.push(&format!("[{label}]")),
            Event::Html(_) | Event::InlineHtml(_) => {}
            Event::Start(_) | Event::End(_) => {}
        }
    }
    builder.finish();

    if builder.paragraphs.first().is_some_and(|paragraph| {
        matches!(paragraph.kind, ParagraphKind::Heading(1))
            && paragraph.plain_text().trim().eq_ignore_ascii_case(title)
    }) {
        builder.paragraphs.remove(0);
    }

    if include_title {
        let mut title_paragraph = Paragraph::new(ParagraphKind::Title);
        title_paragraph.push(title, InlineStyle::default());
        builder.paragraphs.insert(0, title_paragraph);
    }
    builder.paragraphs
}

fn markdown_body(content: &str) -> &str {
    if !content.starts_with("---\n") && !content.starts_with("---\r\n") {
        return content;
    }
    let mut offset = 0;
    for (index, line) in content.split_inclusive('\n').enumerate() {
        offset += line.len();
        if index > 0 && line.trim() == "---" {
            return &content[offset..];
        }
    }
    content
}

fn heading_number(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn document_xml(paragraphs: &[Paragraph]) -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>"#,
    );
    for paragraph in paragraphs {
        xml.push_str(&paragraph_xml(paragraph));
    }
    xml.push_str(r#"<w:sectPr><w:pgSz w:w="12240" w:h="15840"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/></w:sectPr></w:body></w:document>"#);
    xml
}

fn paragraph_xml(paragraph: &Paragraph) -> String {
    let (style, extra_properties) = match paragraph.kind {
        ParagraphKind::Body => ("BodyText", String::new()),
        ParagraphKind::Title => ("Title", String::new()),
        ParagraphKind::Byline => ("Byline", String::new()),
        ParagraphKind::Heading(1) => ("Heading1", String::new()),
        ParagraphKind::Heading(2) => ("Heading2", String::new()),
        ParagraphKind::Heading(_) => ("Heading3", String::new()),
        ParagraphKind::Quote => ("Quotation", String::new()),
        ParagraphKind::List { depth } => {
            let left = 720 + depth.saturating_sub(1) * 360;
            (
                "ListParagraph",
                format!(r#"<w:ind w:left="{left}" w:hanging="360"/>"#),
            )
        }
        ParagraphKind::SceneBreak => ("SceneBreak", String::new()),
        ParagraphKind::Code => ("CodeBlock", String::new()),
    };
    let page_break = if paragraph.page_break_before {
        "<w:pageBreakBefore/>"
    } else {
        ""
    };
    let mut xml =
        format!(r#"<w:p><w:pPr><w:pStyle w:val="{style}"/>{page_break}{extra_properties}</w:pPr>"#);
    for run in &paragraph.runs {
        xml.push_str(&run_xml(run));
    }
    xml.push_str("</w:p>");
    xml
}

fn run_xml(run: &Run) -> String {
    let mut properties = String::new();
    if run.style.bold > 0 {
        properties.push_str("<w:b/>");
    }
    if run.style.italic > 0 {
        properties.push_str("<w:i/>");
    }
    if run.style.strike > 0 {
        properties.push_str("<w:strike/>");
    }
    if run.style.code {
        properties.push_str(r#"<w:rStyle w:val="CodeChar"/>"#);
    }

    let mut xml = String::from("<w:r>");
    if !properties.is_empty() {
        xml.push_str("<w:rPr>");
        xml.push_str(&properties);
        xml.push_str("</w:rPr>");
    }
    let parts = run.text.split('\n').collect::<Vec<_>>();
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            xml.push_str("<w:br/>");
        }
        if !part.is_empty() {
            xml.push_str(r#"<w:t xml:space="preserve">"#);
            xml.push_str(&xml_escape(part));
            xml.push_str("</w:t>");
        }
    }
    xml.push_str("</w:r>");
    xml
}

fn core_properties_xml(title: &str, metadata: &ExportMetadata) -> String {
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let author = normalized_author(&metadata.author);
    let language = normalized_language(&metadata.language);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:title>{title}</dc:title><dc:creator>{author}</dc:creator><dc:language>{language}</dc:language><cp:lastModifiedBy>Writing Environment</cp:lastModifiedBy><dcterms:created xsi:type="dcterms:W3CDTF">{now}</dcterms:created><dcterms:modified xsi:type="dcterms:W3CDTF">{now}</dcterms:modified></cp:coreProperties>"#,
        title = xml_escape(title),
        author = xml_escape(author),
        language = xml_escape(language),
    )
}

fn styles_xml(language: &str) -> String {
    let language = normalized_language(language);
    STYLES.replacen(
        "<w:rPrDefault><w:rPr>",
        &format!(
            r#"<w:rPrDefault><w:rPr><w:lang w:val="{}"/>"#,
            xml_escape(language)
        ),
        1,
    )
}

fn normalized_author(author: &str) -> &str {
    author.trim()
}

fn normalized_language(language: &str) -> &str {
    let language = language.trim();
    if !language.is_empty()
        && language.len() <= 35
        && language
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        language
    } else {
        "und"
    }
}

fn xml_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            '\t' => escaped.push_str("&#9;"),
            character if character >= ' ' => escaped.push(character),
            _ => {}
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;
    use printpdf::PdfParseOptions;
    use std::io::{Cursor, Read};
    use zip::ZipArchive;

    fn package(markdown: &str) -> Vec<u8> {
        let mut cursor = Cursor::new(Vec::new());
        write_docx(&mut cursor, "A & B", markdown).unwrap();
        cursor.into_inner()
    }

    fn part(bytes: &[u8], name: &str) -> String {
        let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut contents = String::new();
        archive
            .by_name(name)
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();
        contents
    }

    fn epub_package(markdown: &str) -> Vec<u8> {
        let mut cursor = Cursor::new(Vec::new());
        write_epub(&mut cursor, "Café & Rain", markdown).unwrap();
        cursor.into_inner()
    }

    #[test]
    fn docx_contains_the_required_word_package_parts() {
        let bytes = package("A paragraph.");
        let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
        for name in [
            "[Content_Types].xml",
            "_rels/.rels",
            "docProps/app.xml",
            "docProps/core.xml",
            "word/document.xml",
            "word/styles.xml",
            "word/_rels/document.xml.rels",
        ] {
            assert!(archive.by_name(name).is_ok(), "missing {name}");
        }
    }

    #[test]
    fn markdown_structure_becomes_word_styles_and_runs() {
        let bytes = package(
            "---\ntitle: Hidden metadata\n---\n# A & B\n\n## Chapter One\n\nPlain *italic* and **bold** and `code`.\n\n> Quoted.\n\n1. First\n2. Second\n\n***",
        );
        let document = part(&bytes, "word/document.xml");
        assert_eq!(document.matches("w:val=\"Title\"").count(), 1);
        assert!(!document.contains("Hidden metadata"));
        assert!(document.contains("w:val=\"Heading2\""));
        assert!(document.contains("<w:i/>"));
        assert!(document.contains("<w:b/>"));
        assert!(document.contains("w:val=\"CodeChar\""));
        assert!(document.contains("w:val=\"Quotation\""));
        assert_eq!(document.matches("w:val=\"ListParagraph\"").count(), 2);
        assert!(document.contains("1. "));
        assert!(document.contains("2. "));
        assert!(document.contains("w:val=\"SceneBreak\""));
        assert!(document.contains("* * *"));
    }

    #[test]
    fn a_matching_opening_heading_is_not_duplicated() {
        let paragraphs = manuscript_paragraphs("The Arrival", "# The Arrival\n\nRain.");
        assert_eq!(paragraphs.len(), 2);
        assert_eq!(paragraphs[0].kind, ParagraphKind::Title);
        assert_eq!(paragraphs[1].kind, ParagraphKind::Body);
    }

    #[test]
    fn word_xml_escapes_text_and_omits_invalid_controls() {
        let bytes = package("Five < six & seven.\u{0007}");
        let document = part(&bytes, "word/document.xml");
        let core = part(&bytes, "docProps/core.xml");
        assert!(document.contains("Five &lt; six &amp; seven."));
        assert!(!document.contains('\u{0007}'));
        assert!(core.contains("A &amp; B"));
    }

    #[test]
    fn export_requires_an_absolute_docx_destination() {
        assert!(validate_destination("draft.docx").is_err());
        assert!(validate_destination("/tmp/draft.pdf").is_err());
    }

    #[test]
    fn document_language_tags_are_sanitized() {
        assert_eq!(normalized_language("pt-BR"), "pt-BR");
        assert_eq!(normalized_language("en-US"), "en-US");
        assert_eq!(normalized_language("en_US"), "und");
        assert_eq!(normalized_language("\"/><script>"), "und");
    }

    #[test]
    fn export_atomically_creates_and_replaces_a_docx() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("Draft.docx");
        let target_string = target.to_string_lossy().into_owned();
        export_sheet_docx(&target_string, "Draft", "First version.").unwrap();
        export_sheet_docx(&target_string, "Draft", "Second version.").unwrap();
        let bytes = fs::read(target).unwrap();
        assert!(part(&bytes, "word/document.xml").contains("Second version."));
    }

    #[test]
    fn pdf_uses_embedded_fonts_and_paginates_long_manuscripts() {
        let markdown = format!("## Chapter One\n\n{}", "Café, ação, and rain. ".repeat(900));
        let bytes = pdf_bytes("The Arrival", &markdown).unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
        let parsed = PdfDocument::parse(&bytes, &PdfParseOptions::default(), &mut Vec::new())
            .expect("generated PDF should parse");
        assert!(parsed.pages.len() > 1);
        assert_eq!(parsed.metadata.info.document_title, "The Arrival");
        assert!(parsed.resources.fonts.map.len() >= 2);
    }

    #[test]
    fn epub_has_required_epub_three_parts_and_semantic_markdown() {
        let bytes = epub_package(
            "---\ntitle: Hidden\n---\n# Café & Rain\n\n## Chapter One\n\nPlain *italic* and **bold**.\n\n> Quoted.\n\n***",
        );
        let mut archive = ZipArchive::new(Cursor::new(&bytes)).unwrap();
        let mut media_type = String::new();
        let first_name = archive.by_index(0).unwrap().name().to_string();
        archive
            .by_name("mimetype")
            .unwrap()
            .read_to_string(&mut media_type)
            .unwrap();
        assert_eq!(first_name, "mimetype");
        assert_eq!(media_type, "application/epub+zip");
        for name in [
            "META-INF/container.xml",
            "OEBPS/content.opf",
            "OEBPS/nav.xhtml",
            "OEBPS/text/chapter-1.xhtml",
            "OEBPS/styles/book.css",
        ] {
            assert!(archive.by_name(name).is_ok(), "missing {name}");
        }
        drop(archive);

        let chapter = part(&bytes, "OEBPS/text/chapter-1.xhtml");
        assert_eq!(chapter.matches("class=\"title\"").count(), 1);
        assert!(!chapter.contains("Hidden"));
        assert!(chapter.contains("<h2>Chapter One</h2>"));
        assert!(chapter.contains("<em>italic</em>"));
        assert!(chapter.contains("<strong>bold</strong>"));
        assert!(chapter.contains("<blockquote>"));
        assert!(chapter.contains("class=\"scene-break\""));

        let package = part(&bytes, "OEBPS/content.opf");
        assert!(package.contains("version=\"3.0\""));
        assert!(package.contains("Café &amp; Rain"));
        assert!(package.contains("properties=\"nav\""));
    }

    #[test]
    fn pdf_and_epub_exports_validate_extensions_and_replace_atomically() {
        let directory = tempfile::tempdir().unwrap();
        let pdf_target = directory.path().join("Draft.pdf");
        let epub_target = directory.path().join("Draft.epub");
        let pdf_path = pdf_target.to_string_lossy();
        let epub_path = epub_target.to_string_lossy();

        assert!(validate_export_destination("draft.pdf", "pdf", "PDF").is_err());
        assert!(validate_export_destination(&pdf_path, "epub", "EPUB").is_err());
        export_sheet_pdf(&pdf_path, "Draft", "First version.").unwrap();
        export_sheet_pdf(&pdf_path, "Draft", "Second version.").unwrap();
        export_sheet_epub(&epub_path, "Draft", "First version.").unwrap();
        export_sheet_epub(&epub_path, "Draft", "Second version.").unwrap();

        assert!(fs::read(&pdf_target).unwrap().starts_with(b"%PDF-"));
        assert!(part(
            &fs::read(&epub_target).unwrap(),
            "OEBPS/text/chapter-1.xhtml"
        )
        .contains("Second version."));
    }

    #[test]
    fn assembled_documents_preserve_order_titles_and_page_breaks() {
        let sections = vec![
            ExportSection {
                title: "Opening".into(),
                markdown: "# Opening\n\nFirst sheet.".into(),
            },
            ExportSection {
                title: "Afterward".into(),
                markdown: "# Afterward\n\nSecond sheet.".into(),
            },
        ];
        let options = ExportOptions {
            title_page: true,
            page_breaks: true,
        };
        let metadata = ExportMetadata {
            author: "Thiago Author".into(),
            language: "pt-BR".into(),
        };

        let mut docx = Cursor::new(Vec::new());
        write_docx_document(&mut docx, "Collected Work", &sections, options, &metadata).unwrap();
        let docx_bytes = docx.into_inner();
        let document = part(&docx_bytes, "word/document.xml");
        let core = part(&docx_bytes, "docProps/core.xml");
        let styles = part(&docx_bytes, "word/styles.xml");
        assert!(document.find("Collected Work").unwrap() < document.find("Opening").unwrap());
        assert!(document.find("Opening").unwrap() < document.find("Afterward").unwrap());
        assert_eq!(document.matches("<w:pageBreakBefore/>").count(), 2);
        assert_eq!(document.matches("First sheet.").count(), 1);
        assert!(document.contains("w:val=\"Byline\""));
        assert!(core.contains("<dc:creator>Thiago Author</dc:creator>"));
        assert!(core.contains("<dc:language>pt-BR</dc:language>"));
        assert!(styles.contains("<w:lang w:val=\"pt-BR\"/>"));

        let pdf = pdf_document_bytes("Collected Work", &sections, options, &metadata).unwrap();
        let parsed = PdfDocument::parse(&pdf, &PdfParseOptions::default(), &mut Vec::new())
            .expect("assembled PDF should parse");
        assert_eq!(parsed.pages.len(), 3);
        assert_eq!(parsed.metadata.info.author, "Thiago Author");
        assert_eq!(parsed.metadata.info.subject, "Language: pt-BR");

        let mut epub = Cursor::new(Vec::new());
        write_epub_document(&mut epub, "Collected Work", &sections, options, &metadata).unwrap();
        let bytes = epub.into_inner();
        let package = part(&bytes, "OEBPS/content.opf");
        let navigation = part(&bytes, "OEBPS/nav.xhtml");
        assert!(package.contains("text/title.xhtml"));
        assert!(package.contains("text/chapter-1.xhtml"));
        assert!(package.contains("text/chapter-2.xhtml"));
        assert!(package.contains("<dc:creator>Thiago Author</dc:creator>"));
        assert!(package.contains("<dc:language>pt-BR</dc:language>"));
        assert!(navigation.find("Opening").unwrap() < navigation.find("Afterward").unwrap());
        assert!(part(&bytes, "OEBPS/text/title.xhtml").contains("Thiago Author"));
        assert!(part(&bytes, "OEBPS/text/chapter-1.xhtml").contains("First sheet."));
        assert!(part(&bytes, "OEBPS/text/chapter-2.xhtml").contains("Second sheet."));
    }

    #[test]
    #[ignore = "writes manual export fixtures to WRITING_ENVIRONMENT_EXPORT_FIXTURE_DIR"]
    fn write_manual_export_fixtures() {
        let directory = std::env::var("WRITING_ENVIRONMENT_EXPORT_FIXTURE_DIR")
            .expect("set WRITING_ENVIRONMENT_EXPORT_FIXTURE_DIR");
        let markdown = r#"---
title: Hidden metadata
---
# The Arrival

## Chapter One

The rain arrived before anyone expected it. Café windows shone across the avenue, and Mara **waited** beneath the old awning.

This paragraph is long enough to demonstrate the manuscript measure, first-line indent, and double-spaced rhythm in the fixed-layout export. It also includes *italic emphasis*, `inline code`, an em dash — and Portuguese text: ação, coração, manhã.

> We only need one clear signal, she said. Then we move.

1. Check the northern door
2. Call Elias
3. Leave before dawn

***

### A quieter heading

The water climbed another inch.
"#;
        let sections = vec![
            ExportSection { title: "The Arrival".into(), markdown: markdown.into() },
            ExportSection {
                title: "A Light Offshore".into(),
                markdown: "# A Light Offshore\n\nThe light paused on the empty horizon, then swept back toward the harbor.\n\nA second paragraph proves that each sheet keeps a clear manuscript rhythm.".into(),
            },
            ExportSection {
                title: "The Empty Room".into(),
                markdown: "# The Empty Room\n\nBy morning, every photograph had been turned over.".into(),
            },
        ];
        let options = ExportOptions {
            title_page: true,
            page_breaks: true,
        };
        let metadata = ExportMetadata {
            author: "Thiago Author".into(),
            language: "pt-BR".into(),
        };
        export_document_docx(
            &Path::new(&directory)
                .join("Collected Work.docx")
                .to_string_lossy(),
            "Collected Work",
            &sections,
            options,
            &metadata,
        )
        .unwrap();
        export_document_pdf(
            &Path::new(&directory)
                .join("Collected Work.pdf")
                .to_string_lossy(),
            "Collected Work",
            &sections,
            options,
            &metadata,
        )
        .unwrap();
        export_document_epub(
            &Path::new(&directory)
                .join("Collected Work.epub")
                .to_string_lossy(),
            "Collected Work",
            &sections,
            options,
            &metadata,
        )
        .unwrap();
    }
}

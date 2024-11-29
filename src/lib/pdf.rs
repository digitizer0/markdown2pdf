//! PDF generation module for markdown-to-pdf conversion.
//!
//! This module handles the complete process of converting parsed markdown content into professionally formatted PDF documents.
//! It provides robust support for generating PDFs with proper typography, layout, and styling while maintaining the semantic
//! structure of the original markdown.
//!
//! The PDF generation process preserves the hierarchical document structure through careful handling of block-level and inline
//! elements. Block elements like headings, paragraphs, lists and code blocks are rendered with appropriate spacing and indentation.
//! Inline formatting such as emphasis, links and inline code maintain proper nesting and style inheritance.
//!
//! The styling system offers extensive customization options through a flexible configuration model. This includes control over:
//! fonts, text sizes, colors, margins, spacing, and special styling for different content types. The module automatically handles
//! font loading, page layout, and proper rendering of all markdown elements while respecting the configured styles.
//!
//! Error handling is built in throughout the generation process to provide meaningful feedback if issues occur during PDF creation.
//! The module is designed to be both robust for production use and flexible enough to accommodate various document structures
//! and styling needs.

use crate::{
    styling::{MdPdfFont, StyleMatch},
    Token,
};
use genpdfi::{
    fonts::{FontData, FontFamily},
    Document,
};

/// The main PDF document generator that orchestrates the conversion process from markdown to PDF.
/// This struct serves as the central coordinator for document generation, managing the overall
/// structure, styling application, and proper sequencing of content elements.
/// It stores the input markdown tokens that will be processed into PDF content, along with style
/// configuration that controls the visual appearance and layout of the generated document.
/// The generator maintains two separate font families - a main text font used for regular document
/// content and a specialized monospace font specifically for code sections.
/// These fonts are loaded based on the style configuration and stored internally for use during
/// the PDF generation process.
#[allow(dead_code)]
pub struct Pdf {
    input: Vec<Token>,
    style: StyleMatch,
    font_family: FontFamily<FontData>,
    code_font_family: FontFamily<FontData>,
}

impl Pdf {
    /// Creates a new PDF generator instance to process markdown tokens.
    /// The generator maintains document structure and applies styling/layout rules during conversion.
    ///
    /// It automatically loads two font families based on the style configuration:
    /// - A main text font for regular content
    /// - A code font specifically for code blocks and inline code segments
    ///
    /// Font loading is handled automatically but will panic if the specified fonts cannot be loaded
    /// successfully. The generator internally stores the input tokens, style configuration, and loaded
    /// font families for use during PDF generation.
    ///
    /// Through the style configuration, the generator controls all visual aspects of the output PDF
    /// including typography, dimensions, colors and spacing between elements. The style settings
    /// determine the complete visual appearance and layout characteristics of the final generated
    /// PDF document.
    pub fn new(input: Vec<Token>, style: StyleMatch) -> Self {
        let font_family = MdPdfFont::load_font_family(style.text.font_family)
            .expect("Failed to load font family");
        let code_font_family = MdPdfFont::load_font_family(style.code.font_family)
            .expect("Failed to load code font family");

        Self {
            input,
            style,
            font_family,
            code_font_family,
        }
    }

    /// Finalizes and outputs the processed document to a PDF file at the specified path.
    /// Provides comprehensive error handling to catch and report any issues during the
    /// final rendering phase.
    pub fn render(document: genpdfi::Document, path: &str) -> Option<String> {
        match document.render_to_file(path) {
            Ok(_) => None,
            Err(err) => Some(err.to_string()),
        }
    }

    /// Initializes and returns a new PDF document with configured styling and layout.
    ///
    /// Creates a new document instance with the main font family and configures the page decorator
    /// with margins from the style settings. The document's base font size is set according to the
    /// text style configuration.
    ///
    /// The function processes all input tokens and renders them into the document structure before
    /// returning the complete document ready for final output. The document contains all content
    /// with proper styling, formatting and layout applied according to the style configuration.
    ///
    /// Through the style configuration, this method controls the overall document appearance including:
    /// - Page margins and layout
    /// - Base font size
    /// - Content processing and rendering
    pub fn render_into_document(&self) -> Document {
        let mut doc = genpdfi::Document::new(self.font_family.clone());
        let mut decorator = genpdfi::SimplePageDecorator::new();

        decorator.set_margins(genpdfi::Margins::trbl(
            self.style.margins.top,
            self.style.margins.right,
            self.style.margins.bottom,
            self.style.margins.left,
        ));

        doc.set_page_decorator(decorator);
        doc.set_font_size(self.style.text.size);

        self.process_tokens(&mut doc);
        doc
    }

    /// Processes and renders tokens directly into the document structure.
    ///
    /// This method iterates through all input tokens and renders them into the document,
    /// handling each token type appropriately according to its semantic meaning. Block-level
    /// elements like headings, list items, and code blocks trigger the flushing of any
    /// accumulated inline tokens into paragraphs before being rendered themselves.
    ///
    /// The method maintains a buffer of current tokens that gets flushed into paragraphs
    /// when block-level elements are encountered or when explicit paragraph breaks are
    /// needed. This ensures proper document flow and maintains correct spacing between
    /// different content elements while preserving the intended document structure.
    ///
    /// Through careful token processing and rendering, this method builds up the complete
    /// document content with appropriate styling, formatting and layout applied according
    /// to the configured style settings.
    fn process_tokens(&self, doc: &mut Document) {
        let mut current_tokens = Vec::new();

        for token in &self.input {
            match token {
                Token::Heading(content, level) => {
                    self.flush_paragraph(doc, &current_tokens);
                    current_tokens.clear();
                    self.render_heading(doc, content, *level);
                }
                Token::ListItem {
                    content,
                    ordered,
                    number,
                } => {
                    self.flush_paragraph(doc, &current_tokens);
                    current_tokens.clear();
                    self.render_list_item(doc, content, *ordered, *number, 0);
                }
                Token::Code(lang, content) if content.contains('\n') => {
                    self.flush_paragraph(doc, &current_tokens);
                    current_tokens.clear();
                    self.render_code_block(doc, lang, content);
                }
                Token::HorizontalRule => {
                    self.flush_paragraph(doc, &current_tokens);
                    current_tokens.clear();
                    doc.push(genpdfi::elements::Break::new(
                        self.style.horizontal_rule.after_spacing,
                    ));
                }
                Token::Newline => {
                    self.flush_paragraph(doc, &current_tokens);
                    current_tokens.clear();
                }
                _ => {
                    current_tokens.push(token.clone());
                }
            }
        }

        // Flush any remaining tokens
        self.flush_paragraph(doc, &current_tokens);
    }

    /// Renders accumulated tokens as a paragraph in the document.
    ///
    /// This method takes a document and a slice of tokens, and renders them as a paragraph
    /// with appropriate styling. If the tokens slice is empty, no paragraph is rendered.
    /// After rendering the paragraph content, it adds spacing after the paragraph according
    /// to the configured text style.
    fn flush_paragraph(&self, doc: &mut Document, tokens: &[Token]) {
        if tokens.is_empty() {
            return;
        }

        let mut para = genpdfi::elements::Paragraph::default();
        self.render_inline_content(&mut para, tokens);
        doc.push(para);
        doc.push(genpdfi::elements::Break::new(self.style.text.after_spacing));
    }

    /// Renders a heading with the appropriate level styling.
    ///
    /// This method takes a document, heading content tokens, and a level number to render
    /// a heading with the corresponding style settings. It applies font size, bold/italic effects,
    /// and text color based on the heading level configuration. After rendering the heading,
    /// it adds the configured spacing.
    fn render_heading(&self, doc: &mut Document, content: &[Token], level: usize) {
        doc.push(genpdfi::elements::Break::new(0.8)); // TODO: make this configurable before_spacing
        let heading_style = match level {
            1 => &self.style.heading_1,
            2 => &self.style.heading_2,
            3 | _ => &self.style.heading_3,
        };

        let mut para = genpdfi::elements::Paragraph::default();
        let mut style = genpdfi::style::Style::new().with_font_size(heading_style.size);

        if heading_style.bold {
            style = style.bold();
        }
        if heading_style.italic {
            style = style.italic();
        }
        if let Some(color) = heading_style.text_color {
            style = style.with_color(genpdfi::style::Color::Rgb(color.0, color.1, color.2));
        }

        self.render_inline_content_with_style(&mut para, content, style);
        doc.push(para);
        doc.push(genpdfi::elements::Break::new(heading_style.after_spacing));
    }

    /// Renders inline content with a specified style.
    ///
    /// This method processes a sequence of inline tokens and renders them with the given style.
    /// It handles various inline elements like plain text, emphasis, strong emphasis, links, and
    /// inline code, applying appropriate styling modifications for each type while maintaining
    /// the base style properties.
    fn render_inline_content_with_style(
        &self,
        para: &mut genpdfi::elements::Paragraph,
        tokens: &[Token],
        style: genpdfi::style::Style,
    ) {
        for token in tokens {
            match token {
                Token::Text(content) => {
                    para.push_styled(content.clone(), style.clone());
                }
                Token::Emphasis { level, content } => {
                    let mut nested_style = style.clone();
                    match level {
                        1 => nested_style = nested_style.italic(),
                        2 => nested_style = nested_style.bold(),
                        _ => nested_style = nested_style.bold().italic(),
                    }
                    self.render_inline_content_with_style(para, content, nested_style);
                }
                Token::StrongEmphasis(content) => {
                    let nested_style = style.clone().bold();
                    self.render_inline_content_with_style(para, content, nested_style);
                }
                Token::Link(text, url) => {
                    let mut link_style = style.clone();
                    if let Some(color) = self.style.link.text_color {
                        link_style = link_style
                            .with_color(genpdfi::style::Color::Rgb(color.0, color.1, color.2));
                    }
                    para.push_link(text.clone(), url.clone(), link_style);
                }
                Token::Code(_, content) => {
                    let mut code_style = style.clone();
                    if let Some(color) = self.style.code.text_color {
                        code_style = code_style
                            .with_color(genpdfi::style::Color::Rgb(color.0, color.1, color.2));
                    }
                    para.push_styled(content.clone(), code_style);
                }
                _ => {}
            }
        }
    }

    /// Renders inline content with the default text style.
    ///
    /// This is a convenience method that wraps render_inline_content_with_style,
    /// using the default text style configuration. It applies the configured font size
    /// to the content before rendering.
    fn render_inline_content(&self, para: &mut genpdfi::elements::Paragraph, tokens: &[Token]) {
        let style = genpdfi::style::Style::new().with_font_size(self.style.text.size);
        self.render_inline_content_with_style(para, tokens, style);
    }

    /// Renders a code block with appropriate styling.
    ///
    /// This method handles multi-line code blocks, rendering each line as a separate
    /// paragraph with the configured code style. It applies the code font size and
    /// text color settings, and adds the configured spacing after the block.
    fn render_code_block(&self, doc: &mut Document, _lang: &str, content: &str) {
        doc.push(genpdfi::elements::Break::new(0.4)); // TODO: make this configurable `before_spacing`
        let mut style = genpdfi::style::Style::new().with_font_size(self.style.code.size);

        if let Some(color) = self.style.code.text_color {
            style = style.with_color(genpdfi::style::Color::Rgb(color.0, color.1, color.2));
        }

        let indent = "    "; // TODO: make this configurable from style match.
        for line in content.split('\n') {
            let mut para = genpdfi::elements::Paragraph::default();
            para.push_styled(format!("{}{}", indent, line), style.clone());
            doc.push(para);
        }
        doc.push(genpdfi::elements::Break::new(self.style.code.after_spacing));
    }

    /// Renders a list item with appropriate styling and formatting.
    ///
    /// This method handles both ordered and unordered list items, with support for nested lists.
    /// For ordered lists, it includes the item number prefixed with a period (like "1."), while
    /// unordered lists use a bullet point dash character. The content is rendered with the
    /// configured list item style settings from the document style configuration.
    ///
    /// The method processes both the direct content of the list item as well as any nested list
    /// items recursively. Each nested level increases the indentation by 4 spaces to create a
    /// visual hierarchy. The method filters the content to separate inline elements from nested
    /// list items, rendering the inline content first before processing any nested items.
    ///
    /// After rendering each list item's content, appropriate spacing is added based on the
    /// configured after_spacing value. The method maintains consistent styling throughout the
    /// list hierarchy while allowing for proper nesting and indentation of complex list structures.
    fn render_list_item(
        &self,
        doc: &mut Document,
        content: &[Token],
        ordered: bool,
        number: Option<usize>,
        nesting_level: usize,
    ) {
        let mut para = genpdfi::elements::Paragraph::default();
        let style = genpdfi::style::Style::new().with_font_size(self.style.list_item.size);

        let indent = "    ".repeat(nesting_level);
        if !ordered {
            para.push_styled(format!("{}- ", indent), style.clone());
        } else if let Some(n) = number {
            para.push_styled(format!("{}{}. ", indent, n), style.clone());
        }

        let inline_content: Vec<Token> = content
            .iter()
            .filter(|token| !matches!(token, Token::ListItem { .. }))
            .cloned()
            .collect();
        self.render_inline_content_with_style(&mut para, &inline_content, style);
        doc.push(para);
        doc.push(genpdfi::elements::Break::new(
            self.style.list_item.after_spacing,
        ));

        for token in content {
            if let Token::ListItem {
                content: nested_content,
                ordered: nested_ordered,
                number: nested_number,
            } = token
            {
                self.render_list_item(
                    doc,
                    nested_content,
                    *nested_ordered,
                    *nested_number,
                    nesting_level + 1,
                );
            }
        }
    }
}

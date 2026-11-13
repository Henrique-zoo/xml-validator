//! Formatação compartilhada dos diagnósticos associados a textos-fonte.
//!
//! Esta camada conhece apenas texto, spans e localizações: não decide regras
//! de XML nem de validação. Cada anotação usa um recorte da primeira linha
//! física atingida, limitado a 160 caracteres, com até 60 caracteres de
//! contexto anterior e até 80 marcadores. Isso limita a saída de XMLs sem
//! quebras de linha. As medidas usam caracteres Unicode, não largura visual
//! de grafemas; tabs do prefixo são preservados.
//!
//! Os chamadores devem fornecer um span e uma localização da mesma fonte,
//! com o início do span na linha indicada. A formatação propaga falhas do
//! destino, sem gravar arquivos ou escolher cores por conta própria.

use std::fmt;

use crate::source::{SourceLocation, Span};

/// Quantidade máxima de caracteres exibidos de uma linha física.
const MAX_EXCERPT_CHARS: usize = 160;
/// Contexto preservado antes do começo do span.
const LEADING_CONTEXT_CHARS: usize = 60;
/// Quantidade máxima de caracteres sublinhados no diagnóstico.
const MAX_MARKER_CHARS: usize = 80;

/// Exibe uma mensagem e destaca no texto-fonte a região que a originou.
///
/// # Errors
///
/// Propaga [`fmt::Error`] se a escrita do cabeçalho ou da anotação falhar.
pub(crate) fn format_diagnostic(
    f: &mut fmt::Formatter<'_>,
    heading: &str,
    source: &str,
    span: Span,
    location: SourceLocation,
    message: &str,
) -> fmt::Result {
    writeln!(
        f,
        "{heading} em {}:{}: {message}",
        location.line, location.column
    )?;

    format_source_annotation(f, source, span, location, message)
}

/// Exibe uma linha do texto-fonte e sublinha o trecho associado a uma anotação.
///
/// Equivale a [`format_source_annotation_indented`] com indentação vazia.
///
/// # Errors
///
/// Propaga [`fmt::Error`] se a escrita no destino falhar.
pub(crate) fn format_source_annotation(
    f: &mut fmt::Formatter<'_>,
    source: &str,
    span: Span,
    location: SourceLocation,
    label: &str,
) -> fmt::Result {
    format_source_annotation_indented(f, source, span, location, label, "")
}

/// Exibe uma anotação de fonte prefixando todas as linhas com uma indentação.
///
/// A indentação serve tanto a grupos de problemas simultâneos quanto a ramos
/// alternativos. O contexto visual não altera a certeza de uma regra.
/// Spans vazios recebem um marcador de inserção; labels vazias produzem
/// somente o sublinhado. Recortes indisponíveis não produzem anotação.
///
/// # Errors
///
/// Propaga [`fmt::Error`] se a escrita no destino falhar.
///
/// # Panics
///
/// Pode entrar em pânico se `span` começar antes de `location.line_span`.
/// O chamador deve manter a correspondência entre fonte, span e localização.
pub(crate) fn format_source_annotation_indented(
    f: &mut fmt::Formatter<'_>,
    source: &str,
    span: Span,
    location: SourceLocation,
    label: &str,
    indentation: &str,
) -> fmt::Result {
    let Some(line_text) = location.line_span.text(source) else {
        return Ok(());
    };

    // Partir do offset evita percorrer uma linha XML inteira para cada label.
    let offset = (span.start() - location.line_span.start()) as usize;
    let Some(prefix) = line_text.get(..offset) else {
        return Ok(());
    };
    let excerpt_start = prefix
        .char_indices()
        .rev()
        .take(LEADING_CONTEXT_CHARS)
        .last()
        .map_or(offset, |(index, _)| index);
    let mut remaining_chars = line_text[excerpt_start..].chars();
    let excerpt_body: String = remaining_chars.by_ref().take(MAX_EXCERPT_CHARS).collect();
    let truncated_right = remaining_chars.next().is_some();
    let truncated_left = excerpt_start > 0;
    let excerpt = format!(
        "{}{}{}",
        if truncated_left { "…" } else { "" },
        excerpt_body,
        if truncated_right { "…" } else { "" }
    );
    let before_marker = &line_text[excerpt_start..offset];
    let marker_prefix_body: String = before_marker
        .chars()
        .map(|ch| if ch == '\t' { '\t' } else { ' ' })
        .collect();
    let marker_prefix = format!(
        "{}{}",
        if truncated_left { " " } else { "" },
        marker_prefix_body
    );
    let marker_width = span
        .text(source)
        .map(|text| {
            text.chars()
                .take(MAX_MARKER_CHARS)
                .take_while(|ch| !matches!(ch, '\n' | '\r'))
                .count()
                .max(1)
        })
        .unwrap_or(1)
        .min(
            MAX_EXCERPT_CHARS
                .saturating_sub(before_marker.chars().count())
                .max(1),
        );

    let line_number = location.line.to_string();
    let gutter_width = line_number.len();

    writeln!(f, "{indentation}{:>width$} |", "", width = gutter_width)?;
    writeln!(
        f,
        "{indentation}{line_number:>width$} | {excerpt}",
        width = gutter_width
    )?;
    write!(
        f,
        "{indentation}{:>width$} | {marker_prefix}{}",
        "",
        "^".repeat(marker_width),
        width = gutter_width
    )?;

    if label.is_empty() {
        Ok(())
    } else {
        write!(f, " {label}")
    }
}

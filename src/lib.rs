//! Leitura dos XMLs SDP com modelos tipados, textos internados e spans de origem.
#![warn(missing_docs, rustdoc::broken_intra_doc_links)]

/// Apresentação de diagnósticos com recortes da fonte.
pub mod diagnostic;
/// Identificadores compactos para textos repetidos.
pub mod interner;
/// Parsers auxiliares de formatos de entrada.
pub mod parsers;
/// Texto-fonte e posições para diagnósticos.
pub mod source;
/// Modelos e leitura dos XMLs SDP.
pub mod xml;

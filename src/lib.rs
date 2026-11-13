//! Validador offline dos XMLs do SDP, com modelos tipados e diagnósticos localizados.
//!
//! O chamador conserva o texto XML e o [`interner::Interner`] usado no parsing.
//! [`validation::Validador`] reutiliza o catálogo incorporado entre documentos;
//! cada [`xml::reader::ParsedAtracacoes`] possui seus próprios mapas de origem.
//! A biblioteca não faz I/O nem altera os documentos. A CLI lê arquivos em lote.

#![warn(missing_docs, rustdoc::broken_intra_doc_links)]

/// Apresentação textual compartilhada entre erros de leitura e validação.
pub mod diagnostic;
/// Interner para reutilizar identificares para strings iguais.
pub mod interner;
/// Espaço reservado para parsers de outros formatos de entrada.
pub mod parsers;
/// Catálogos locais incorporados ao executável.
pub mod reference_data;
/// Texto-fonte, intervalos de bytes e localização para diagnósticos.
pub mod source;
/// Regras semânticas e árvore de diagnósticos do SDP.
pub mod validation;
/// Modelos XML, reader e espaço reservado para escrita.
pub mod xml;

//! Representação e leitura do formato XML do SDP, com espaço reservado à escrita.
//!
//! Esta camada espelha o documento externo. Seus modelos podem conter estados
//! que são estruturalmente legíveis, mas semanticamente inválidos. A
//! consistência entre campos é responsabilidade de [`crate::validation`].
//! Os mesmos modelos foram preparados para transformações posteriores, mas
//! [`writer`](crate::xml::writer) ainda não implementa a geração de XML. A leitura não carrega
//! esquemas XSD nem certifica a aceitação do arquivo pelo sistema oficial.

/// Modelos que espelham os elementos e valores presentes no XML.
pub mod models;
/// Reader com preservação dos spans do documento de origem.
pub mod reader;
/// Módulo reservado à serialização dos modelos para XML.
pub mod writer;

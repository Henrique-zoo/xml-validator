//! Escrita dos modelos SDP no formato XML.
//!
//! O módulo está reservado para a implementação do serializador.
//! Ainda não oferece funções de escrita nem é chamado pela CLI. Implementar
//! [`serde::Serialize`] em valores como [`Peso`](crate::xml::models::peso::Peso) não gera,
//! por si só, a estrutura, os nomes das tags e o namespace do documento SDP.

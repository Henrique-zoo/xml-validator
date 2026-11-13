//! Tipos que representam diretamente o vocabulário do XML SDP.
//!
//! Esses tipos são a representação canônica usada pelo reader e pela validação,
//! preparada para reutilização nas futuras transformação e escrita. Não são
//! modelos de domínio cuja construção comprova validade semântica: combinações
//! incompatíveis continuam representáveis para que o validador as diagnostique.
//!
//! Textos emprestam fatias do XML original ou são resolvidos pelo interner. Um
//! `Option` representa ausência da tag, não o código numérico “não informado”
//! de uma enumeração. Pesos usam quilogramas inteiros internamente, embora o
//! XML os expresse em toneladas. Regras entre campos ficam em
//! [`crate::validation`]. As estruturas agregadas ainda não implementam a
//! serialização completa do documento; suporte Serde de tipos individuais
//! não substitui esse writer.

/// Dados de atracação e estruturas auxiliares.
pub mod atracacao;
/// Dados de carga geral e conteinerizada.
pub mod carga;
/// Identificação do leiaute de porto organizado ou TUP.
pub mod documento;
/// Enumerações numéricas definidas pelo SDP.
pub mod enums;
/// Peso decimal exato informado em toneladas.
pub mod peso;
/// Coletas e quantidades decimais de resíduos da embarcação.
pub mod residuos;

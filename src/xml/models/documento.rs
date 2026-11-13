//! Identificação do leiaute XML usado pelo documento SDP.
//!
//! A escolha é local e usa comparação textual sensível a maiúsculas e
//! minúsculas do namespace padrão da raiz. Este módulo não resolve prefixos
//! XML, caminhos de arquivos nem endereços de rede.

use std::fmt;

/// Variante do leiaute SDP declarada no namespace de `<Movimento>`.
///
/// Porto organizado e terminal de uso privado compartilham a maior parte da
/// estrutura, mas usam identificadores de instalação e campos de peso
/// diferentes.
///
/// Aqui, "namespace" é o valor textual do atributo XML `xmlns`. Ele funciona
/// como identificador do vocabulário usado pelo documento; o reader não tenta
/// abrir, baixar nem interpretar `SDPv2.xsd` ou `SDPTup.xsd` como arquivos de
/// esquema. O nome é comparado com as constantes desta enumeração para escolher
/// qual variante decodificar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LeiauteSdp {
    /// Documento de porto organizado, com namespace `SDPv2.xsd`.
    Porto,
    /// Documento de terminal de uso privado, com namespace `SDPTup.xsd`.
    Tup,
}

impl fmt::Display for LeiauteSdp {
    /// Escreve o nome do leiaute e seu namespace, não uma tag XML.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Porto => "Porto organizado (SDPv2.xsd)",
            Self::Tup => "Terminal de uso privado (SDPTup.xsd)",
        })
    }
}

impl LeiauteSdp {
    /// Namespace esperado para documentos de porto organizado.
    pub const NAMESPACE_PORTO: &str = "SDPv2.xsd";
    /// Namespace esperado para documentos de terminal de uso privado.
    pub const NAMESPACE_TUP: &str = "SDPTup.xsd";

    /// Reconhece um leiaute a partir do namespace padrão de `<Movimento>`.
    ///
    /// Retorna `None` para valores desconhecidos. A ausência e o valor
    /// desconhecido são convertidos pelo reader em erros estruturais, pois sem
    /// essa informação não é possível decidir quais tags pertencem ao leiaute.
    pub fn from_namespace(namespace: &str) -> Option<Self> {
        match namespace {
            Self::NAMESPACE_PORTO => Some(Self::Porto),
            Self::NAMESPACE_TUP => Some(Self::Tup),
            _ => None,
        }
    }

    /// Retorna o namespace XML canônico deste leiaute.
    pub const fn namespace(self) -> &'static str {
        match self {
            Self::Porto => Self::NAMESPACE_PORTO,
            Self::Tup => Self::NAMESPACE_TUP,
        }
    }
}

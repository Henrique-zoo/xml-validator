//! Leitura do XML SDP com preservação da localização no arquivo-fonte.
//!
//! O reader executa duas etapas: constrói uma representação estrutural a
//! partir dos eventos do `quick_xml` e depois a decodifica para os modelos
//! tipados. Regras semânticas entre campos ficam fora deste módulo.
//!
//! # Mapas de origem
//!
//! O resultado preserva a origem em dois níveis deliberadamente separados:
//!
//! - [`SdpSourceMap`](crate::xml::reader::SdpSourceMap) e seus mapas tipados
//!   respondem **qual elemento** originou cada campo do modelo. Por exemplo,
//!   [`AtracacaoSourceMap::codigo_tup`](crate::xml::reader::AtracacaoSourceMap::codigo_tup)
//!   contém um `Option<ElementId>`;
//! - [`XmlSourceMap`](crate::xml::reader::XmlSourceMap) responde **qual span**
//!   pertence ao elemento. Ele resolve o
//!   [`ElementId`](crate::xml::reader::ElementId) para tokens e
//!   [`Span`](crate::source::Span);
//! - [`SourceFile`](crate::source::SourceFile) mantém o texto e resolve cada
//!   span para linha e coluna.
//!
//! ```text
//! Atracacao.codigo_tup
//!     → AtracacaoSourceMap.codigo_tup
//!     → ElementId
//!     → XmlSourceMap::start_span
//!     → Span
//!     → SourceFile::location
//! ```
//!
//! Os mapas tipados armazenam IDs de quatro bytes em vez de copiar spans. Isso
//! também mantém o índice XML genérico independente do vocabulário do SDP.
//!
//! # Empréstimos
//!
//! Logo após a leitura, os textos dos modelos são `&str` do XML fornecido a
//! [`parse_atracacoes`](crate::xml::reader::parse_atracacoes). O lifetime
//! `'xml` distingue os valores que serão preservados, enquanto `'alt` permite
//! que campos transformáveis sejam posteriormente emprestados de outra fonte.
//! O limite `'xml: 'alt` possibilita usar o próprio XML como origem inicial
//! desses campos. O resultado do reader continua sem poder sobreviver ao
//! texto-fonte.

/// Conversão da árvore XML para os modelos tipados.
mod decode;
/// Representação estrutural intermediária e consumo dos eventos XML.
mod document;
/// Erros sintáticos, estruturais e de conversão.
mod error;
/// Spans, tokens, linhas lógicas e índices de localização.
mod source;

pub use decode::parse_atracacoes;
pub use error::XmlParseError;
pub use source::{
    AtracacaoSourceMap, CargaConteinerizadaSourceMap, CargaSourceMap, ColetaResiduoSourceMap,
    ElementId, ResiduoSourceMap, SdpSourceMap, SourceMapRange, TemposDescontosSourceMap, XmlLine,
    XmlLineKind, XmlSourceMap, XmlToken, XmlTokenKind,
};

use crate::{
    source::{SourceFile, SourceLocation, Span},
    xml::models::{atracacao::Atracacao, documento::LeiauteSdp},
};

/// Resultado completo da leitura estrutural e da decodificação tipada.
///
/// `atracacoes` e `sdp_source_map.atracacoes` são vetores paralelos: o mapa de
/// índice `i` descreve a atracação de índice `i`. O
/// [`Self::xml_source_map`] é o índice global compartilhado por todos esses
/// mapas tipados.
///
/// Os valores textuais e [`Self::source`] emprestam o mesmo texto de entrada,
/// evitando cópias do conteúdo do XML. Não há referência de um campo desta
/// estrutura a outro campo dela. Alterações manuais dos vetores públicos devem
/// preservar sua ordem e correspondência com os mapas; os spans continuam
/// descrevendo o documento original, não os valores transformados.
#[derive(Debug)]
pub struct ParsedAtracacoes<'xml> {
    /// Leiaute reconhecido a partir do namespace de `<Movimento>`.
    pub leiaute: LeiauteSdp,
    /// Atracações decodificadas na mesma ordem em que aparecem no XML.
    pub atracacoes: Vec<Atracacao<'xml>>,
    /// Texto original e índice usado para resolver spans em linha e coluna.
    pub source: SourceFile<'xml>,
    /// Índice global que resolve [`ElementId`] para tokens e spans.
    ///
    /// Este tipo não conhece campos como `codigo_tup`; ele conhece apenas a
    /// estrutura física e lexical do documento XML.
    pub xml_source_map: XmlSourceMap,
    /// Mapas tipados que relacionam campos dos modelos a [`ElementId`].
    ///
    /// Use esses IDs com [`Self::xml_source_map`] ou com os métodos
    /// [`Self::element_start_span`] e [`Self::element_value_span`].
    pub sdp_source_map: SdpSourceMap,
}

impl<'xml> ParsedAtracacoes<'xml> {
    /// Retorna o documento XML original.
    pub const fn source(&self) -> &'xml str {
        self.source.text()
    }

    /// Recupera o texto original delimitado por um token.
    ///
    /// # Panics
    ///
    /// Entra em pânico se o span estiver fora do texto ou não delimitar UTF-8
    /// válido. O token deve pertencer a este documento; a identidade da fonte
    /// não é verificada quando os offsets são válidos.
    pub fn token_text(&self, token: &XmlToken) -> &'xml str {
        token
            .span
            .text(self.source.text())
            .expect("spans produzidos pelo parser sempre delimitam UTF-8 válido")
    }

    /// Retorna a fatia de tokens referenciada por uma linha lógica.
    ///
    /// # Panics
    ///
    /// Entra em pânico se o intervalo estiver fora do vetor de tokens. A linha
    /// deve pertencer a este resultado; a identidade da fonte não é verificada.
    pub fn line_tokens(&self, line: &XmlLine) -> &[XmlToken] {
        &self.xml_source_map.tokens[line.tokens.range()]
    }

    /// Recupera o texto original delimitado por uma linha lógica.
    ///
    /// # Panics
    ///
    /// Entra em pânico se o span estiver fora do texto ou não delimitar UTF-8
    /// válido. A linha deve pertencer a este documento; a identidade da fonte
    /// não é verificada quando os offsets são válidos.
    pub fn line_text(&self, line: &XmlLine) -> &'xml str {
        line.span
            .text(self.source.text())
            .expect("spans produzidos pelo parser sempre delimitam UTF-8 válido")
    }

    /// Percorre cada atracação junto ao respectivo mapa tipado de campos.
    ///
    /// O pareamento é garantido pela ordem mantida durante a decodificação e
    /// evita que consumidores precisem sincronizar os dois vetores manualmente.
    /// Se um consumidor alterar os vetores públicos de forma incompatível,
    /// o iterador `zip` termina no menor deles; não verifica esse invariante.
    pub fn atracacoes_with_source(
        &self,
    ) -> impl Iterator<Item = (&Atracacao<'xml>, &AtracacaoSourceMap)> {
        self.atracacoes.iter().zip(&self.sdp_source_map.atracacoes)
    }

    /// Resolve um [`ElementId`] para o span de sua tag de abertura.
    ///
    /// # Panics
    ///
    /// Entra em pânico se o ID ou suas referências de token estiverem fora
    /// das tabelas. O elemento deve pertencer ao mapa deste documento.
    pub fn element_start_span(&self, element: ElementId) -> Span {
        self.xml_source_map.start_span(element)
    }

    /// Resolve um [`ElementId`] para o span de seu valor.
    ///
    /// Quando o elemento não possui valor textual, retorna o span de sua tag
    /// de abertura, garantindo um ponto útil para diagnóstico.
    ///
    /// # Panics
    ///
    /// Entra em pânico se o ID ou suas referências de token estiverem fora
    /// das tabelas. O elemento deve pertencer ao mapa deste documento.
    pub fn element_value_span(&self, element: ElementId) -> Span {
        self.xml_source_map.value_or_start_span(element)
    }

    /// Resolve linha, coluna e linha física de um span.
    ///
    /// A consulta usa o índice construído uma única vez pelo reader e não
    /// percorre o documento desde o início.
    pub fn location(&self, span: Span) -> SourceLocation {
        self.source.location(span)
    }

    /// Consome o resultado e devolve somente os valores tipados.
    ///
    /// Descarta os mapas de origem e o índice de linhas, não o texto original:
    /// ele pertence ao chamador e continua emprestado pelos modelos retornados.
    pub fn into_atracacoes(self) -> Vec<Atracacao<'xml>> {
        self.atracacoes
    }
}

/// Testes estruturais, de empréstimos e de localização do reader.
#[cfg(test)]
mod tests;

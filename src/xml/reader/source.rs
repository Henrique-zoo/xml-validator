//! Tokens XML, unidades lógicas e índices de origem do modelo SDP.
//!
//! # Dois níveis de endereçamento
//!
//! Este módulo separa a associação semântica da localização física:
//!
//! - [`AtracacaoSourceMap`], [`CargaSourceMap`] e os demais mapas tipados
//!   associam cada campo conhecido do SDP a um [`ElementId`];
//! - [`XmlSourceMap`] usa o mesmo ID para encontrar os tokens e spans daquele
//!   elemento no documento original.
//!
//! Portanto, um mapa tipado não é uma segunda cópia do índice XML. Ele é uma
//! tabela compacta de referências para o índice global:
//!
//! ```text
//! campo do modelo → ElementId → XmlElementSource → TokenId → Span
//! ```
//!
//! Entidades repetidas ficam em vetores planos de [`SdpSourceMap`] e são
//! relacionadas por [`SourceMapRange`]. Essa organização evita um `Vec` por
//! carga ou contêiner e mantém `Option<ElementId>` com quatro bytes.

use std::{fmt, num::NonZeroU32, ops::Range};

use crate::source::Span;

/// Identificador compacto de um elemento na tabela global de origem XML.
///
/// O valor não nulo permite que `Option<ElementId>` também ocupe quatro bytes.
/// Um ID só é válido no [`XmlSourceMap`] que o produziu; misturar IDs de
/// documentos diferentes viola a invariável do reader.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ElementId(
    /// Índice do elemento acrescido de um, reservando zero para `Option::None`.
    NonZeroU32,
);

impl ElementId {
    /// Constrói um identificador a partir de um índice de vetor.
    ///
    /// # Panics
    ///
    /// Entra em pânico se `index >= u32::MAX`, pois o índice acrescido de um
    /// precisa caber na representação não nula.
    pub(super) fn from_index(index: usize) -> Self {
        let value: u32 = (index + 1)
            .try_into()
            .expect("uma fonte de até 4 GiB não comporta mais elementos");
        Self(NonZeroU32::new(value).expect("índices recebem deslocamento de um"))
    }

    /// Retorna o índice correspondente na tabela interna de elementos.
    ///
    /// Consumidores normalmente devem usar os métodos de [`XmlSourceMap`] em
    /// vez de indexar tabelas diretamente.
    pub const fn index(self) -> usize {
        self.0.get() as usize - 1
    }
}

/// Identificador compacto de um token na tabela lexical.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TokenId(
    /// Índice do token acrescido de um, reservando zero para `Option::None`.
    NonZeroU32,
);

impl TokenId {
    /// Constrói um identificador a partir de um índice de vetor.
    ///
    /// # Panics
    ///
    /// Entra em pânico se `index >= u32::MAX`.
    pub(super) fn from_index(index: usize) -> Self {
        let value: u32 = (index + 1)
            .try_into()
            .expect("uma fonte de até 4 GiB não comporta mais tokens");
        Self(NonZeroU32::new(value).expect("índices recebem deslocamento de um"))
    }

    /// Retorna o índice correspondente na tabela de tokens.
    pub(super) const fn index(self) -> usize {
        self.0.get() as usize - 1
    }
}

/// Intervalo compacto, exclusivo no fim, usado para indexar uma tabela plana.
///
/// Por exemplo, [`AtracacaoSourceMap::cargas`] delimita em
/// [`SdpSourceMap::cargas`] somente as cargas daquela atracação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SourceMapRange {
    /// Índice inclusivo do primeiro item na tabela, não um offset no XML.
    start: u32,
    /// Índice exclusivo do último item na tabela.
    end: u32,
}

impl SourceMapRange {
    /// Constrói um intervalo a partir de índices de vetor.
    ///
    /// O chamador deve garantir `start <= end` e limites válidos na tabela
    /// correspondente; o construtor não recebe essa tabela para verificá-los.
    ///
    /// # Panics
    ///
    /// Entra em pânico se algum índice exceder [`u32::MAX`].
    pub(super) fn new(start: usize, end: usize) -> Self {
        Self {
            start: start.try_into().expect("tabela limitada por u32"),
            end: end.try_into().expect("tabela limitada por u32"),
        }
    }

    /// Retorna o intervalo convertido para índices da plataforma.
    pub fn range(self) -> Range<usize> {
        self.start as usize..self.end as usize
    }

    /// Retorna a quantidade de itens contidos no intervalo.
    pub const fn len(self) -> u32 {
        self.end - self.start
    }

    /// Informa se o intervalo não contém itens.
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
}

/// Categorias léxicas preservadas pelo leitor XML.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XmlTokenKind {
    /// Tag de abertura, como `<Carga>`.
    Tag,
    /// Tag vazia, como `<Carga/>`.
    EmptyTag,
    /// Conteúdo textual, CDATA ou referência de entidade.
    Value,
    /// Tag de fechamento, como `</Carga>`.
    EndTag,
    /// Declaração XML, como `<?xml version="1.0"?>`.
    Declaration,
    /// Comentário, `DOCTYPE` ou instrução de processamento.
    Other,
}

impl fmt::Display for XmlTokenKind {
    /// Descreve a categoria do token em português.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Tag => "tag de abertura",
            Self::EmptyTag => "tag vazia",
            Self::Value => "valor XML",
            Self::EndTag => "tag de fechamento",
            Self::Declaration => "declaração XML",
            Self::Other => "outro token XML",
        })
    }
}

/// Token XML associado ao seu trecho exato no arquivo-fonte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XmlToken {
    /// Região exata ocupada pelo token no XML.
    pub span: Span,
    /// Profundidade do token na árvore XML; `Movimento` está na profundidade 0.
    pub depth: u32,
    /// Categoria léxica do token.
    pub kind: XmlTokenKind,
}

/// Formas de unidade lógica observadas no arquivo XML.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XmlLineKind {
    /// Abertura, um ou mais tokens de valor e fechamento consecutivos.
    TagValueEndTag,
    /// Abertura e fechamento consecutivos, sem valor entre eles.
    TagEndTag,
    /// Abertura e valor encerrados por uma quebra física antes do fechamento.
    TagValue,
    /// Unidade formada somente por uma tag de abertura ou vazia.
    Tag,
    /// Unidade formada somente por um ou mais tokens de valor.
    Value,
    /// Valor iniciado depois de uma quebra física e seu fechamento.
    ValueEndTag,
    /// Unidade formada somente por uma tag de fechamento.
    EndTag,
    /// Unidade formada pela declaração XML.
    Declaration,
    /// Combinação de tokens que não corresponde às formas anteriores.
    Other,
}

impl fmt::Display for XmlLineKind {
    /// Descreve a composição lexical da unidade lógica.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TagValueEndTag => "tag de abertura, valor e tag de fechamento",
            Self::TagEndTag => "tag de abertura e tag de fechamento",
            Self::TagValue => "tag de abertura e valor",
            Self::Tag => "tag",
            Self::Value => "valor XML",
            Self::ValueEndTag => "valor e tag de fechamento",
            Self::EndTag => "tag de fechamento",
            Self::Declaration => "declaração XML",
            Self::Other => "outra combinação de tokens XML",
        })
    }
}

/// Unidade lógica XML independente da formatação física do documento.
///
/// Uma quebra de linha física sempre encerra a unidade corrente. Na ausência
/// dela, tags contêiner, tags vazias e fechamentos estabelecem os limites
/// estruturais. Assim, `<Tag>valor</Tag>` continua sendo uma unidade mesmo em
/// um documento inteiro escrito numa única linha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XmlLine {
    /// Região do primeiro ao último token, sem indentação ou quebra de linha.
    pub span: Span,
    /// Intervalo na tabela [`XmlSourceMap::tokens`].
    pub tokens: SourceMapRange,
    /// Forma léxica reconhecida para a linha.
    pub kind: XmlLineKind,
}

/// Registro físico de um elemento na tabela global do documento.
///
/// O índice deste registro é o valor representado por [`ElementId`]. Os IDs de
/// token apontam para [`XmlSourceMap::tokens`], onde os spans efetivos ficam
/// armazenados uma única vez.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct XmlElementSource {
    /// Token da tag de abertura ou da tag vazia.
    pub(super) start_tag: TokenId,
    /// Token da tag de fechamento; ausente para tags vazias ou incompletas.
    pub(super) end_tag: Option<TokenId>,
    /// Primeiro token textual pertencente diretamente ao elemento.
    pub(super) first_value: Option<TokenId>,
    /// Último token textual, usado quando o valor atravessa linhas físicas.
    pub(super) last_value: Option<TokenId>,
}

/// Índice físico e genérico de um documento XML.
///
/// Este mapa não conhece `Atracacao`, `Carga` nem nomes de campos do SDP. Ele
/// conhece tokens, elementos e spans no texto. Os mapas tipados apontam para
/// esta estrutura por meio de [`ElementId`]. Linha e coluna são resolvidas
/// separadamente pelo [`SourceFile`](crate::source::SourceFile) do documento.
///
/// As consultas indexam vetores diretamente e pressupõem IDs deste mapa.
/// Alterar os tokens públicos não pode invalidar as referências armazenadas
/// nos elementos e nas linhas. Não existe marcador de identidade do arquivo
/// no ID: reutilizá-lo em outro mapa não é detectado se o índice estiver válido.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlSourceMap {
    /// Unidades lógicas em ordem de ocorrência no documento.
    pub lines: Vec<XmlLine>,
    /// Tokens em ordem de ocorrência no documento.
    pub tokens: Vec<XmlToken>,
    /// Registros físicos indexados diretamente por [`ElementId`].
    pub(super) elements: Vec<XmlElementSource>,
}

impl XmlSourceMap {
    /// Retorna o span da tag de abertura de um elemento.
    ///
    /// # Panics
    ///
    /// Entra em pânico se o ID do elemento ou da abertura estiver fora das tabelas.
    pub fn start_span(&self, element: ElementId) -> Span {
        self.tokens[self.elements[element.index()].start_tag.index()].span
    }

    /// Retorna o span do valor de um elemento, quando presente.
    ///
    /// Cobre do primeiro ao último token textual direto do elemento, incluindo
    /// qualquer trecho intermediário. É um recorte da fonte, não uma string
    /// decodificada: CDATA e referências de entidades preservam sua grafia.
    ///
    /// # Panics
    ///
    /// Entra em pânico se o ID ou as referências de token estiverem fora das tabelas.
    pub fn value_span(&self, element: ElementId) -> Option<Span> {
        let source = self.elements[element.index()];
        let first = source.first_value?;
        let last = source.last_value.unwrap_or(first);
        Some(
            self.tokens[first.index()]
                .span
                .cover(self.tokens[last.index()].span),
        )
    }

    /// Retorna o span da tag de fechamento de um elemento, quando presente.
    ///
    /// Tags autocontidas não têm um fechamento separado e retornam `None`.
    ///
    /// # Panics
    ///
    /// Entra em pânico se o ID ou a referência do fechamento estiver fora das tabelas.
    pub fn end_span(&self, element: ElementId) -> Option<Span> {
        self.elements[element.index()]
            .end_tag
            .map(|token| self.tokens[token.index()].span)
    }

    /// Retorna o span de um elemento completo.
    ///
    /// Cobre abertura, conteúdo e fechamento. Sem fechamento separado, retorna
    /// somente a tag de abertura ou autocontida.
    ///
    /// # Panics
    ///
    /// Entra em pânico se o ID ou as referências de token estiverem fora das tabelas.
    pub fn element_span(&self, element: ElementId) -> Span {
        let start = self.start_span(element);
        self.end_span(element).map_or(start, |end| start.cover(end))
    }

    /// Retorna o valor do elemento ou sua tag de abertura quando ele está vazio.
    ///
    /// # Panics
    ///
    /// Entra em pânico se o ID ou as referências de token estiverem fora das tabelas.
    pub fn value_or_start_span(&self, element: ElementId) -> Span {
        self.value_span(element)
            .unwrap_or_else(|| self.start_span(element))
    }
}

/// Associação entre os campos de `<TemposDescontos>` e seus elementos XML.
///
/// Cada campo contém um [`ElementId`] resolvível pelo [`XmlSourceMap`] global.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TemposDescontosSourceMap {
    /// Elemento `<TemposDescontos>`.
    pub element: ElementId,
    /// Elemento `<Codigo>`.
    pub codigo: ElementId,
    /// Elemento `<DataHoraInicio>`.
    pub data_hora_inicio: ElementId,
    /// Elemento `<DataHoraTermino>`.
    pub data_hora_termino: ElementId,
}

/// Associação entre os campos de `<CargaConteinerizada>` e seus elementos XML.
///
/// Campos opcionais usam `Option<ElementId>` sem custo adicional de tamanho em
/// relação a um ID obrigatório.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CargaConteinerizadaSourceMap {
    /// Elemento `<CargaConteinerizada>`.
    pub element: ElementId,
    /// Elemento `<CodigoCargaConteinerizada>`, quando presente.
    pub codigo_carga_conteinerizada: Option<ElementId>,
    /// Elemento `<CEMercante>`, quando presente.
    pub ce_mercante: Option<ElementId>,
    /// Elemento `<PesoCargaLiquida>`, quando presente.
    pub peso_carga_liquida: Option<ElementId>,
    /// Elemento `<PesoCargaLiquidaPropria>`, quando presente.
    pub peso_carga_liquida_propria: Option<ElementId>,
    /// Elemento `<PesoCargaLiquidaTerceiros>`, quando presente.
    pub peso_carga_liquida_terceiros: Option<ElementId>,
    /// Elemento `<IDConteiner>`.
    pub id_conteiner: ElementId,
    /// Elemento `<ImdgCode>`, quando presente.
    pub imdg_code: Option<ElementId>,
}

/// Associação entre os campos de uma `<Carga>` e seus elementos XML.
///
/// O mapa é paralelo ao respectivo valor de `Carga`. Contêineres descendentes
/// não são armazenados dentro dele: [`Self::conteineres`] delimita a fatia
/// correspondente no vetor plano [`SdpSourceMap::conteineres`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CargaSourceMap {
    /// Elemento `<Carga>`.
    pub element: ElementId,
    /// Elemento `<CnpjOperador>`.
    pub cnpj_operador: ElementId,
    /// Elemento `<CodigoCarga>`.
    pub codigo_carga: ElementId,
    /// Elemento `<CodigoNCM>` com o código NCM/SH8, quando presente.
    pub codigo_ncm: Option<ElementId>,
    /// Elemento `<CheioVazio>`, quando presente.
    pub cheio_vazio: Option<ElementId>,
    /// Elemento `<CEMercante>`, quando presente.
    pub ce_mercante: Option<ElementId>,
    /// Elemento `<NaturezaCarga>`.
    pub natureza_carga: ElementId,
    /// Elemento `<Sentido>`.
    pub sentido: Option<ElementId>,
    /// Elemento `<Navegacao>`, quando aplicável e presente.
    pub navegacao: Option<ElementId>,
    /// Elemento `<BigramaPaisOrigem>`, quando presente.
    pub bigrama_pais_origem: Option<ElementId>,
    /// Elemento `<TrigramaPortoOrigem>`, quando presente.
    pub trigrama_porto_origem: Option<ElementId>,
    /// Elemento `<CodigoTUPOrigem>`, quando presente.
    pub codigo_tup_origem: Option<ElementId>,
    /// Elemento `<CodigoAreaOrigem>`, quando presente.
    pub codigo_area_origem: Option<ElementId>,
    /// Elemento `<BigramaPaisDestino>`, quando presente.
    pub bigrama_pais_destino: Option<ElementId>,
    /// Elemento `<TrigramaPortoDestino>`, quando presente.
    pub trigrama_porto_destino: Option<ElementId>,
    /// Elemento `<CodigoTUPDestino>`, quando presente.
    pub codigo_tup_destino: Option<ElementId>,
    /// Elemento `<CodigoAreaDestino>`, quando presente.
    pub codigo_area_destino: Option<ElementId>,
    /// Elemento `<Quantidade>`, quando presente.
    pub quantidade: Option<ElementId>,
    /// Elemento `<PesoCargaBruta>`.
    pub peso_carga_bruta: ElementId,
    /// Elemento `<PesoCargaBrutaPropria>`, quando presente.
    pub peso_carga_bruta_propria: Option<ElementId>,
    /// Elemento `<PesoCargaBrutaTerceiros>`, quando presente.
    pub peso_carga_bruta_terceiros: Option<ElementId>,
    /// Elemento `<TipoOperacaoCarga>`.
    pub tipo_operacao_carga: ElementId,
    /// Elemento `<ImdgCode>`.
    pub imdg_code: ElementId,
    /// Intervalo correspondente em [`SdpSourceMap::conteineres`].
    pub conteineres: SourceMapRange,
}

/// Associação entre os campos de uma `<Atracacao>` e seus elementos XML.
///
/// O mapa é paralelo ao respectivo valor de `Atracacao`. Relações um-para-muitos
/// usam intervalos em tabelas planas: [`Self::cargas`] e
/// [`Self::tempos_descontos`] não são spans do XML.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtracacaoSourceMap {
    /// Elemento `<Atracacao>`.
    pub element: ElementId,
    /// Elemento `<TrigramaPorto>`, quando presente.
    pub trigrama_porto: Option<ElementId>,
    /// Elemento `<CodigoTUP>`, quando presente.
    pub codigo_tup: Option<ElementId>,
    /// Elemento `<NumeroAtracacao>`.
    pub numero_atracacao: ElementId,
    /// Elemento `<NREscala>`, quando presente.
    pub nr_escala: Option<ElementId>,
    /// Elemento `<LocalAtracacao>`.
    pub local_atracacao: ElementId,
    /// Elemento obrigatório `<IMON>`.
    pub imon: ElementId,
    /// Elemento `<NumeroInscricaoCapitania>`, quando presente.
    pub numero_inscricao_capitania: Option<ElementId>,
    /// Elemento `<Navegacao>`.
    pub navegacao: ElementId,
    /// Elemento `<TipoOperacao>`.
    pub tipo_operacao: ElementId,
    /// Elemento `<CnpjAgente>`.
    pub cnpj_agente: ElementId,
    /// Elemento `<NacionalidadeArmador>`.
    pub nacionalidade_armador: ElementId,
    /// Elemento `<CnpjCodigoArmador>`.
    pub cnpj_codigo_armador: ElementId,
    /// Elemento `<DataHoraChegada>`.
    pub data_hora_chegada: ElementId,
    /// Elemento `<DataHoraPrevistaAtracacao>`.
    pub data_hora_prevista_atracacao: ElementId,
    /// Elemento `<DataHoraAtracacao>`.
    pub data_hora_atracacao: ElementId,
    /// Elemento `<DataHoraDesatracacao>`.
    pub data_hora_desatracacao: ElementId,
    /// Elemento `<DataHoraInicioOperacao>`, quando presente.
    pub data_hora_inicio_operacao: Option<ElementId>,
    /// Elemento `<DataHoraTerminoOperacao>`, quando presente.
    pub data_hora_termino_operacao: Option<ElementId>,
    /// Intervalo correspondente em [`SdpSourceMap::tempos_descontos`].
    pub tempos_descontos: SourceMapRange,
    /// Elemento `<QuantidadePassageirosEmbarcados>`, quando presente.
    pub quantidade_passageiros_embarcados: Option<ElementId>,
    /// Elemento `<QuantidadePassageirosDesembarcados>`, quando presente.
    pub quantidade_passageiros_desembarcados: Option<ElementId>,
    /// Elemento `<QuantidadePassageirosTransito>`, quando presente.
    pub quantidade_passageiros_transito: Option<ElementId>,
    /// Elemento `<FlagPossuiColetaResiduo>`.
    pub flag_possui_coleta_residuo: ElementId,
    /// Intervalo de certificados em [`SdpSourceMap::coletas_residuos`].
    pub coletas_residuos: SourceMapRange,
    /// Intervalo correspondente em [`SdpSourceMap::cargas`].
    pub cargas: SourceMapRange,
}

/// Tabelas planas dos mapas tipados produzidos durante a decodificação.
///
/// `atracacoes[i]` descreve
/// [`ParsedAtracacoes::atracacoes`](super::ParsedAtracacoes::atracacoes)`[i]`.
/// Para entidades
/// descendentes, cada mapa pai contém um [`SourceMapRange`] que delimita seus
/// filhos no vetor correspondente. Todos os [`ElementId`] dessas tabelas são
/// resolvidos pelo mesmo [`XmlSourceMap`] do resultado de parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdpSourceMap {
    /// Elemento raiz `<Movimento>`, cuja tag de abertura declara o leiaute.
    pub movimento: ElementId,
    /// Mapas alinhados ao vetor de atracações.
    pub atracacoes: Vec<AtracacaoSourceMap>,
    /// Mapas de tempos de desconto, referenciados por intervalos das atracações.
    pub tempos_descontos: Vec<TemposDescontosSourceMap>,
    /// Mapas de cargas, referenciados por intervalos das atracações.
    pub cargas: Vec<CargaSourceMap>,
    /// Mapas de contêineres, referenciados por intervalos das cargas.
    pub conteineres: Vec<CargaConteinerizadaSourceMap>,
    /// Certificados de retirada de resíduos, referenciados por atracação.
    pub coletas_residuos: Vec<ColetaResiduoSourceMap>,
    /// Itens de resíduo, referenciados por certificado.
    pub residuos: Vec<ResiduoSourceMap>,
}

/// Elementos de um certificado; valores e campos permanecem nos modelos XML.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColetaResiduoSourceMap {
    /// Bloco `<ColetaResiduo>`.
    pub element: ElementId,
    /// Campo `<NumeroCertificado>`.
    pub numero_certificado: ElementId,
    /// Campo `<CNPJColetor>`.
    pub cnpj_coletor: ElementId,
    /// Campo `<DataHoraInicioColeta>`.
    pub data_hora_inicio_coleta: ElementId,
    /// Campo `<DataHoraTerminoColeta>`.
    pub data_hora_termino_coleta: ElementId,
    /// Campo `<DataHoraEntregaColeta>`.
    pub data_hora_entrega_coleta: ElementId,
    /// Campo `<CNPJReceptor>`.
    pub cnpj_receptor: ElementId,
    /// Intervalo dos itens em [`SdpSourceMap::residuos`].
    pub residuos: SourceMapRange,
}

/// Elementos de um item `<Residuo>` para diagnósticos específicos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResiduoSourceMap {
    /// Bloco que contém o item.
    pub element: ElementId,
    /// Campo `<TipoResiduo>`.
    pub tipo_residuo: ElementId,
    /// Campo `<QTDSolicitado>`.
    pub qtd_solicitado: ElementId,
    /// Campo `<QTDColetado>`.
    pub qtd_coletado: ElementId,
    /// Campo `<TipoUnidade>`.
    pub tipo_unidade: ElementId,
}

/// Classifica uma unidade lógica a partir da sequência de tokens que contém.
fn classify_line(tokens: &[XmlToken]) -> XmlLineKind {
    let first = tokens.first().map(|token| token.kind);
    let last = tokens.last().map(|token| token.kind);

    if first == Some(XmlTokenKind::Tag)
        && last == Some(XmlTokenKind::EndTag)
        && tokens.len() >= 2
        && tokens[1..tokens.len() - 1]
            .iter()
            .all(|token| token.kind == XmlTokenKind::Value)
    {
        return if tokens.len() == 2 {
            XmlLineKind::TagEndTag
        } else {
            XmlLineKind::TagValueEndTag
        };
    }
    if last == Some(XmlTokenKind::EndTag)
        && tokens.len() >= 2
        && tokens[..tokens.len() - 1]
            .iter()
            .all(|token| token.kind == XmlTokenKind::Value)
    {
        return XmlLineKind::ValueEndTag;
    }
    if first == Some(XmlTokenKind::Tag)
        && tokens.len() >= 2
        && tokens[1..]
            .iter()
            .all(|token| token.kind == XmlTokenKind::Value)
    {
        return XmlLineKind::TagValue;
    }
    if !tokens.is_empty() && tokens.iter().all(|token| token.kind == XmlTokenKind::Value) {
        return XmlLineKind::Value;
    }
    match tokens {
        [
            XmlToken {
                kind: XmlTokenKind::Tag | XmlTokenKind::EmptyTag,
                ..
            },
        ] => XmlLineKind::Tag,
        [
            XmlToken {
                kind: XmlTokenKind::EndTag,
                ..
            },
        ] => XmlLineKind::EndTag,
        [
            XmlToken {
                kind: XmlTokenKind::Declaration,
                ..
            },
        ] => XmlLineKind::Declaration,
        _ => XmlLineKind::Other,
    }
}

/// Adiciona uma unidade formada pelo intervalo de tokens informado.
///
/// # Panics
///
/// Entra em pânico se `first..end` não for um intervalo não vazio dentro de
/// `tokens` ou se seus índices não couberem em [`SourceMapRange`].
fn push_line(lines: &mut Vec<XmlLine>, tokens: &[XmlToken], first: usize, end: usize) {
    let first_span = tokens[first].span;
    let last_span = tokens[end - 1].span;
    lines.push(XmlLine {
        span: first_span.cover(last_span),
        tokens: SourceMapRange::new(first, end),
        kind: classify_line(&tokens[first..end]),
    });
}

/// Verifica se há uma quebra física desde o início do token anterior.
fn crossed_physical_line(source: &str, previous: XmlToken, current: XmlToken) -> bool {
    source
        .get(previous.span.start() as usize..current.span.start() as usize)
        .is_some_and(|between| between.contains('\n'))
}

/// Agrupa tokens em unidades lógicas sem depender de um elemento por linha.
///
/// Recebe o texto, não um índice de linhas: os limites são detectados por
/// `\n` entre inícios de tokens e por eventos estruturais. Os tokens devem
/// estar ordenados e pertencer à mesma fonte. Cada token integra uma única
/// unidade; espaços sem token não viram linhas independentes, e tokens como
/// comentários multilinha não são divididos por esta função.
///
/// Aberturas iniciam uma unidade; fechamentos, tags vazias, declarações e
/// outros eventos a encerram. Uma entrada sem tokens produz um vetor vazio.
pub(super) fn build_lines(source: &str, tokens: &[XmlToken]) -> Vec<XmlLine> {
    let mut lines = Vec::new();
    let mut first: Option<usize> = None;

    for (index, token) in tokens.iter().copied().enumerate() {
        if let Some(start) = first {
            let crossed_line = crossed_physical_line(source, tokens[index - 1], token);
            let starts_independent_unit = matches!(
                token.kind,
                XmlTokenKind::Tag
                    | XmlTokenKind::EmptyTag
                    | XmlTokenKind::Declaration
                    | XmlTokenKind::Other
            );
            if crossed_line || starts_independent_unit {
                push_line(&mut lines, tokens, start, index);
                first = None;
            }
        }

        let start = *first.get_or_insert(index);
        let ends_unit = matches!(
            token.kind,
            XmlTokenKind::EmptyTag
                | XmlTokenKind::EndTag
                | XmlTokenKind::Declaration
                | XmlTokenKind::Other
        );
        if ends_unit {
            push_line(&mut lines, tokens, start, index + 1);
            first = None;
        }
    }

    if let Some(start) = first {
        push_line(&mut lines, tokens, start, tokens.len());
    }
    lines
}

/// Testes dos tamanhos dos mapas e da apresentação das categorias lexicais.
#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use super::*;

    /// Confere a representação de quatro bytes de IDs obrigatórios e opcionais.
    #[test]
    fn optional_element_ids_remain_compact() {
        assert_eq!(size_of::<ElementId>(), 4);
        assert_eq!(size_of::<Option<ElementId>>(), 4);
    }

    /// Protege os tamanhos compactos dos mapas tipados contra regressões.
    #[test]
    fn typed_source_maps_store_ids_instead_of_spans() {
        assert_eq!(size_of::<TemposDescontosSourceMap>(), 16);
        assert_eq!(size_of::<CargaConteinerizadaSourceMap>(), 32);
        assert_eq!(size_of::<CargaSourceMap>(), 100);
        assert_eq!(size_of::<AtracacaoSourceMap>(), 116);
    }

    /// Confere a descrição de todas as categorias lexicais e linhas lógicas.
    #[test]
    fn displays_cobrem_as_categorias_de_tokens_e_linhas_logicas() {
        assert_eq!(XmlTokenKind::Tag.to_string(), "tag de abertura");
        assert_eq!(XmlTokenKind::EmptyTag.to_string(), "tag vazia");
        assert_eq!(XmlTokenKind::Value.to_string(), "valor XML");
        assert_eq!(XmlTokenKind::EndTag.to_string(), "tag de fechamento");
        assert_eq!(XmlTokenKind::Declaration.to_string(), "declaração XML");
        assert_eq!(XmlTokenKind::Other.to_string(), "outro token XML");

        assert_eq!(
            XmlLineKind::TagValueEndTag.to_string(),
            "tag de abertura, valor e tag de fechamento"
        );
        assert_eq!(
            XmlLineKind::TagEndTag.to_string(),
            "tag de abertura e tag de fechamento"
        );
        assert_eq!(XmlLineKind::TagValue.to_string(), "tag de abertura e valor");
        assert_eq!(XmlLineKind::Tag.to_string(), "tag");
        assert_eq!(XmlLineKind::Value.to_string(), "valor XML");
        assert_eq!(
            XmlLineKind::ValueEndTag.to_string(),
            "valor e tag de fechamento"
        );
        assert_eq!(XmlLineKind::EndTag.to_string(), "tag de fechamento");
        assert_eq!(XmlLineKind::Declaration.to_string(), "declaração XML");
        assert_eq!(
            XmlLineKind::Other.to_string(),
            "outra combinação de tokens XML"
        );
    }
}

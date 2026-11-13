//! Contexto compartilhado durante uma passagem de validação.
//!
//! As regras recebem este contexto em vez de [`ParsedAtracacoes`] para não
//! dependerem da representação completa produzida pelo reader. O contexto
//! expõe apenas os serviços necessários à validação: leiaute do documento,
//! dados de referência incorporados, acesso ao
//! [`SourceFile`](crate::source::SourceFile), resolução de
//! [`ElementId`](crate::xml::reader::ElementId) e acumulação de diagnósticos.
//!
//! Este arquivo não contém regras de negócio. Novos itens de checklist devem
//! ficar no módulo da entidade validada; o contexto recebe apenas operações
//! reutilizáveis para materializar seus diagnósticos.

use std::fmt;

use crate::{
    interner::{Interner, TextoId},
    reference_data::DadosReferencia,
    source::{SourceFile, Span},
    xml::{
        models::documento::LeiauteSdp,
        reader::{ElementId, ParsedAtracacoes, XmlSourceMap},
    },
};

use super::{AlternativaDiagnostico, ValidationError};

/// Parte da fonte à qual uma anotação de validação deve ser associada.
///
/// O alvo preserva [`ElementId`] enquanto a regra está sendo avaliada. O
/// contexto resolve o span correspondente somente ao materializar o erro.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AlvoDiagnostico {
    /// Tag de abertura do elemento, como `<CheioVazio>`.
    TagAbertura(ElementId),
    /// Conteúdo textual do elemento, com a tag de abertura como alternativa
    /// para elementos vazios.
    Valor(ElementId),
    /// Ponto imediatamente anterior à tag de fechamento do elemento.
    AntesTagFechamento(ElementId),
    /// Span já calculado por uma regra que exige uma região mais específica.
    Span(Span),
}

impl fmt::Display for AlvoDiagnostico {
    /// Descreve o alvo lógico para inspeção, sem resolver o ID em uma posição XML.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TagAbertura(elemento) => {
                write!(
                    formatter,
                    "tag de abertura do elemento #{}",
                    elemento.index()
                )
            }
            Self::Valor(elemento) => write!(formatter, "valor do elemento #{}", elemento.index()),
            Self::AntesTagFechamento(elemento) => write!(
                formatter,
                "posição anterior à tag de fechamento do elemento #{}",
                elemento.index()
            ),
            Self::Span(span) => write!(
                formatter,
                "span de bytes [{}..{})",
                span.start(),
                span.end()
            ),
        }
    }
}

/// Anotação ainda não resolvida para um [`Span`].
///
/// Construtores distintos tornam explícito se uma regra pretende apontar para
/// uma tag, um valor, um ponto de inserção ou uma região já calculada. Como a
/// mensagem é armazenada em [`String`], labels com textos produzidos de formas
/// diferentes podem compartilhar o mesmo vetor ou array.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RotuloDiagnostico {
    /// Região lógica que será resolvida pelo contexto.
    alvo: AlvoDiagnostico,
    /// Explicação da participação deste trecho no erro.
    mensagem: String,
}

impl RotuloDiagnostico {
    /// Cria uma anotação sobre a tag de abertura de um elemento.
    pub(super) fn na_tag_abertura(elemento: ElementId, mensagem: impl Into<String>) -> Self {
        Self {
            alvo: AlvoDiagnostico::TagAbertura(elemento),
            mensagem: mensagem.into(),
        }
    }

    /// Cria uma anotação sobre o valor textual de um elemento.
    pub(super) fn no_valor(elemento: ElementId, mensagem: impl Into<String>) -> Self {
        Self {
            alvo: AlvoDiagnostico::Valor(elemento),
            mensagem: mensagem.into(),
        }
    }

    /// Cria uma anotação no ponto anterior ao fechamento de um elemento pai.
    pub(super) fn antes_tag_fechamento(elemento: ElementId, mensagem: impl Into<String>) -> Self {
        Self {
            alvo: AlvoDiagnostico::AntesTagFechamento(elemento),
            mensagem: mensagem.into(),
        }
    }

    /// Cria uma anotação sobre um span previamente calculado.
    pub(super) fn no_span(span: Span, mensagem: impl Into<String>) -> Self {
        Self {
            alvo: AlvoDiagnostico::Span(span),
            mensagem: mensagem.into(),
        }
    }

    /// Cria uma seta sem texto sobre a tag de abertura de um elemento.
    ///
    /// Esse rótulo é útil para indicar uma evidência já explicada pela
    /// mensagem principal, sem repetir a mesma informação ao lado do trecho.
    pub(super) fn evidencia_na_tag_abertura(elemento: ElementId) -> Self {
        Self::na_tag_abertura(elemento, "")
    }

    /// Cria uma seta sem texto sobre o valor textual de um elemento.
    pub(super) fn evidencia_no_valor(elemento: ElementId) -> Self {
        Self::no_valor(elemento, "")
    }
}

/// Estado de uma passagem de validação semântica.
///
/// O contexto empresta o XML e o índice global de origem. Ele não possui os
/// modelos decodificados e, portanto, não escolhe quais entidades validar;
/// essa travessia continua sendo responsabilidade de
/// [`super::Validador::validar_atracacoes`].
///
/// `'map` limita o empréstimo dos índices e dos catálogos durante a passagem;
/// `'xml` é o tempo de vida do texto, que os diagnósticos podem continuar
/// emprestando depois do descarte do contexto. Todo [`ElementId`] deve vir do
/// mapa deste documento, e todo span direto deve apontar para esta fonte.
/// Não há verificação de identidade entre documentos: um ID fora do vetor
/// causa pânico, e um ID válido de outro documento pode apontar para outro dado.
pub(super) struct ValidationContext<'map, 'xml> {
    /// Interner que guarda o mapeamento entre as strings do xml e seus ids
    interner: &'map Interner<'xml>,
    /// Variante do leiaute reconhecida a partir do namespace de `<Movimento>`.
    leiaute: LeiauteSdp,
    /// Elemento raiz cuja tag de abertura declara o namespace do leiaute.
    movimento: ElementId,
    /// Texto e índice de linhas aos quais todos os spans se referem.
    source: &'map SourceFile<'xml>,
    /// Conjuntos imutáveis usados pelas regras que dependem de referência local.
    dados_referencia: &'map DadosReferencia,
    /// Índice XML que resolve um [`ElementId`] para tokens e spans.
    xml_source_map: &'map XmlSourceMap,
    /// Diagnósticos acumulados pelas regras já executadas.
    errors: Vec<ValidationError<'xml>>,
}

impl<'map, 'xml> ValidationContext<'map, 'xml> {
    /// Constrói o contexto a partir da saída completa do reader.
    pub(super) fn new(
        parsed: &'map ParsedAtracacoes<'xml>,
        interner: &'map Interner<'xml>,
        dados_referencia: &'map DadosReferencia,
    ) -> Self {
        Self {
            interner,
            leiaute: parsed.leiaute,
            movimento: parsed.sdp_source_map.movimento,
            source: &parsed.source,
            dados_referencia,
            xml_source_map: &parsed.xml_source_map,
            errors: Vec::new(),
        }
    }

    /// Informa se um código de carga consta no catálogo ISO incorporado.
    pub(super) fn codigo_conteiner_existe(&self, codigo: &str) -> bool {
        self.dados_referencia.codigo_conteiner_existe(codigo)
    }

    /// Retorna o leiaute de porto organizado ou TUP do documento corrente.
    pub(super) const fn leiaute(&self) -> LeiauteSdp {
        self.leiaute
    }

    /// Retorna o elemento raiz `<Movimento>` que declara o leiaute.
    pub(super) const fn movimento(&self) -> ElementId {
        self.movimento
    }

    /// Resolve um elemento para o span de sua tag de abertura.
    pub(super) fn span_tag_abertura(&self, element: ElementId) -> Span {
        self.xml_source_map.start_span(element)
    }

    /// Resolve um elemento para o span de seu valor textual.
    ///
    /// Se o elemento estiver vazio, retorna sua tag de abertura para manter um
    /// ponto útil no diagnóstico.
    pub(super) fn span_valor(&self, element: ElementId) -> Span {
        self.xml_source_map.value_or_start_span(element)
    }

    /// Empresta a grafia original de um valor para restrições de comprimento.
    pub(super) fn valor_original(&self, element: ElementId) -> &'xml str {
        self.span_valor(element)
            .text(self.source.text())
            .expect("o elemento pertence à fonte")
            .trim()
    }

    /// Retorna um span vazio imediatamente antes da tag de fechamento.
    ///
    /// Essa posição serve de âncora para diagnosticar um filho ausente sem
    /// atribuir o erro à tag de fechamento do elemento pai.
    ///
    /// Tags autocontidas usam o início da tag como posição alternativa.
    pub(super) fn span_antes_tag_fechamento(&self, element: ElementId) -> Span {
        self.xml_source_map
            .end_span(element)
            .unwrap_or_else(|| self.xml_source_map.start_span(element))
            .start_point()
    }

    /// Registra um erro cujas anotações podem apontar para tipos diferentes de
    /// região da fonte.
    ///
    /// Cada [`RotuloDiagnostico`] escolhe independentemente entre tag de
    /// abertura, valor, ponto anterior ao fechamento ou um span direto. Use
    /// [`Self::erro_com_detalhes`] quando a regra também precisar de uma
    /// explicação complementar. Os métodos especializados abaixo preservam
    /// atalhos para conjuntos homogêneos de labels.
    pub(super) fn erro<I>(&mut self, message: impl Into<String>, labels: I)
    where
        I: IntoIterator<Item = RotuloDiagnostico>,
    {
        self.registrar_erro(message, None, labels)
    }

    /// Registra um erro com o enunciado geral da regra violada.
    ///
    /// `details` deve ser uma constante de [`super::regras_de_negocio`], sem
    /// interpolação de valores do documento. O texto é emprestado, não copiado.
    /// A mensagem descreve a incompatibilidade concreta; as labels mostram
    /// evidências, valores encontrados e sugestões de correção locais.
    pub(super) fn erro_com_detalhes<I>(
        &mut self,
        message: impl Into<String>,
        details: &'static str,
        labels: I,
    ) where
        I: IntoIterator<Item = RotuloDiagnostico>,
    {
        self.registrar_erro(message, Some(details), labels)
    }

    /// Resolve os alvos das labels e armazena o diagnóstico materializado.
    fn registrar_erro<I>(
        &mut self,
        message: impl Into<String>,
        details: Option<&'static str>,
        labels: I,
    ) where
        I: IntoIterator<Item = RotuloDiagnostico>,
    {
        let labels = self.resolver_rotulos(labels);
        let error = ValidationError::new(self.source, message, details, labels);
        self.errors.push(error);
    }

    /// Executa regras usuais em uma coleção isolada, compartilhando fonte e catálogo.
    fn coletar(&self, validar: impl FnOnce(&mut Self)) -> Vec<ValidationError<'xml>> {
        let mut ramo = Self {
            interner: self.interner,
            leiaute: self.leiaute,
            movimento: self.movimento,
            source: self.source,
            dados_referencia: self.dados_referencia,
            xml_source_map: self.xml_source_map,
            errors: Vec::new(),
        };
        validar(&mut ramo);
        ramo.errors
    }

    /// Produz um ramo sem inserir seus erros na lista principal.
    pub(super) fn alternativa(
        &self,
        titulo: impl Into<String>,
        validar: impl FnOnce(&mut Self),
    ) -> AlternativaDiagnostico<'xml> {
        AlternativaDiagnostico {
            titulo: titulo.into(),
            diagnosticos: self.coletar(validar),
        }
    }

    /// Registra uma ambiguidade com uma regra geral e interpretações exclusivas.
    pub(super) fn erro_com_alternativas<I>(
        &mut self,
        mensagem: impl Into<String>,
        details: &'static str,
        labels: I,
        alternativas: Vec<AlternativaDiagnostico<'xml>>,
    ) where
        I: IntoIterator<Item = RotuloDiagnostico>,
    {
        let mut erro = ValidationError::new(
            self.source,
            mensagem,
            Some(details),
            self.resolver_rotulos(labels),
        );
        erro.alternativas = alternativas;
        self.errors.push(erro);
    }

    /// Agrupa problemas sob um enunciado geral; não cria um grupo vazio.
    pub(super) fn agrupar<I>(
        &mut self,
        mensagem: impl Into<String>,
        details: &'static str,
        labels: I,
        validar: impl FnOnce(&mut Self),
    ) where
        I: IntoIterator<Item = RotuloDiagnostico>,
    {
        let relacionados = self.coletar(validar);
        if relacionados.is_empty() {
            return;
        }
        let mut erro = ValidationError::new(
            self.source,
            mensagem,
            Some(details),
            self.resolver_rotulos(labels),
        );
        erro.relacionados = relacionados;
        self.errors.push(erro);
    }

    /// Resolve alvos heterogêneos para os spans usados no diagnóstico final.
    fn resolver_rotulos<I>(&self, labels: I) -> Vec<(Span, String)>
    where
        I: IntoIterator<Item = RotuloDiagnostico>,
    {
        labels
            .into_iter()
            .map(|label| {
                let span = match label.alvo {
                    AlvoDiagnostico::TagAbertura(element) => self.span_tag_abertura(element),
                    AlvoDiagnostico::Valor(element) => self.span_valor(element),
                    AlvoDiagnostico::AntesTagFechamento(element) => {
                        self.span_antes_tag_fechamento(element)
                    }
                    AlvoDiagnostico::Span(span) => span,
                };

                (span, label.mensagem)
            })
            .collect()
    }

    /// Registra um erro relacionado às tags de abertura de elementos XML.
    ///
    /// As anotações são recebidas como `(ElementId, mensagem)`. O contexto
    /// converte cada identificador em span e posição humana somente quando o
    /// diagnóstico é criado. `details` é o enunciado estático da regra.
    pub(super) fn erro_nas_tags_abertura<I, L>(
        &mut self,
        message: impl Into<String>,
        details: &'static str,
        labels: I,
    ) where
        I: IntoIterator<Item = (ElementId, L)>,
        L: Into<String>,
    {
        let labels = labels
            .into_iter()
            .map(|(element, label)| RotuloDiagnostico::na_tag_abertura(element, label));
        self.erro_com_detalhes(message, details, labels)
    }

    /// Registra um erro relacionado aos valores de elementos XML.
    ///
    /// As anotações são recebidas como `(ElementId, mensagem)` e resolvidas
    /// para o span textual de cada elemento. Elementos vazios usam a tag de
    /// abertura como posição alternativa. `details` é o enunciado da regra.
    pub(super) fn erro_nos_valores<I, L>(
        &mut self,
        message: impl Into<String>,
        details: &'static str,
        labels: I,
    ) where
        I: IntoIterator<Item = (ElementId, L)>,
        L: Into<String>,
    {
        let labels = labels
            .into_iter()
            .map(|(element, label)| RotuloDiagnostico::no_valor(element, label));
        self.erro_com_detalhes(message, details, labels)
    }

    /// Registra um erro em pontos imediatamente anteriores a tags de fechamento.
    ///
    /// Cada `ElementId` identifica o elemento pai em que um filho deveria ter
    /// aparecido. As anotações usam spans vazios, de modo que o renderizador
    /// mostre um cursor de inserção em vez de sublinhar a tag de fechamento.
    /// `details` é o enunciado estático da regra associada à ausência.
    pub(super) fn erro_antes_tags_fechamento<I, L>(
        &mut self,
        message: impl Into<String>,
        details: &'static str,
        labels: I,
    ) where
        I: IntoIterator<Item = (ElementId, L)>,
        L: Into<String>,
    {
        let labels = labels
            .into_iter()
            .map(|(element, label)| RotuloDiagnostico::antes_tag_fechamento(element, label));
        self.erro_com_detalhes(message, details, labels)
    }

    /// Encerra a passagem, retornando sucesso ou todos os erros acumulados.
    ///
    /// # Errors
    ///
    /// Retorna o vetor acumulado, sem achatamento dos grupos ou alternativas,
    /// quando ao menos um diagnóstico foi registrado. Consome apenas o
    /// contexto; os diagnósticos continuam emprestando o XML original.
    pub(super) fn finalizar(self) -> Result<(), Vec<ValidationError<'xml>>> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors)
        }
    }

    pub(super) fn texto(&self, id: TextoId) -> &'xml str {
        self.interner.resolve(id)
    }
}

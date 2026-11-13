//! Árvore XML intermediária construída a partir dos eventos do `quick_xml`.
//!
//! A representação usa índices estáveis em vetores. Isso permite relacionar
//! elementos, tokens e spans sem copiar o conteúdo textual do documento.
//!
//! Esta árvore existe somente durante a decodificação. Depois que os modelos e
//! mapas tipados são construídos, os nós estruturais e a tabela de símbolos
//! são descartados; [`XmlSourceMap`] permanece no resultado público.
//!
//! Filhos de um elemento formam uma lista encadeada por [`ElementId`]. Essa
//! arena evita uma alocação de `Vec` para cada elemento folha. Nomes de tags e
//! namespaces são internados como fatias do XML original, sem criar `String`.
//!
//! Os nomes são locais, sem prefixo. O namespace armazenado é apenas o
//! atributo `xmlns` declarado no próprio elemento: não há resolução de
//! prefixos nem herança de declarações. Não se trata de um processador XSD.
//! Métodos que recebem IDs pressupõem que eles pertençam a este documento;
//! as consultas indexadas podem entrar em pânico com IDs fora das tabelas.

use std::{collections::HashMap, fmt, num::NonZeroU32, str::FromStr};

use quick_xml::{
    Reader,
    events::attributes::AttrError,
    events::{BytesStart, Event},
};

use crate::source::{SourceFile, SourceLocation, Span};

use super::{
    error::XmlParseError,
    source::{
        ElementId, TokenId, XmlElementSource, XmlSourceMap, XmlToken, XmlTokenKind, build_lines,
    },
};

/// Nó estrutural compacto usado durante a decodificação.
///
/// O nó não armazena texto nem spans diretamente. `name` e `namespace`
/// referenciam a [`SymbolTable`]; a localização física fica na tabela paralela
/// de `XmlElementSource`, indexada pelo mesmo [`ElementId`].
#[derive(Debug, Clone, Copy)]
pub(super) struct ElementNode {
    /// Nome local internado do elemento, sem eventual prefixo de namespace.
    name: SymbolId,
    /// Namespace padrão declarado diretamente neste elemento, quando houver.
    namespace: Option<SymbolId>,
    /// Primeiro filho direto, usado como entrada da lista encadeada de filhos.
    first_child: Option<ElementId>,
    /// Último filho direto, mantido para inserir novos filhos em tempo constante.
    last_child: Option<ElementId>,
    /// Próximo elemento com o mesmo pai, ou `None` para o último irmão.
    next_sibling: Option<ElementId>,
}

/// Identificador de uma fatia internada durante a leitura estrutural.
///
/// Como [`SymbolTable`] guarda `&str` do XML original, o identificador evita
/// repetir ponteiros largos em cada [`ElementNode`].
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SymbolId(
    /// Índice do símbolo acrescido de um para manter o zero reservado.
    NonZeroU32,
);

/// Internador dos poucos nomes de tags e namespaces distintos do documento.
///
/// As chaves e os valores apontam diretamente para fatias do XML original;
/// internar um símbolo não copia nem aloca seu texto.
#[derive(Debug, Default)]
struct SymbolTable<'xml> {
    /// Fatias únicas, indexadas por [`SymbolId`].
    values: Vec<&'xml str>,
    /// Índice reverso usado para reutilizar um símbolo já conhecido.
    ids: HashMap<&'xml str, SymbolId>,
}

impl<'xml> SymbolTable<'xml> {
    /// Retorna o identificador existente ou registra uma fatia pela primeira vez.
    ///
    /// Pode alocar nas tabelas, mas nunca copia o texto do símbolo. A igualdade
    /// é por conteúdo, mesmo quando duas ocorrências vêm de offsets diferentes.
    ///
    /// # Panics
    ///
    /// Entra em pânico se não houver mais IDs representáveis por `u32`.
    fn intern(&mut self, value: &'xml str) -> SymbolId {
        if let Some(id) = self.ids.get(value) {
            return *id;
        }
        let raw: u32 = (self.values.len() + 1)
            .try_into()
            .expect("quantidade de símbolos limitada pelo documento");
        let id = SymbolId(NonZeroU32::new(raw).expect("índices recebem deslocamento de um"));
        self.values.push(value);
        self.ids.insert(value, id);
        id
    }

    /// Resolve um identificador para a string internada correspondente.
    fn resolve(&self, id: SymbolId) -> &str {
        &self.values[id.0.get() as usize - 1]
    }
}

/// Documento estrutural transitório usado durante a decodificação tipada.
///
/// `elements` e `source_map.elements` são tabelas paralelas indexadas pelo
/// mesmo [`ElementId`]: a primeira descreve a árvore, enquanto a segunda
/// descreve a origem física. Ao final de `parse_atracacoes`, o [`SourceFile`]
/// e o mapa de origem são movidos para o resultado público; a árvore e a
/// tabela de símbolos são descartadas.
#[derive(Debug)]
pub(super) struct XmlDocument<'xml> {
    /// Texto XML original.
    source: SourceFile<'xml>,
    /// Índice lexical produzido durante a leitura.
    pub(super) source_map: XmlSourceMap,
    /// Tabela de elementos endereçada por [`ElementId`].
    elements: Vec<ElementNode>,
    /// Nomes distintos usados pelos nós estruturais.
    symbols: SymbolTable<'xml>,
    /// Elementos encontrados fora de qualquer outro elemento.
    pub(super) roots: Vec<ElementId>,
}

impl<'xml> XmlDocument<'xml> {
    /// Cria um erro de parsing associado a este documento.
    pub(super) fn error(&self, span: Span, message: impl Into<String>) -> XmlParseError<'xml> {
        XmlParseError::at(&self.source, span, message)
    }

    /// Resolve uma posição humana usando o arquivo-fonte deste documento.
    pub(super) fn location(&self, span: Span) -> SourceLocation {
        self.source.location(span)
    }

    /// Consome a árvore transitória e preserva as estruturas necessárias às
    /// etapas posteriores.
    pub(super) fn into_source_parts(self) -> (SourceFile<'xml>, XmlSourceMap) {
        (self.source, self.source_map)
    }

    /// Obtém um nó pelo identificador estável.
    pub(super) fn element(&self, id: ElementId) -> &ElementNode {
        &self.elements[id.index()]
    }

    /// Retorna o nome local de um elemento.
    pub(super) fn name(&self, id: ElementId) -> &str {
        self.symbols.resolve(self.element(id).name)
    }

    /// Retorna o namespace padrão declarado diretamente no elemento.
    ///
    /// `None` significa ausência de `xmlns` neste nó, não ausência de um
    /// namespace que pudesse ser herdado segundo a especificação XML.
    pub(super) fn namespace(&self, id: ElementId) -> Option<&str> {
        self.element(id)
            .namespace
            .map(|namespace| self.symbols.resolve(namespace))
    }

    /// Retorna o span da tag de abertura de um elemento.
    pub(super) fn start_span(&self, id: ElementId) -> Span {
        self.source_map.start_span(id)
    }

    /// Retorna o span do valor ou, quando ausente, o da tag de abertura.
    pub(super) fn value_span(&self, id: ElementId) -> Span {
        self.source_map.value_or_start_span(id)
    }

    /// Percorre filhos selecionados sem alocar um vetor para cada consulta.
    pub(super) fn children_named_any<'a>(
        &'a self,
        parent: ElementId,
        names: &'a [&str],
    ) -> impl Iterator<Item = ElementId> + 'a {
        std::iter::successors(self.element(parent).first_child, |child| {
            self.element(*child).next_sibling
        })
        .filter(move |child| names.contains(&self.name(*child)))
    }

    /// Exige exatamente um filho; compartilha a detecção de duplicatas.
    ///
    /// `names` reúne nomes alternativos para uma única posição estrutural.
    ///
    /// # Errors
    ///
    /// Retorna erro se nenhum filho tiver um dos nomes ou se houver mais de
    /// uma ocorrência entre todos eles. A ausência aponta para o pai; a
    /// duplicidade, para a segunda ocorrência encontrada.
    pub(super) fn required_child(
        &self,
        parent: ElementId,
        names: &[&str],
    ) -> Result<ElementId, XmlParseError<'xml>> {
        self.optional_child(parent, names)?.ok_or_else(|| {
            self.error(
                self.start_span(parent),
                format!("tag obrigatória <{}> ausente", names.join("> ou <")),
            )
        })
    }

    /// Aceita zero ou um filho, parando assim que identifica uma duplicata.
    ///
    /// # Errors
    ///
    /// Retorna erro quando mais de um filho corresponder ao conjunto `names`,
    /// mesmo que os nomes das ocorrências sejam diferentes.
    pub(super) fn optional_child(
        &self,
        parent: ElementId,
        names: &[&str],
    ) -> Result<Option<ElementId>, XmlParseError<'xml>> {
        let mut children = self.children_named_any(parent, names);
        let first = children.next();
        if let Some(duplicate) = children.next() {
            return Err(self.error(
                self.start_span(duplicate),
                format!("tag <{}> repetida", self.name(duplicate)),
            ));
        }
        Ok(first)
    }

    /// Rejeita o primeiro filho cujo nome não seja permitido no contexto.
    ///
    /// Não verifica ordem, obrigatoriedade ou repetição das tags permitidas.
    ///
    /// # Errors
    ///
    /// Retorna erro na abertura do primeiro filho direto fora de `allowed`.
    pub(super) fn ensure_only_children(
        &self,
        parent: ElementId,
        allowed: &[&str],
    ) -> Result<(), XmlParseError<'xml>> {
        let mut current = self.element(parent).first_child;
        while let Some(child) = current {
            if !allowed.contains(&self.name(child)) {
                return Err(self.error(
                    self.start_span(child),
                    format!("tag <{}> não esperada neste contexto", self.name(child)),
                ));
            }
            current = self.element(child).next_sibling;
        }
        Ok(())
    }

    /// Extrai o texto emprestado de um elemento folha.
    ///
    /// Espaços externos são removidos e um invólucro CDATA completo é
    /// descartado. Elementos aninhados e valores ausentes produzem erro.
    /// O resultado continua sendo um recorte bruto: referências como `&amp;`
    /// não são expandidas e eventos textuais separados não são concatenados
    /// em uma nova string. O span de origem preserva a grafia original.
    ///
    /// # Errors
    ///
    /// Retorna erro se houver elementos filhos ou nenhum token de valor.
    /// Não impõe um formato ao conteúdo nem rejeita, por si só, uma string
    /// vazia obtida após retirar um invólucro CDATA.
    pub(super) fn text(&self, id: ElementId) -> Result<&'xml str, XmlParseError<'xml>> {
        let element = self.element(id);
        if let Some(child) = element.first_child {
            return Err(self.error(
                self.start_span(child),
                format!("a tag <{}> deve conter somente um valor", self.name(id)),
            ));
        }

        let Some(span) = self.source_map.value_span(id) else {
            return Err(self.error(
                self.start_span(id),
                format!("a tag <{}> está sem valor", self.name(id)),
            ));
        };

        let raw = span
            .text(self.source.text())
            .expect("span de valor produzido a partir do próprio XML");
        let raw = raw
            .strip_prefix("<![CDATA[")
            .and_then(|value| value.strip_suffix("]]>"))
            .unwrap_or(raw);
        Ok(raw.trim())
    }

    /// Obtém o texto de um filho obrigatório.
    ///
    /// # Errors
    ///
    /// Propaga ausência, duplicidade ou falhas de [`Self::text`].
    pub(super) fn required_text(
        &self,
        parent: ElementId,
        names: &[&str],
    ) -> Result<&'xml str, XmlParseError<'xml>> {
        self.text(self.required_child(parent, names)?)
    }

    /// Obtém o texto de um filho opcional.
    ///
    /// A ausência produz `Ok(None)`; uma tag existente sem valor não equivale
    /// a ausência.
    ///
    /// # Errors
    ///
    /// Propaga duplicidade ou falhas de [`Self::text`] no filho presente.
    pub(super) fn optional_text(
        &self,
        parent: ElementId,
        names: &[&str],
    ) -> Result<Option<&'xml str>, XmlParseError<'xml>> {
        self.optional_child(parent, names)?
            .map(|child| self.text(child))
            .transpose()
    }

    /// Converte o texto de um elemento usando sua implementação de `FromStr`.
    ///
    /// `expected` descreve o tipo no diagnóstico; a implementação de `T`
    /// determina quais valores são aceitos.
    ///
    /// # Errors
    ///
    /// Propaga falhas de extração textual e converte a falha de `T::from_str`
    /// em um diagnóstico apontando para o valor original.
    pub(super) fn parse<T>(&self, id: ElementId, expected: &str) -> Result<T, XmlParseError<'xml>>
    where
        T: FromStr,
        T::Err: fmt::Display,
    {
        let text = self.text(id)?;
        text.parse().map_err(|error| {
            self.error(
                self.value_span(id),
                format!("valor `{text}` inválido para {expected}: {error}"),
            )
        })
    }

    /// Converte o texto de um filho obrigatório.
    ///
    /// # Errors
    ///
    /// Propaga ausência, duplicidade, extração textual ou conversão para `T`.
    pub(super) fn required_parse<T>(
        &self,
        parent: ElementId,
        names: &[&str],
        expected: &str,
    ) -> Result<T, XmlParseError<'xml>>
    where
        T: FromStr,
        T::Err: fmt::Display,
    {
        self.parse(self.required_child(parent, names)?, expected)
    }

    /// Converte o texto de um filho opcional.
    ///
    /// # Errors
    ///
    /// Propaga duplicidade, extração textual ou conversão do valor presente.
    /// Retorna `Ok(None)` somente quando não existe o filho solicitado.
    pub(super) fn optional_parse<T>(
        &self,
        parent: ElementId,
        names: &[&str],
        expected: &str,
    ) -> Result<Option<T>, XmlParseError<'xml>>
    where
        T: FromStr,
        T::Err: fmt::Display,
    {
        self.optional_child(parent, names)?
            .map(|child| self.parse(child, expected))
            .transpose()
    }
}

/// Insere um token na tabela lateral e devolve seu índice.
fn push_token(tokens: &mut Vec<XmlToken>, kind: XmlTokenKind, span: Span, depth: usize) -> TokenId {
    let id = TokenId::from_index(tokens.len());
    tokens.push(XmlToken {
        span,
        depth: depth
            .try_into()
            .expect("profundidade limitada pelo arquivo"),
        kind,
    });
    id
}

/// Divide um evento textual nas linhas físicas que ele atravessa.
///
/// O `quick_xml` pode devolver uma única ocorrência de `Text` contendo uma ou
/// mais quebras de linha. Cada trecho não vazio após `trim` vira um token.
/// Retorna os IDs do primeiro e do último token para delimitar o valor completo
/// na decodificação, ou `None` se não houver texto aproveitável ou o span for
/// inválido. Os spans não incluem espaços externos de cada trecho.
fn push_value_tokens(
    tokens: &mut Vec<XmlToken>,
    source: &SourceFile,
    event_span: Span,
    depth: usize,
) -> Option<(TokenId, TokenId)> {
    let raw = event_span.text(source.text())?;
    let mut position = event_span.start() as usize;
    let mut first_value = None;
    let mut last_value = None;

    for physical_segment in raw.split_inclusive('\n') {
        let segment_without_newline = physical_segment
            .strip_suffix('\n')
            .unwrap_or(physical_segment);

        let trimmed = segment_without_newline.trim();

        if !trimmed.is_empty() {
            let leading_whitespace =
                segment_without_newline.len() - segment_without_newline.trim_start().len();
            let start = position + leading_whitespace;
            let span = source.span(start..start + trimmed.len());
            let token = push_token(tokens, XmlTokenKind::Value, span, depth);
            first_value.get_or_insert(token);
            last_value = Some(token);
        }
        position += physical_segment.len();
    }

    first_value.zip(last_value)
}

/// Insere um elemento e o liga ao pai atual ou à lista de raízes.
///
/// `elements` e `element_sources` devem ter o mesmo comprimento na entrada.
/// O último ID de `stack`, se houver, é o pai. A ligação por último filho
/// mantém a ordem de leitura sem percorrer os irmãos já inseridos.
fn push_element(
    elements: &mut Vec<ElementNode>,
    element_sources: &mut Vec<XmlElementSource>,
    roots: &mut Vec<ElementId>,
    stack: &[ElementId],
    name: SymbolId,
    namespace: Option<SymbolId>,
    start_tag: TokenId,
) -> ElementId {
    let id = ElementId::from_index(elements.len());
    elements.push(ElementNode {
        name,
        namespace,
        first_child: None,
        last_child: None,
        next_sibling: None,
    });
    element_sources.push(XmlElementSource {
        start_tag,
        end_tag: None,
        first_value: None,
        last_value: None,
    });

    if let Some(parent) = stack.last() {
        let parent_index = parent.index();
        if let Some(previous) = elements[parent_index].last_child {
            elements[previous.index()].next_sibling = Some(id);
        } else {
            elements[parent_index].first_child = Some(id);
        }
        elements[parent_index].last_child = Some(id);
    } else {
        roots.push(id);
    }
    id
}

/// Recupera o nome local diretamente do trecho correspondente no XML original.
///
/// Remove o prefixo até o último `:` sem consultar declarações de namespace.
/// O retorno empresta `source`, não o evento temporário do `quick_xml`.
///
/// # Panics
///
/// Pode entrar em pânico se `span` não delimitar a tag reconhecida pelo reader
/// ou se `end_tag` não corresponder à presença do prefixo `</`.
fn source_local_name(source: &str, span: Span, end_tag: bool) -> &str {
    let name_start = span.start() as usize + if end_tag { 2 } else { 1 };
    let event_end = span.end() as usize;
    let name_end = source.as_bytes()[name_start..event_end]
        .iter()
        .position(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n' | b'/' | b'>'))
        .map_or(event_end, |relative| name_start + relative);
    let qualified_name = source
        .get(name_start..name_end)
        .expect("o nome informado pelo quick_xml delimita UTF-8 válido");
    qualified_name
        .rsplit_once(':')
        .map_or(qualified_name, |(_, local)| local)
}

/// Lê o atributo `xmlns` e o recupera diretamente do XML original.
///
/// Não resolve `xmlns:prefixo` nem expande referências no valor. O evento e
/// seu span devem corresponder à mesma tag da fonte.
///
/// # Errors
///
/// Propaga [`AttrError`] se a iteração pelos atributos encontrar uma falha,
/// incluindo atributos malformados ou duplicados.
fn default_namespace<'xml>(
    tag: &BytesStart<'_>,
    source: &'xml str,
    span: Span,
) -> Result<Option<&'xml str>, AttrError> {
    let event_text = span
        .text(source)
        .expect("o evento informado pelo quick_xml delimita UTF-8 válido");
    let mut namespace = None;

    for attribute in tag.attributes() {
        let attribute = attribute?;
        if attribute.key.as_ref() == "xmlns" {
            let value = attribute.value.as_ref();
            let start = event_text
                .find(value)
                .expect("o valor bruto do atributo pertence ao evento XML");
            namespace = event_text.get(start..start + value.len());
        }
    }
    Ok(namespace)
}
/// Consome os eventos XML e constrói a árvore estrutural com seus spans.
///
/// Faz a passagem lexical e constrói o índice de linhas uma vez. Mantém uma
/// pilha de elementos abertos; ao final, agrupa os tokens em linhas lógicas.
/// A exigência de uma única raiz SDP e o vocabulário permitido são tratados
/// pela decodificação, não por esta função.
///
/// # Errors
///
/// Retorna [`XmlParseError`] quando o XML estiver malformado, houver
/// fechamento incompatível, tag não encerrada, atributo inválido ou conteúdo
/// textual fora de elementos. Também retorna erro se a fonte exceder o limite
/// de offsets do [`SourceFile`]. Interrompe na primeira falha.
pub(super) fn read_document<'xml>(
    text: &'xml str,
) -> Result<XmlDocument<'xml>, XmlParseError<'xml>> {
    let source_file =
        SourceFile::try_new(text).map_err(|error| XmlParseError::invalid_source(text, error))?;

    let mut reader = Reader::from_str(source_file.text());
    let mut tokens = Vec::new();
    let mut elements = Vec::new();
    let mut element_sources = Vec::new();
    let mut symbols = SymbolTable::default();
    let mut roots = Vec::new();
    let mut stack = Vec::new();

    loop {
        let start = reader.buffer_position() as usize;
        let event = match reader.read_event() {
            Ok(event) => event,
            Err(error) => {
                let span = source_file.point(reader.error_position() as usize);
                return Err(XmlParseError::at(
                    &source_file,
                    span,
                    format!("XML malformado: {error}"),
                ));
            }
        };
        let end = reader.buffer_position() as usize;
        let span = source_file.span(start..end);
        match event {
            Event::Start(tag) => {
                let token = push_token(&mut tokens, XmlTokenKind::Tag, span, stack.len());
                let namespace =
                    default_namespace(&tag, source_file.text(), span).map_err(|error| {
                        XmlParseError::at(
                            &source_file,
                            span,
                            format!("atributo XML inválido: {error}"),
                        )
                    })?;
                let name = symbols.intern(source_local_name(source_file.text(), span, false));
                let namespace = namespace.map(|namespace| symbols.intern(namespace));
                let element = push_element(
                    &mut elements,
                    &mut element_sources,
                    &mut roots,
                    &stack,
                    name,
                    namespace,
                    token,
                );
                stack.push(element);
            }
            Event::Empty(tag) => {
                let token = push_token(&mut tokens, XmlTokenKind::EmptyTag, span, stack.len());
                let namespace =
                    default_namespace(&tag, source_file.text(), span).map_err(|error| {
                        XmlParseError::at(
                            &source_file,
                            span,
                            format!("atributo XML inválido: {error}"),
                        )
                    })?;
                let name = symbols.intern(source_local_name(source_file.text(), span, false));
                let namespace = namespace.map(|namespace| symbols.intern(namespace));
                push_element(
                    &mut elements,
                    &mut element_sources,
                    &mut roots,
                    &stack,
                    name,
                    namespace,
                    token,
                );
            }
            Event::End(_) => {
                let depth = stack.len().saturating_sub(1);
                let token = push_token(&mut tokens, XmlTokenKind::EndTag, span, depth);
                let Some(element) = stack.pop() else {
                    let found = source_local_name(source_file.text(), span, true);
                    return Err(XmlParseError::at(
                        &source_file,
                        span,
                        format!("end tag </{found}> sem tag de abertura"),
                    ));
                };
                let found = source_local_name(source_file.text(), span, true);
                let expected = symbols.resolve(elements[element.index()].name);
                if expected != found {
                    return Err(XmlParseError::at(
                        &source_file,
                        span,
                        format!("esperado </{}>, encontrado </{found}>", expected),
                    ));
                }
                element_sources[element.index()].end_tag = Some(token);
            }
            Event::Text(_) | Event::CData(_) | Event::GeneralRef(_) => {
                let Some((first_value, last_value)) =
                    push_value_tokens(&mut tokens, &source_file, span, stack.len())
                else {
                    continue;
                };
                let Some(element) = stack.last().copied() else {
                    return Err(XmlParseError::at(
                        &source_file,
                        tokens[first_value.index()].span,
                        "valor encontrado fora do elemento raiz",
                    ));
                };
                let element_source = &mut element_sources[element.index()];
                element_source.first_value.get_or_insert(first_value);
                element_source.last_value = Some(last_value);
            }
            Event::Decl(_) => {
                push_token(&mut tokens, XmlTokenKind::Declaration, span, 0);
            }
            Event::Comment(_) | Event::PI(_) | Event::DocType(_) => {
                push_token(&mut tokens, XmlTokenKind::Other, span, stack.len());
            }
            Event::Eof => break,
        }
    }

    if let Some(element) = stack.last().copied() {
        let span = tokens[element_sources[element.index()].start_tag.index()].span;
        return Err(XmlParseError::at(
            &source_file,
            span,
            format!(
                "tag <{}> sem end tag",
                symbols.resolve(elements[element.index()].name)
            ),
        ));
    }

    let lines = build_lines(source_file.text(), &tokens);
    Ok(XmlDocument {
        source: source_file,
        source_map: XmlSourceMap {
            lines,
            tokens,
            elements: element_sources,
        },
        elements,
        symbols,
        roots,
    })
}

//! Diagnósticos semânticos, agrupamentos e interpretações alternativas.
//!
//! `relacionados` reúne problemas que coexistem. `alternativas` contém ramos
//! mutuamente exclusivos: cada ramo descreve exigências sob uma intenção.
//! Nenhuma regra precisa carregar um grau de confiança para usar esta árvore.

use crate::{
    diagnostic::format_source_annotation_indented,
    source::{SourceFile, SourceLocation, Span},
};
use std::fmt;

/// Trecho do XML que justifica ou localiza um diagnóstico.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationLabel {
    /// Explicação local ou texto vazio para uma seta sem legenda.
    pub message: String,
    /// Região no documento original.
    pub span: Span,
    /// Localização calculada uma única vez pelo índice compartilhado.
    pub location: SourceLocation,
}

/// Uma interpretação possível e os problemas que precisam ser resolvidos nela.
///
/// Os ramos de um mesmo diagnóstico são mutuamente exclusivos. Uma interface
/// pode expandir o ramo escolhido pelo usuário; seus problemas não devem ser
/// contabilizados junto aos dos ramos descartados. Escolher um ramo não altera
/// o XML nem comprova que ele tenha sido corrigido: a edição requer revalidação.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlternativaDiagnostico<'xml> {
    /// Intenção representada pelo ramo.
    pub titulo: String,
    /// Exigências aplicáveis somente sob essa interpretação.
    pub diagnosticos: Vec<ValidationError<'xml>>,
}

/// Violação objetiva ou agrupamento de violações do documento.
///
/// Mensagem, detalhes e labels explicam o problema. Os filhos em
/// `relacionados` coexistem; os ramos de `alternativas` são escolhas.
/// O texto-fonte é compartilhado por empréstimo, inclusive entre os ramos.
#[derive(Clone, PartialEq, Eq)]
pub struct ValidationError<'xml> {
    /// Texto completo emprestado; não duplica o documento por diagnóstico.
    source: &'xml str,
    /// Descrição curta do problema.
    pub message: String,
    /// Enunciado geral de [`super::regras_de_negocio`], compartilhado sem cópia.
    ///
    /// Não contém valores da ocorrência: esses pertencem à mensagem ou às
    /// labels. O lifetime estático independe do XML, do mapa e do validador.
    /// `None` permite diagnósticos técnicos sem uma regra de negócio associada.
    pub details: Option<&'static str>,
    /// Trechos relevantes, ordenados pela posição na fonte.
    pub labels: Box<[ValidationLabel]>,
    /// Problemas associados que coexistem, sem implicar menor certeza.
    pub relacionados: Vec<ValidationError<'xml>>,
    /// Interpretações mutuamente exclusivas, quando há ambiguidade real.
    pub alternativas: Vec<AlternativaDiagnostico<'xml>>,
}

impl fmt::Debug for ValidationError<'_> {
    /// Exibe a árvore de metadados, omitindo o conteúdo completo do XML.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ValidationError")
            .field("message", &self.message)
            .field("details", &self.details)
            .field("labels", &self.labels)
            .field("relacionados", &self.relacionados)
            .field("alternativas", &self.alternativas)
            .field("source_len", &self.source.len())
            .finish()
    }
}

impl<'xml> ValidationError<'xml> {
    /// Materializa labels e resolve suas localizações pelo índice da fonte.
    ///
    /// Os spans devem pertencer a `source`. As labels são ordenadas pelo byte
    /// inicial; linha e coluna são calculadas uma vez, evitando novo acesso ao
    /// índice na renderização. O resultado empresta somente o texto, não o
    /// [`SourceFile`], e começa sem filhos nem interpretações alternativas.
    pub(super) fn new<I, L>(
        source: &SourceFile<'xml>,
        message: impl Into<String>,
        details: Option<&'static str>,
        labels: I,
    ) -> Self
    where
        I: IntoIterator<Item = (Span, L)>,
        L: Into<String>,
    {
        let mut labels: Vec<_> = labels
            .into_iter()
            .map(|(span, message)| ValidationLabel {
                message: message.into(),
                span,
                location: source.location(span),
            })
            .collect();
        labels.sort_by_key(|label| label.span.start());
        Self {
            source: source.text(),
            message: message.into(),
            details,
            labels: labels.into_boxed_slice(),
            relacionados: Vec::new(),
            alternativas: Vec::new(),
        }
    }

    /// Texto original ao qual todos os spans deste diagnóstico se referem.
    pub const fn texto_fonte(&self) -> &'xml str {
        self.source
    }

    /// Adaptador ANSI: erros objetivos em vermelho, ramos alternativos em laranja.
    ///
    /// A cor indica o contexto de uma escolha, não uma probabilidade. A versão
    /// `Display` normal permanece sem ANSI para arquivos e testes.
    pub const fn colorido(&self) -> DiagnosticoColorido<'_, 'xml> {
        DiagnosticoColorido { erro: self }
    }

    /// Renderiza filhos cumulativos e alternativas identificadas separadamente.
    ///
    /// Escreve mensagem, detalhes e labels nessa ordem; depois, os filhos e
    /// os ramos alternativos. Cada nível acrescenta quatro espaços. Quando
    /// `cores` é verdadeiro, ramos alternativos e seus descendentes usam
    /// laranja; os demais diagnósticos usam vermelho.
    ///
    /// # Errors
    ///
    /// Propaga [`fmt::Error`] quando o destino de formatação recusa a escrita.
    fn formatar(
        &self,
        f: &mut fmt::Formatter<'_>,
        nivel: usize,
        cores: bool,
        alternativa: bool,
    ) -> fmt::Result {
        let indentacao = "    ".repeat(nivel);
        if cores {
            f.write_str(if alternativa {
                "\x1b[38;5;208m"
            } else {
                "\x1b[31m"
            })?;
        }
        write!(f, "{indentacao}Erro de validação: {}", self.message)?;
        if let Some(details) = &self.details {
            write!(f, "\n{indentacao}  detalhes: {details}")?;
        }
        for label in &self.labels {
            write!(
                f,
                "\n{indentacao}  trecho relacionado em {}:{}:\n",
                label.location.line, label.location.column
            )?;
            format_source_annotation_indented(
                f,
                self.source,
                label.span,
                label.location,
                &label.message,
                &indentacao,
            )?;
        }
        if cores {
            f.write_str("\x1b[0m")?;
        }
        for filho in &self.relacionados {
            writeln!(f)?;
            filho.formatar(f, nivel + 1, cores, alternativa)?;
        }
        for ramo in &self.alternativas {
            write!(f, "\n{indentacao}    Alternativa — {}:", ramo.titulo)?;
            for erro in &ramo.diagnosticos {
                writeln!(f)?;
                erro.formatar(f, nivel + 2, cores, true)?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for ValidationError<'_> {
    /// Renderiza a árvore de diagnósticos sem sequências ANSI.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.formatar(f, 0, false, false)
    }
}
impl std::error::Error for ValidationError<'_> {}

/// Apresentação colorida da árvore, habilitada pela interface quando adequado.
pub struct DiagnosticoColorido<'error, 'xml> {
    /// Diagnóstico emprestado pela duração da apresentação, sem clonar a árvore.
    erro: &'error ValidationError<'xml>,
}
impl fmt::Display for DiagnosticoColorido<'_, '_> {
    /// Renderiza erros em vermelho e interpretações alternativas em laranja.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.erro.formatar(f, 0, true, false)
    }
}

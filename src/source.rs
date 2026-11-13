//! Localiza trechos nos textos-fonte processados pela aplicação.
//!
//! Spans armazenam somente intervalos de bytes compactos. Linha e coluna são
//! calculadas pelo índice compartilhado apenas quando um diagnóstico precisa
//! ser produzido.
//!
//! # Representação
//!
//! Um [`Span`](crate::source::Span) usa offsets de byte `[start, end)`, e não números de linha. Isso
//! mantém cada ocorrência com oito bytes e funciona igualmente para XML
//! formatado em várias linhas ou escrito inteiro em uma única linha. O
//! índice de linhas é construído uma vez por documento, permanece encapsulado em
//! [`SourceFile`](crate::source::SourceFile) e resolve uma [`SourceLocation`](crate::source::SourceLocation) com buscas binárias.
//!
//! Todos os spans e localizações devem ser usados com o mesmo texto-fonte que
//! os originou. O tipo não carrega um identificador de arquivo para preservar
//! sua representação compacta; essa invariável é mantida pelas camadas do
//! reader e de diagnóstico.

use std::{fmt, ops::Range};

/// Falha ao construir ou operar sobre a infraestrutura comum de código-fonte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceError {
    /// O texto não cabe nos offsets de 32 bits usados por [`Span`].
    TooLarge {
        /// Tamanho efetivo do texto, em bytes.
        actual: usize,
        /// Maior tamanho representável, em bytes.
        maximum: usize,
    },
}

impl fmt::Display for SourceError {
    /// Descreve o comprimento observado e o limite suportado pela fonte.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge { actual, maximum } => {
                write!(
                    f,
                    "o texto-fonte possui {} bytes, mas o limite é {} bytes",
                    actual, maximum,
                )
            }
        }
    }
}

impl std::error::Error for SourceError {}

/// Região semiaberta `[start, end)` de um texto-fonte de até `u32::MAX` bytes.
///
/// Os offsets são medidos em bytes UTF-8. Métodos que extraem texto verificam
/// se ambos coincidem com fronteiras válidas de caracteres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    /// Offset inicial inclusivo, em bytes do texto original.
    start: u32,
    /// Offset final exclusivo; igual a `start` para um ponto de inserção.
    end: u32,
}

impl Span {
    /// Span vazio no primeiro byte do texto-fonte.
    pub const START: Self = Self { start: 0, end: 0 };

    /// Constrói um span a partir de offsets validados.
    ///
    /// O chamador garante `start <= end`; esta função não conhece o texto nem
    /// verifica fronteiras UTF-8. Prefira a delimitação oferecida por [`SourceFile`].
    ///
    /// # Panics
    ///
    /// Entra em pânico se um offset não couber em `u32`; em builds com assertions
    /// de depuração também rejeita um intervalo invertido.
    pub(crate) fn new(start: usize, end: usize) -> Self {
        debug_assert!(start <= end);
        Self {
            start: start.try_into().expect("fonte limitada a 4 GiB"),
            end: end.try_into().expect("fonte limitada a 4 GiB"),
        }
    }

    /// Retorna o offset inicial, medido em bytes.
    pub const fn start(self) -> u32 {
        self.start
    }

    /// Retorna um span vazio posicionado no início deste span.
    pub const fn start_point(self) -> Self {
        Self {
            start: self.start,
            end: self.start,
        }
    }

    /// Retorna o offset exclusivo do fim, medido em bytes.
    pub const fn end(self) -> u32 {
        self.end
    }

    /// Retorna o comprimento do intervalo em bytes.
    pub const fn len(self) -> u32 {
        self.end - self.start
    }

    /// Informa se o intervalo é vazio.
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Retorna o intervalo com offsets adequados para indexar uma string.
    pub fn range(self) -> Range<usize> {
        self.start as usize..self.end as usize
    }

    /// Obtém a fatia do texto delimitada por este span.
    ///
    /// Retorna `None` quando os offsets estão fora de `source` ou não
    /// coincidem com fronteiras válidas de UTF-8.
    pub fn text(self, source: &str) -> Option<&str> {
        source.get(self.range())
    }

    /// Produz o menor span que cobre os dois intervalos.
    ///
    /// Os dois spans devem pertencer ao mesmo arquivo. Lacunas entre os
    /// intervalos também são incluídas; não há verificação da origem.
    pub const fn cover(self, other: Self) -> Self {
        Self {
            start: if self.start < other.start {
                self.start
            } else {
                other.start
            },
            end: if self.end > other.end {
                self.end
            } else {
                other.end
            },
        }
    }
}

/// Posição humana e linha física relacionadas ao início de um [`Span`].
///
/// A estrutura é materializada somente para diagnósticos; tokens e mapas de
/// campos continuam armazenando apenas spans compactos ou IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLocation {
    /// Número da linha, começando em `1`.
    pub line: u32,
    /// Número da coluna em valores escalares Unicode, começando em `1`.
    /// Não mede células de terminal: tabulações e caracteres combinantes contam
    /// individualmente, sem expansão visual.
    pub column: u32,
    /// Intervalo da linha física, sem `CR`/`LF` no final.
    pub line_span: Span,
}

impl Default for SourceLocation {
    /// Cria uma localização inicial em `1:1`, com span de linha vazio.
    fn default() -> Self {
        Self {
            line: 1,
            column: 1,
            line_span: Span::START,
        }
    }
}

/// Índice compartilhado dos inícios de linha e caracteres UTF-8 multibyte.
///
/// A construção custa `O(n)` uma única vez. Cada consulta de linha e coluna
/// usa buscas binárias, sem percorrer novamente o prefixo da linha.
/// O índice pertence a exatamente um texto-fonte e permanece encapsulado no
/// [`SourceFile`] que o construiu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LineIndex {
    /// Offset zero e offsets imediatamente posteriores a cada `LF`, ordenados.
    starts: Vec<u32>,
    /// Offsets ordenados dos bytes que não iniciam um caractere UTF-8.
    utf8_continuation_bytes: Vec<u32>,
    /// Comprimento validado do texto associado, em bytes.
    source_len: u32,
}

impl LineIndex {
    /// Indexa as quebras de linha de um texto cujo tamanho já foi validado.
    ///
    /// `source_len` deve ser a conversão exata de `source.len()` para `u32`.
    ///
    /// # Panics
    ///
    /// Em builds com assertions de depuração, entra em pânico se o comprimento
    /// informado não corresponder ao texto. O limite já deve ter sido verificado.
    pub(crate) fn new(source: &str, source_len: u32) -> Self {
        debug_assert_eq!(source.len(), source_len as usize);
        let mut starts = vec![0];
        let mut utf8_continuation_bytes = Vec::new();
        for (index, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                starts.push((index + 1) as u32);
            } else if byte & 0b1100_0000 == 0b1000_0000 {
                utf8_continuation_bytes.push(index as u32);
            }
        }

        Self {
            starts,
            utf8_continuation_bytes,
            source_len,
        }
    }

    /// Converte um intervalo de bytes em um [`Span`].
    ///
    /// Limita ambos os extremos ao arquivo e torna o fim pelo menos igual ao
    /// início. Não ajusta offsets que caiam no meio de caracteres UTF-8.
    pub(crate) fn span(&self, range: Range<usize>) -> Span {
        let source_len = self.source_len as usize;
        let start = range.start.min(source_len);
        let end = range.end.clamp(start, source_len);
        Span::new(start, end)
    }

    /// Cria um span para o caractere que começa na posição informada.
    ///
    /// Retorna um intervalo vazio no fim do arquivo ou quando o offset aponta
    /// para um byte de continuação UTF-8. `source` deve ser o texto indexado.
    pub(crate) fn point(&self, source: &str, position: usize) -> Span {
        let position = position.min(self.source_len as usize);
        let len = source
            .get(position..)
            .and_then(|tail| tail.chars().next())
            .map(char::len_utf8)
            .unwrap_or(0);
        self.span(position..position + len)
    }

    /// Resolve linha, coluna e intervalo da linha física de um span.
    ///
    /// `source` deve ser o texto indexado, e o início de `span` deve estar
    /// dentro dele, em uma fronteira UTF-8. São feitas buscas binárias nos
    /// inícios de linha e nos bytes de continuação, sem reconstruir prefixos.
    /// O fim do span não participa do cálculo; a linha física exclui `LF` e
    /// um eventual `CR` final.
    pub(crate) fn locate(&self, source: &str, span: Span) -> SourceLocation {
        let position = span.start;
        let line_index = self.starts.partition_point(|start| *start <= position) - 1;
        let line_start = self.starts[line_index];
        let continuation_at_line = self
            .utf8_continuation_bytes
            .partition_point(|offset| *offset < line_start);
        let continuation_at_position = self
            .utf8_continuation_bytes
            .partition_point(|offset| *offset < position);
        let continuation_count = (continuation_at_position - continuation_at_line) as u32;
        let column = position - line_start - continuation_count + 1;

        let mut line_end = self
            .starts
            .get(line_index + 1)
            .copied()
            .map(|next_start| next_start - 1)
            .unwrap_or(self.source_len);
        if line_end > line_start && source.as_bytes().get(line_end as usize - 1) == Some(&b'\r') {
            line_end -= 1;
        }

        SourceLocation {
            line: (line_index + 1).try_into().unwrap_or(u32::MAX),
            column,
            line_span: Span {
                start: line_start,
                end: line_end,
            },
        }
    }
}

/// Texto-fonte acompanhado do índice usado para localizar seus spans.
///
/// `text` e `line_index` ficam privados porque formam uma unidade: substituir
/// o texto sem reconstruir o índice invalidaria todas as localizações. Um
/// `SourceFile` deve acompanhar os [`Span`] produzidos a partir dele durante
/// todo o processamento do documento.
#[derive(Debug)]
pub struct SourceFile<'src> {
    /// Texto emprestado, mantido imutável para preservar os offsets existentes.
    text: &'src str,
    /// Índice construído exclusivamente a partir de `text`.
    line_index: LineIndex,
}

impl<'src> SourceFile<'src> {
    /// Tenta construir o arquivo-fonte e indexar suas linhas e caracteres UTF-8.
    ///
    /// O texto é apenas emprestado; a construção aloca os índices de início de
    /// linha e de bytes de continuação UTF-8 usados nas consultas posteriores.
    ///
    /// # Errors
    ///
    /// Retorna [`SourceError::TooLarge`] quando o comprimento de `text` não
    /// cabe em `u32`, tipo usado pelos offsets compactos de [`Span`].
    pub fn try_new(text: &'src str) -> Result<Self, SourceError> {
        let source_len = u32::try_from(text.len()).map_err(|_| SourceError::TooLarge {
            actual: text.len(),
            maximum: u32::MAX as usize,
        })?;

        Ok(Self {
            text,
            line_index: LineIndex::new(text, source_len),
        })
    }

    /// Retorna o texto completo associado ao índice.
    pub const fn text(&self) -> &'src str {
        self.text
    }

    /// Resolve linha, coluna e linha física para o início de `span`.
    ///
    /// O span deve ter sido criado para este mesmo arquivo-fonte.
    pub fn location(&self, span: Span) -> SourceLocation {
        self.line_index.locate(self.text, span)
    }

    /// Converte um intervalo de bytes em um span limitado ao arquivo.
    ///
    /// Limita os offsets ao comprimento e normaliza intervalos invertidos,
    /// mas não ajusta fronteiras UTF-8. O recorte posterior com [`Span::text`]
    /// continua podendo retornar `None` se um limite dividir um caractere.
    pub(crate) fn span(&self, range: Range<usize>) -> Span {
        self.line_index.span(range)
    }

    /// Cria um span para o caractere que começa no offset informado.
    ///
    /// Limita posições além do texto ao fim do arquivo. No fim ou no meio de
    /// um caractere UTF-8, retorna um span vazio.
    pub(crate) fn point(&self, position: usize) -> Span {
        self.line_index.point(self.text, position)
    }
}

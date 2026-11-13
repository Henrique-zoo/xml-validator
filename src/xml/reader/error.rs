//! Erros encontrados durante a leitura e a decodificação do XML.
//!
//! O reader interrompe no primeiro erro e preserva texto, span e localização
//! suficientes para renderizá-lo sem manter a árvore intermediária. O erro
//! empresta o XML, mas não o índice de linhas usado durante sua construção.

use std::fmt;

use crate::{
    diagnostic::format_diagnostic,
    source::{SourceError, SourceFile, SourceLocation, Span},
};

/// Erro sintático ou de conversão com localização renderizável.
pub struct XmlParseError<'xml> {
    /// Texto original emprestado durante toda a vida do diagnóstico.
    source: &'xml str,
    /// Explicação legível do problema encontrado.
    pub message: String,
    /// Região do XML que originou o problema.
    pub span: Span,
    /// Linha e coluna calculadas para o diagnóstico.
    pub location: SourceLocation,
}

impl fmt::Debug for XmlParseError<'_> {
    /// Exibe metadados do erro e tamanho da fonte, sem imprimir o XML completo.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XmlParseError")
            .field("message", &self.message)
            .field("span", &self.span)
            .field("location", &self.location)
            .field("source_len", &self.source.len())
            .finish()
    }
}

impl<'xml> XmlParseError<'xml> {
    /// Constrói um erro vinculado ao documento e ao span informados.
    ///
    /// O span deve pertencer à fonte; linha e coluna são resolvidas agora,
    /// permitindo descartar o [`SourceFile`] sem invalidar o diagnóstico.
    pub(super) fn at(source: &SourceFile<'xml>, span: Span, message: impl Into<String>) -> Self {
        Self {
            source: source.text(),
            message: message.into(),
            span,
            location: source.location(span),
        }
    }

    /// Converte uma falha da infraestrutura de fonte em erro do reader XML.
    ///
    /// Como ainda não existe índice de linhas, ancora a falha no início do
    /// arquivo, em `1:1`, com um span vazio. Não tenta indexar a fonte recusada.
    pub(super) fn invalid_source(source: &'xml str, error: SourceError) -> Self {
        Self {
            source,
            message: error.to_string(),
            span: Span::START,
            location: SourceLocation::default(),
        }
    }

    /// Retorna o documento XML completo associado ao erro.
    pub const fn source_text(&self) -> &'xml str {
        self.source
    }
}

impl fmt::Display for XmlParseError<'_> {
    /// Renderiza o erro com localização e um recorte anotado da fonte.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_diagnostic(
            f,
            "Erro de parsing XML",
            self.source,
            self.span,
            self.location,
            &self.message,
        )
    }
}

impl std::error::Error for XmlParseError<'_> {}

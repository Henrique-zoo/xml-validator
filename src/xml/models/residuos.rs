//! Vocabulário de coleta de resíduos do Manual SDP §9.2.1.
//!
//! Os decimais são armazenados em milésimos da unidade declarada (m³ ou kg),
//! sem ponto flutuante. Cadastros de empresas e tipos de resíduo não são consultados.

use chrono::NaiveDateTime;
use std::{fmt, str::FromStr};

/// Quantidade exata em milésimos de uma unidade de resíduo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuantidadeResiduo {
    /// Valor escalado por mil, independente da unidade física.
    pub milesimos: u64,
}

/// Decimal não negativo inválido ou não representável em `u64` milésimos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseQuantidadeResiduoError;

impl fmt::Display for ParseQuantidadeResiduoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("esperado decimal não negativo com até três casas, sem sinal")
    }
}
impl std::error::Error for ParseQuantidadeResiduoError {}

impl FromStr for QuantidadeResiduo {
    type Err = ParseQuantidadeResiduoError;
    /// Converte a parte inteira e até três decimais, com aritmética verificada.
    ///
    /// # Errors
    /// Retorna erro para sinal, vírgula, fração vazia, mais de três casas ou overflow.
    fn from_str(texto: &str) -> Result<Self, Self::Err> {
        let erro = ParseQuantidadeResiduoError;
        let (inteiro, fracao) = texto
            .split_once('.')
            .map_or((texto, None), |(a, b)| (a, Some(b)));
        if inteiro.is_empty()
            || !inteiro.bytes().all(|b| b.is_ascii_digit())
            || fracao.is_some_and(|f| {
                f.is_empty() || f.len() > 3 || !f.bytes().all(|b| b.is_ascii_digit())
            })
        {
            return Err(erro);
        }
        let inteiro = inteiro.parse::<u64>().map_err(|_| erro)?;
        let fracao = match fracao {
            Some(f) => f.parse::<u64>().map_err(|_| erro)? * 10u64.pow(3 - f.len() as u32),
            None => 0,
        };
        inteiro
            .checked_mul(1000)
            .and_then(|n| n.checked_add(fracao))
            .map(|milesimos| Self { milesimos })
            .ok_or(erro)
    }
}

impl fmt::Display for QuantidadeResiduo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{:03}", self.milesimos / 1000, self.milesimos % 1000)
    }
}

/// Resíduo informado dentro de uma coleta.
#[derive(Debug)]
pub struct Residuo {
    /// Código cadastral, preservado sem comprovar existência no GISIS.
    pub tipo_residuo: u32,
    /// Quantidade solicitada, na unidade de `tipo_unidade`.
    pub qtd_solicitado: QuantidadeResiduo,
    /// Quantidade efetivamente coletada, na unidade de `tipo_unidade`.
    pub qtd_coletado: QuantidadeResiduo,
    /// Código de unidade: 1 para m³ e 2 para kg; validade pertence à validação.
    pub tipo_unidade: u32,
}

/// Certificado de retirada de resíduos associado a uma atracação.
#[derive(Debug)]
pub struct ColetaResiduo<'xml> {
    /// Identificador obrigatório do certificado.
    pub numero_certificado: &'xml str,
    /// CNPJ da empresa coletora, sem máscara.
    pub cnpj_coletor: &'xml str,
    /// Início da coleta, sem fuso horário.
    pub data_hora_inicio_coleta: NaiveDateTime,
    /// Término da coleta, sem fuso horário.
    pub data_hora_termino_coleta: NaiveDateTime,
    /// Entrega dos resíduos, sem fuso horário.
    pub data_hora_entrega_coleta: NaiveDateTime,
    /// CNPJ da empresa receptora, sem máscara.
    pub cnpj_receptor: &'xml str,
    /// Itens coletados; a validação exige ao menos um.
    pub residuos: Vec<Residuo>,
}

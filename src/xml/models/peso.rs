//! Representação decimal exata dos pesos informados pelo SDP.
//!
//! O XML expressa pesos em toneladas com até três casas decimais. O valor é
//! armazenado em quilogramas inteiros para evitar arredondamentos de ponto
//! flutuante.
//!
//! [`Peso`](crate::xml::models::peso::Peso) admite zero; positividade, limites do SDP e relações entre pesos
//! pertencem à validação. A conversão textual não limita o campo a 13 posições.
//! [`std::fmt::Display`] e a serialização produzem toneladas com três casas.

use std::{
    fmt,
    iter::Sum,
    ops::{Add, AddAssign},
    str::FromStr,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

/// Peso informado no XML em toneladas e armazenado exatamente em quilogramas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Peso {
    /// Massa exata, sem sinal, na unidade interna de um quilograma.
    quilogramas: u64,
}

impl Peso {
    /// Quantidade de quilogramas correspondente a uma tonelada.
    pub const QUILOGRAMAS_POR_TONELADA: u64 = 1_000;
    /// Peso nulo, usado como identidade da soma.
    pub const ZERO: Self = Self { quilogramas: 0 };

    /// Constrói um peso a partir de uma quantidade exata de quilogramas.
    pub const fn from_quilogramas(quilogramas: u64) -> Self {
        Self { quilogramas }
    }

    /// Constrói um peso a partir de toneladas inteiras, sem parte fracionária.
    ///
    /// O chamador deve garantir `toneladas <= u64::MAX / 1_000`. A multiplicação
    /// não é verificada explicitamente: em builds sem verificação de overflow,
    /// exceder esse limite produz um valor truncado por aritmética modular.
    ///
    /// # Panics
    ///
    /// Com verificação de overflow habilitada, entra em pânico se a conversão
    /// para quilogramas ultrapassar `u64::MAX`.
    pub const fn from_toneladas(toneladas: u64) -> Self {
        Self {
            quilogramas: toneladas * Self::QUILOGRAMAS_POR_TONELADA,
        }
    }

    /// Retorna o valor exato armazenado, em quilogramas.
    pub const fn quilogramas(self) -> u64 {
        self.quilogramas
    }

    /// Soma dois pesos sem permitir estouro da representação em `u64`.
    ///
    /// Retorna `None` quando o total não pode ser representado por [`Peso`].
    pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        match self.quilogramas.checked_add(rhs.quilogramas) {
            Some(quilogramas) => Some(Self { quilogramas }),
            None => None,
        }
    }

    /// Retorna uma aproximação em toneladas para cálculos que exigem ponto
    /// flutuante.
    pub fn toneladas_f64(self) -> f64 {
        self.quilogramas as f64 / Self::QUILOGRAMAS_POR_TONELADA as f64
    }
}

impl Add for Peso {
    /// Peso exato resultante da adição, também armazenado em quilogramas.
    type Output = Self;

    /// Soma dois pesos exatos.
    ///
    /// # Panics
    ///
    /// Entra em pânico se o total ultrapassar [`u64::MAX`] quilogramas. Use
    /// [`Peso::checked_add`] quando o estouro precisar ser tratado como dado
    /// inválido.
    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(rhs)
            .expect("a soma dos pesos excedeu u64::MAX quilogramas")
    }
}

impl AddAssign for Peso {
    /// Acrescenta `rhs` ao peso atual usando a mesma soma verificada de `+`.
    ///
    /// # Panics
    ///
    /// Entra em pânico se o total ultrapassar `u64::MAX` quilogramas.
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sum for Peso {
    /// Soma um iterador de pesos, usando [`Peso::ZERO`] para o iterador vazio.
    ///
    /// # Panics
    ///
    /// Entra em pânico se algum total intermediário ultrapassar
    /// [`u64::MAX`] quilogramas.
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Self>,
    {
        iter.fold(Self::ZERO, Add::add)
    }
}

impl<'peso> Sum<&'peso Peso> for Peso {
    /// Soma referências sem exigir cópias explícitas no chamador.
    ///
    /// # Panics
    ///
    /// Entra em pânico se algum total intermediário ultrapassar `u64::MAX`
    /// quilogramas, assim como a soma de valores possuídos.
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = &'peso Peso>,
    {
        iter.copied().sum()
    }
}

/// Erro retornado quando um texto não representa um [`Peso`] válido.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsePesoError;

impl fmt::Display for ParsePesoError {
    /// Descreve o formato decimal esperado, comum às falhas de conversão.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("esperado um peso em toneladas, com até 3 casas decimais")
    }
}

impl std::error::Error for ParsePesoError {}

impl FromStr for Peso {
    /// Falha de formato decimal ou de capacidade da representação inteira.
    type Err = ParsePesoError;

    /// Converte toneladas decimais para quilogramas sem arredondamento.
    ///
    /// Aceita parte inteira obrigatória e de uma a três casas após um ponto
    /// opcional. Por exemplo, `1`, `1.2` e `1.250` correspondem a 1.000,
    /// 1.200 e 1.250 quilogramas. Não remove espaços nem aceita sinal ou vírgula.
    ///
    /// # Errors
    ///
    /// Retorna [`ParsePesoError`] para formato inválido, mais de três casas
    /// decimais ou valor cuja conversão exceda `u64::MAX` quilogramas.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut parts = text.split('.');
        let toneladas = parts.next().unwrap_or_default();
        let fracao = parts.next();

        if toneladas.is_empty()
            || !toneladas.bytes().all(|byte| byte.is_ascii_digit())
            || parts.next().is_some()
            || fracao.is_some_and(|value| {
                value.is_empty()
                    || value.len() > 3
                    || !value.bytes().all(|byte| byte.is_ascii_digit())
            })
        {
            return Err(ParsePesoError);
        }

        let toneladas: u64 = toneladas.parse().map_err(|_| ParsePesoError)?;
        let quilogramas_da_fracao = match fracao {
            Some(value) => {
                let parsed: u64 = value.parse().map_err(|_| ParsePesoError)?;
                parsed * 10_u64.pow(3 - value.len() as u32)
            }
            None => 0,
        };

        let quilogramas = toneladas
            .checked_mul(Self::QUILOGRAMAS_POR_TONELADA)
            .and_then(|value| value.checked_add(quilogramas_da_fracao))
            .ok_or(ParsePesoError)?;

        Ok(Self { quilogramas })
    }
}

impl fmt::Display for Peso {
    /// Escreve toneladas com ponto decimal e exatamente três casas fracionárias.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let toneladas = self.quilogramas / Self::QUILOGRAMAS_POR_TONELADA;
        let quilogramas = self.quilogramas % Self::QUILOGRAMAS_POR_TONELADA;
        write!(f, "{toneladas}.{quilogramas:03}")
    }
}

impl Serialize for Peso {
    /// Serializa a forma textual em toneladas, não o número interno de quilogramas.
    ///
    /// # Errors
    ///
    /// Propaga falhas do serializador ao escrever a representação de [`Display`](fmt::Display).
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Peso {
    /// Lê uma string e aplica a mesma conversão decimal de [`FromStr`].
    ///
    /// Aloca temporariamente a string recebida pelo Serde; o reader próprio
    /// chama `FromStr` sobre uma fatia do XML e não usa este caminho.
    ///
    /// # Errors
    ///
    /// Propaga a falha ao obter a string ou um [`ParsePesoError`] adaptado ao
    /// tipo de erro do desserializador.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(D::Error::custom)
    }
}

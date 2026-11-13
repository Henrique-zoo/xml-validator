//! Funções puras compartilhadas entre regras de validação.
//!
//! Os helpers deste módulo não conhecem o modelo XML, mapas de origem nem o
//! [`ValidationContext`](crate::validation::context::ValidationContext). Eles apenas
//! verificam valores e devolvem informação suficiente para que a regra
//! chamadora construa um diagnóstico adequado ao campo e ao contexto
//! semântico em que o valor apareceu.

use std::fmt;

use stdbr::cnpj::{self, CnpjError};

/// Motivo pelo qual um texto não obedece ao formato lexical de um código.
///
/// O erro não conhece tags nem posições no XML. As regras chamadoras
/// acrescentam esse contexto ao convertê-lo em um diagnóstico.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ErroFormatoTexto {
    /// O código deveria possuir uma quantidade exata de caracteres.
    ComprimentoExato {
        /// Quantidade exigida pelo formato.
        esperado: usize,
        /// Quantidade encontrada no valor.
        encontrado: usize,
    },
    /// O texto ultrapassou o comprimento máximo permitido.
    ComprimentoMaximo {
        /// Maior quantidade aceita pelo formato.
        maximo: usize,
        /// Quantidade encontrada no valor.
        encontrado: usize,
    },
    /// Um caractere não pertence ao conjunto permitido pelo formato.
    CaractereInvalido {
        /// Posição humana do caractere, iniciada em 1.
        posicao: usize,
        /// Caractere encontrado nessa posição.
        encontrado: char,
        /// Descrição do conjunto de caracteres aceito.
        esperado: &'static str,
    },
}

impl fmt::Display for ErroFormatoTexto {
    /// Descreve o comprimento ou o caractere inválido para uma label local.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ComprimentoExato {
                esperado,
                encontrado,
            } => write!(
                formatter,
                "este valor contém {encontrado} caracteres; eram esperados exatamente {esperado}"
            ),
            Self::ComprimentoMaximo { maximo, encontrado } => write!(
                formatter,
                "este valor contém {encontrado} caracteres; são permitidos no máximo {maximo}"
            ),
            Self::CaractereInvalido {
                posicao,
                encontrado,
                esperado,
            } => write!(
                formatter,
                "o caractere `{encontrado}` na posição {posicao} não é {esperado}"
            ),
        }
    }
}

/// Exige que `valor` possua exatamente `N` caracteres Unicode.
///
/// Conta valores escalares, não bytes nem grafemas, e não remove espaços.
///
/// # Errors
///
/// Retorna [`ErroFormatoTexto::ComprimentoExato`] com ambas as quantidades
/// quando elas diferirem.
pub(super) fn validar_comprimento_exato<const N: usize>(
    valor: &str,
) -> Result<(), ErroFormatoTexto> {
    let encontrado = valor.chars().count();
    if encontrado == N {
        Ok(())
    } else {
        Err(ErroFormatoTexto::ComprimentoExato {
            esperado: N,
            encontrado,
        })
    }
}

/// Exige que `valor` possua no máximo `N` caracteres Unicode.
///
/// A string vazia é aceita, inclusive para `N = 0`.
///
/// # Errors
///
/// Retorna [`ErroFormatoTexto::ComprimentoMaximo`] quando a contagem de
/// valores escalares Unicode ultrapassar `N`.
pub(super) fn validar_comprimento_maximo<const N: usize>(
    valor: &str,
) -> Result<(), ErroFormatoTexto> {
    let encontrado = valor.chars().count();
    if encontrado <= N {
        Ok(())
    } else {
        Err(ErroFormatoTexto::ComprimentoMaximo {
            maximo: N,
            encontrado,
        })
    }
}

/// Exige um código de exatamente `N` algarismos ASCII.
///
/// A verificação do comprimento precede a dos caracteres. Assim, cada chamada
/// produz um único motivo determinístico para a rejeição do valor.
///
/// # Errors
///
/// Retorna comprimento incorreto ou, se ele for válido, a primeira posição
/// que não contiver um algarismo ASCII de `0` a `9`.
pub(super) fn validar_codigo_numerico<const N: usize>(valor: &str) -> Result<(), ErroFormatoTexto> {
    validar_comprimento_exato::<N>(valor)?;
    validar_caracteres(valor, char::is_ascii_digit, "um algarismo ASCII de 0 a 9")
}

/// Exige um código de exatamente `N` caracteres alfanuméricos ASCII.
///
/// Letras maiúsculas e minúsculas são aceitas. Regras que exijam caixa alta
/// devem aplicar essa restrição adicional no validador específico do campo.
///
/// # Errors
///
/// Retorna comprimento incorreto antes de procurar o primeiro caractere que
/// não seja letra ou algarismo ASCII. A posição é contada a partir de 1.
pub(super) fn validar_codigo_alfanumerico<const N: usize>(
    valor: &str,
) -> Result<(), ErroFormatoTexto> {
    validar_comprimento_exato::<N>(valor)?;
    validar_caracteres(
        valor,
        char::is_ascii_alphanumeric,
        "uma letra ou um algarismo ASCII",
    )
}

/// Exige exatamente `N` letras ASCII maiúsculas, de `A` a `Z`.
///
/// Apesar do nome genérico, letras minúsculas e letras acentuadas não são
/// aceitas. O valor não é normalizado antes da verificação.
///
/// # Errors
///
/// Retorna comprimento incorreto antes de procurar o primeiro caractere
/// inválido, cuja posição é contada a partir de 1.
pub(super) fn validar_codigo_alfabetico<const N: usize>(
    valor: &str,
) -> Result<(), ErroFormatoTexto> {
    validar_comprimento_exato::<N>(valor)?;
    validar_caracteres(valor, char::is_ascii_uppercase, "uma letra ASCII")
}

/// Exige um texto alfanumérico ASCII com no máximo `N` caracteres.
///
/// Aceita a string vazia e letras de ambas as caixas, sem normalização.
///
/// # Errors
///
/// Retorna excesso de comprimento ou o primeiro caractere não alfanumérico,
/// nessa ordem de precedência.
pub(super) fn validar_alfanumerico_com_comprimento_maximo<const N: usize>(
    valor: &str,
) -> Result<(), ErroFormatoTexto> {
    validar_comprimento_maximo::<N>(valor)?;
    validar_caracteres(
        valor,
        char::is_ascii_alphanumeric,
        "uma letra ou um algarismo ASCII",
    )
}

/// Retorna o primeiro caractere rejeitado pelo predicado informado.
///
/// `esperado` descreve o conjunto permitido na mensagem, sem afetar a
/// decisão do predicado. Uma string vazia satisfaz a verificação.
///
/// # Errors
///
/// Retorna [`ErroFormatoTexto::CaractereInvalido`] com posição baseada em 1,
/// medida em valores escalares Unicode, não em bytes.
fn validar_caracteres(
    valor: &str,
    permitido: fn(&char) -> bool,
    esperado: &'static str,
) -> Result<(), ErroFormatoTexto> {
    if let Some((posicao, encontrado)) = valor
        .chars()
        .enumerate()
        .find(|(_, caractere)| !permitido(caractere))
    {
        return Err(ErroFormatoTexto::CaractereInvalido {
            posicao: posicao + 1,
            encontrado,
            esperado,
        });
    }

    Ok(())
}

/// Verifica se um texto representa um CNPJ estruturalmente válido.
///
/// Em caso de falha, retorna o [`CnpjError`] original e uma mensagem adequada
/// para anotar o valor verificado. A regra chamadora continua responsável pela
/// mensagem principal do diagnóstico, pelos spans relacionados e por decidir
/// se o campo deveria ou não conter um CNPJ.
///
/// A validação estrita de [`stdbr::cnpj`] aceita a forma compacta de 14
/// caracteres e a forma pontuada `XX.XXX.XXX/XXXX-DD`, tanto para CNPJs
/// numéricos quanto para o formato alfanumérico. São verificados comprimento,
/// caracteres e separadores, bases degeneradas e os dois dígitos verificadores.
///
/// A função comprova apenas a formação do identificador. Por decisão de
/// arquitetura, a validação não consulta o cadastro da Receita Federal e,
/// portanto, não comprova que o CNPJ exista, esteja ativo ou pertença ao
/// responsável informado.
///
/// # Errors
///
/// Retorna a causa de [`stdbr::cnpj`] junto à mensagem local correspondente.
/// Não registra diagnóstico nem associa o erro a um span; isso cabe à regra.
pub(super) fn validar_cnpj(value: &str) -> Result<(), (CnpjError, String)> {
    let Err(error) = cnpj::is_valid_strict(value) else {
        return Ok(());
    };

    let message = match &error {
        CnpjError::InvalidLength => format!(
            "este valor possui {} caracteres; esperavam-se 14 sem pontuação ou 18 na forma pontuada",
            value.chars().count(),
        ),
        CnpjError::InvalidCharacter => String::from(
            "as 12 primeiras posições admitem somente algarismos ou letras ASCII maiúsculas; as duas últimas admitem somente algarismos",
        ),
        CnpjError::InvalidFormat => {
            String::from("o CNPJ deve seguir um dos padrões XX.XXX.XXX/XXXX-DD ou XXXXXXXXXXXXDD")
        }
        CnpjError::AllCharsEqual => {
            String::from("as 12 posições da base do CNPJ não podem conter todas o mesmo caractere")
        }
        CnpjError::InvalidCheckDigits => String::from(
            "os dois últimos algarismos não correspondem aos dígitos verificadores calculados",
        ),
    };

    Err((error, message))
}

/// Formata nomes já selecionados por uma regra para uma mensagem em português.
pub trait ListagemOxford {
    /// Separa itens por vírgula e os dois últimos por `e`, sem lista intermediária.
    ///
    /// Por exemplo, `["A", "B", "C"]` produz `"A, B e C"`. A lista vazia
    /// produz uma string vazia; os itens não são normalizados nem escapados.
    fn join_oxford(&self) -> String;
}
impl ListagemOxford for [&str] {
    /// Constrói a listagem com uma alocação dimensionada pelos itens e separadores.
    fn join_oxford(&self) -> String {
        match self {
            [] => String::new(),
            [unico] => unico.to_string(),
            [resto @ .., ultimo] => {
                // Dois bytes por ", " e três pelo último separador " e ".
                let capacidade =
                    self.iter().map(|s| s.len()).sum::<usize>() + (resto.len() - 1) * 2 + 3;

                let mut resultado = resto.iter().enumerate().fold(
                    String::with_capacity(capacidade),
                    |mut acc, (i, item)| {
                        if i > 0 {
                            acc.push_str(", ");
                        }
                        acc.push_str(item);
                        acc
                    },
                );
                resultado.push_str(" e ");
                resultado.push_str(ultimo);
                resultado
            }
        }
    }
}

/// Testes dos motivos específicos de rejeição dos formatos lexicais.
#[cfg(test)]
mod tests {
    use super::*;

    /// Distingue comprimento incorreto da posição de um caractere não numérico.
    #[test]
    fn codigo_numerico_distingue_comprimento_de_caractere_invalido() {
        assert_eq!(
            validar_codigo_numerico::<4>("123"),
            Err(ErroFormatoTexto::ComprimentoExato {
                esperado: 4,
                encontrado: 3,
            })
        );
        assert_eq!(
            validar_codigo_numerico::<4>("12A4"),
            Err(ErroFormatoTexto::CaractereInvalido {
                posicao: 3,
                encontrado: 'A',
                esperado: "um algarismo ASCII de 0 a 9",
            })
        );
    }

    /// Confere que o código alfanumérico aceite ASCII e rejeite pontuação e acentos.
    #[test]
    fn codigo_alfanumerico_rejeita_pontuacao_e_unicode() {
        assert!(validar_codigo_alfanumerico::<4>("22G1").is_ok());
        assert!(matches!(
            validar_codigo_alfanumerico::<4>("22-1"),
            Err(ErroFormatoTexto::CaractereInvalido { posicao: 3, .. })
        ));
        assert!(matches!(
            validar_codigo_alfanumerico::<4>("22Á1"),
            Err(ErroFormatoTexto::CaractereInvalido { posicao: 3, .. })
        ));
    }

    /// Confere o limite de comprimento e a quantidade observada no erro.
    #[test]
    fn texto_alfanumerico_informa_quando_excede_o_limite() {
        assert_eq!(
            validar_alfanumerico_com_comprimento_maximo::<4>("ABCDE"),
            Err(ErroFormatoTexto::ComprimentoMaximo {
                maximo: 4,
                encontrado: 5,
            })
        );
    }
}

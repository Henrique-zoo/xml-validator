//! Enumerações numéricas usadas pelo XML SDP.
//!
//! Os discriminantes correspondem aos códigos transmitidos no documento. As
//! implementações de [`TryFrom<u32>`] rejeitam códigos desconhecidos,
//! [`std::fmt::Display`] produz descrições próprias para diagnósticos e
//! `Serialize_repr` escreve novamente o valor numérico esperado pelo SDP.
//!
//! [`NavegacaoAtracacao`](crate::xml::models::enums::NavegacaoAtracacao) admite somente 1 a 5; [`NavegacaoCarga`](crate::xml::models::enums::NavegacaoCarga) também
//! representa o código 0. Ausência da tag da carga é um `Option` no modelo,
//! não a variante de código zero. Compatibilidade entre campos pertence a
//! [`crate::validation`], não às conversões numéricas deste módulo.

use std::fmt::{self, Display};

use serde_repr::{Deserialize_repr, Serialize_repr};

/// Nacionalidade do armador responsável pela embarcação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u32)]
pub enum NacionalidadeArmador {
    /// Armador brasileiro, identificado por CNPJ (`1`).
    Brasileiro = 1,
    /// Armador estrangeiro, identificado pelo código cadastral do SDP (`2`).
    Estrangeiro = 2,
}

impl Display for NacionalidadeArmador {
    /// Escreve o nome legível da variante e seu código SDP, sem serializar XML.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Brasileiro => "Brasileiro (1)",
            Self::Estrangeiro => "Estrangeiro (2)",
        })
    }
}

/// Sentido em que uma carga foi movimentada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize_repr, Serialize_repr)]
#[repr(u32)]
pub enum Sentido {
    /// O sentido não se aplica à operação de abastecimento (`0`).
    #[default]
    NaoInformado = 0,
    /// Carga desembarcada da embarcação (`1`).
    Desembarque = 1,
    /// Carga embarcada na embarcação (`2`).
    Embarque = 2,
}

impl Display for Sentido {
    /// Escreve o nome legível da variante e seu código SDP, sem serializar XML.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NaoInformado => "Não se aplica (0)",
            Self::Desembarque => "Desembarque (1)",
            Self::Embarque => "Embarque (2)",
        })
    }
}

/// Natureza física da carga movimentada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u32)]
pub enum NaturezaCarga {
    /// Granel sólido (`1`).
    GranelSolido = 1,
    /// Granel líquido (`2`).
    GranelLiquido = 2,
    /// Carga geral, solta ou conteinerizada (`3`).
    CargaGeral = 3,
}

impl Display for NaturezaCarga {
    /// Escreve o nome legível da variante e seu código SDP, sem serializar XML.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::GranelSolido => "Granel sólido (1)",
            Self::GranelLiquido => "Granel líquido (2)",
            Self::CargaGeral => "Carga geral (3)",
        })
    }
}

/// Modalidade de navegação declarada na carga, incluindo a sentinela zero.
///
/// A existência de [`Self::NaoInformada`] permite decodificar o leiaute, mas
/// não torna zero válido em toda operação. Essa aplicabilidade é semântica.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u32)]
pub enum NavegacaoCarga {
    /// Modalidade não aplicável à operação (`0`).
    NaoInformada = 0,
    /// Navegação interior (`1`).
    NavegacaoInterior = 1,
    /// Apoio portuário (`2`).
    ApoioPortuario = 2,
    /// Cabotagem (`3`).
    Cabotagem = 3,
    /// Apoio marítimo (`4`).
    ApoioMaritimo = 4,
    /// Navegação de longo curso (`5`).
    LongoCurso = 5,
}

impl Display for NavegacaoCarga {
    /// Escreve o nome legível da variante e seu código SDP, sem serializar XML.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NaoInformada => "Não se aplica (0)",
            Self::NavegacaoInterior => "Navegação interior (1)",
            Self::ApoioPortuario => "Apoio portuário (2)",
            Self::Cabotagem => "Cabotagem (3)",
            Self::ApoioMaritimo => "Apoio marítimo (4)",
            Self::LongoCurso => "Longo curso (5)",
        })
    }
}

/// Modalidade de navegação da embarcação informada na atracação.
///
/// Ao contrário de [`NavegacaoCarga`], não representa ausência nem código zero.
/// A conversão usada pelo reader rejeita valores fora de 1 a 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u32)]
pub enum NavegacaoAtracacao {
    /// Navegação interior (`1`).
    NavegacaoInterior = 1,
    /// Apoio portuário (`2`).
    ApoioPortuario = 2,
    /// Cabotagem (`3`).
    Cabotagem = 3,
    /// Apoio marítimo (`4`).
    ApoioMaritimo = 4,
    /// Navegação de longo curso (`5`).
    LongoCurso = 5,
}

impl PartialEq<NavegacaoCarga> for NavegacaoAtracacao {
    /// Compara modalidades equivalentes; a sentinela da carga nunca é igual.
    fn eq(&self, other: &NavegacaoCarga) -> bool {
        matches!(
            (self, other),
            (
                NavegacaoAtracacao::NavegacaoInterior,
                NavegacaoCarga::NavegacaoInterior
            ) | (
                NavegacaoAtracacao::ApoioPortuario,
                NavegacaoCarga::ApoioPortuario
            ) | (NavegacaoAtracacao::Cabotagem, NavegacaoCarga::Cabotagem)
                | (
                    NavegacaoAtracacao::ApoioMaritimo,
                    NavegacaoCarga::ApoioMaritimo
                )
                | (NavegacaoAtracacao::LongoCurso, NavegacaoCarga::LongoCurso)
        )
    }
}

impl TryFrom<NavegacaoCarga> for NavegacaoAtracacao {
    /// Valor de origem que não possui correspondente no tipo da atracação.
    type Error = NavegacaoCarga;

    /// Converte uma modalidade efetivamente informada, preservando seu código.
    ///
    /// # Errors
    ///
    /// Devolve [`NavegacaoCarga::NaoInformada`] quando a entrada é zero.
    fn try_from(value: NavegacaoCarga) -> Result<Self, Self::Error> {
        match value {
            NavegacaoCarga::NaoInformada => Err(value),
            NavegacaoCarga::NavegacaoInterior => Ok(NavegacaoAtracacao::NavegacaoInterior),
            NavegacaoCarga::ApoioPortuario => Ok(NavegacaoAtracacao::ApoioPortuario),
            NavegacaoCarga::Cabotagem => Ok(NavegacaoAtracacao::Cabotagem),
            NavegacaoCarga::ApoioMaritimo => Ok(NavegacaoAtracacao::ApoioMaritimo),
            NavegacaoCarga::LongoCurso => Ok(NavegacaoAtracacao::LongoCurso),
        }
    }
}

impl Display for NavegacaoAtracacao {
    /// Escreve o nome legível da variante e seu código SDP, sem serializar XML.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NavegacaoInterior => "Navegação interior (1)",
            Self::ApoioPortuario => "Apoio portuário (2)",
            Self::Cabotagem => "Cabotagem (3)",
            Self::ApoioMaritimo => "Apoio marítimo (4)",
            Self::LongoCurso => "Longo curso (5)",
        })
    }
}

/// Tipo principal de operação realizada durante a atracação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u32)]
pub enum TipoOperacao {
    /// Movimentação de carga (`1`).
    MovimentacaoDeCarga = 1,
    /// Movimentação de passageiros (`2`).
    Passageiros = 2,
    /// Operação de apoio (`3`).
    Apoio = 3,
    /// Operação da Marinha (`4`).
    Marinha = 4,
    /// Abastecimento da embarcação (`5`).
    Abastecimento = 5,
    /// Reparo ou manutenção (`6`).
    ReparoManutencao = 6,
    /// Operação mista (`7`).
    Misto = 7,
    /// Retirada de resíduos (`8`).
    RetiradaDeResiduos = 8,
}

impl Display for TipoOperacao {
    /// Escreve o nome legível da variante e seu código SDP, sem serializar XML.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MovimentacaoDeCarga => "Movimentação de carga (1)",
            Self::Passageiros => "Passageiros (2)",
            Self::Apoio => "Apoio (3)",
            Self::Marinha => "Marinha (4)",
            Self::Abastecimento => "Abastecimento (5)",
            Self::ReparoManutencao => "Reparo/manutenção (6)",
            Self::Misto => "Misto (7)",
            Self::RetiradaDeResiduos => "Retirada de resíduos (8)",
        })
    }
}

/// Tipo de operação aplicado especificamente a uma carga.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u32)]
pub enum TipoOperacaoCarga {
    /// Apoio (`3`).
    Apoio = 3,
    /// Abastecimento (`5`).
    Abastecimento = 5,
    /// Safamento (`8`).
    Safamento = 8,
    /// Remoção a bordo (`10`).
    RemocaoABordo = 10,
    /// Operação intermediária (`11`).
    OperacaoIntermediaria = 11,
    /// Transferência interna (`12`).
    TransferenciaInterna = 12,
    /// Exportação em navegação de longo curso (`13`).
    LongoCursoExportacao = 13,
    /// Importação em navegação de longo curso (`14`).
    LongoCursoImportacao = 14,
    /// Exportação de longo curso com baldeação de carga estrangeira (`15`).
    LongoCursoExportacaoComBaldeacaoDeCargaEstrangeira = 15,
    /// Importação de longo curso com baldeação de carga estrangeira (`16`).
    LongoCursoImportacaoComBaldeacaoDeCargaEstrangeira = 16,
    /// Cabotagem (`17`).
    Cabotagem = 17,
    /// Navegação interior (`18`).
    Interior = 18,
    /// Baldeação de carga nacional (`19`).
    BaldeacaoDeCargaNacional = 19,
    /// Baldeação de carga estrangeira apenas de passagem (`20`).
    BaldeacaoDeCargaEstrangeiraDePassagem = 20,
}

impl TipoOperacaoCarga {
    /// Indica se a operação pertence ao conjunto principal, de códigos 13 a 20.
    ///
    /// Não verifica a compatibilidade com o tipo de operação da atracação.
    pub fn eh_principal(&self) -> bool {
        match self {
            Self::Apoio
            | Self::Abastecimento
            | Self::Safamento
            | Self::RemocaoABordo
            | Self::OperacaoIntermediaria
            | Self::TransferenciaInterna => false,
            _ => true,
        }
    }

    /// Indica uma operação auxiliar: apoio, abastecimento, safamento, remoção
    /// a bordo, operação intermediária ou transferência interna.
    pub fn eh_auxiliar(&self) -> bool {
        !self.eh_principal()
    }
}

impl PartialEq<TipoOperacao> for TipoOperacaoCarga {
    /// Reconhece somente os pares homônimos apoio e abastecimento.
    ///
    /// Não compara os discriminantes isoladamente: código 8 significa
    /// safamento na carga e retirada de resíduos na atracação.
    fn eq(&self, other: &TipoOperacao) -> bool {
        matches!(
            (self, other),
            (Self::Abastecimento, TipoOperacao::Abastecimento) | (Self::Apoio, TipoOperacao::Apoio)
        )
    }
}

impl TryFrom<TipoOperacao> for TipoOperacaoCarga {
    /// Operação da atracação que não possui uma conversão unívoca para carga.
    type Error = TipoOperacao;

    /// Converte apoio e abastecimento para a operação de carga homônima.
    ///
    /// # Errors
    ///
    /// Devolve a entrada para qualquer outra operação; movimentação e misto
    /// permitem várias operações de carga e não selecionam uma automaticamente.
    fn try_from(value: TipoOperacao) -> Result<Self, Self::Error> {
        match value {
            TipoOperacao::Abastecimento => Ok(TipoOperacaoCarga::Abastecimento),
            TipoOperacao::Apoio => Ok(TipoOperacaoCarga::Apoio),
            _ => Err(value),
        }
    }
}

impl Display for TipoOperacaoCarga {
    /// Escreve o nome legível da variante e seu código SDP, sem serializar XML.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Apoio => formatter.write_str("Apoio (3)"),
            Self::Abastecimento => formatter.write_str("Abastecimento (5)"),
            Self::Safamento => formatter.write_str("Safamento (8)"),
            Self::RemocaoABordo => formatter.write_str("Remoção a bordo (10)"),
            Self::OperacaoIntermediaria => formatter.write_str("Operação intermediária (11)"),
            Self::TransferenciaInterna => formatter.write_str("Transferência interna (12)"),
            Self::LongoCursoExportacao => formatter.write_str("Longo curso exportação (13)"),
            Self::LongoCursoImportacao => formatter.write_str("Longo curso importação (14)"),
            Self::LongoCursoExportacaoComBaldeacaoDeCargaEstrangeira => formatter
                .write_str("Longo curso exportação com baldeação de carga estrangeira (15)"),
            Self::LongoCursoImportacaoComBaldeacaoDeCargaEstrangeira => formatter
                .write_str("Longo curso importação com baldeação de carga estrangeira (16)"),
            Self::Cabotagem => formatter.write_str("Cabotagem (17)"),
            Self::Interior => formatter.write_str("Interior (18)"),
            Self::BaldeacaoDeCargaNacional => {
                formatter.write_str("Baldeação de carga nacional (19)")
            }
            Self::BaldeacaoDeCargaEstrangeiraDePassagem => {
                formatter.write_str("Baldeação de carga estrangeira de passagem (20)")
            }
        }
    }
}

/// Implementa a conversão validada dos códigos numéricos do SDP.
macro_rules! impl_try_from_u32 {
    ($type:ty { $($value:literal => $variant:path),+ $(,)? }) => {
        impl TryFrom<u32> for $type {
            /// Código desconhecido, sem informação adicional ao valor de entrada.
            type Error = ();

            /// Converte exclusivamente os discriminantes declarados nesta enum.
            ///
            /// # Errors
            ///
            /// Retorna `Err(())` para qualquer inteiro sem variante correspondente.
            fn try_from(value: u32) -> Result<Self, Self::Error> {
                match value {
                    $($value => Ok($variant),)+
                    _ => Err(()),
                }
            }
        }
    };
}

impl_try_from_u32!(NacionalidadeArmador {
    1 => NacionalidadeArmador::Brasileiro,
    2 => NacionalidadeArmador::Estrangeiro,
});

impl_try_from_u32!(Sentido {
    0 => Sentido::NaoInformado,
    1 => Sentido::Desembarque,
    2 => Sentido::Embarque,
});

impl_try_from_u32!(NaturezaCarga {
    1 => NaturezaCarga::GranelSolido,
    2 => NaturezaCarga::GranelLiquido,
    3 => NaturezaCarga::CargaGeral,
});

impl_try_from_u32!(NavegacaoCarga {
    0 => NavegacaoCarga::NaoInformada,
    1 => NavegacaoCarga::NavegacaoInterior,
    2 => NavegacaoCarga::ApoioPortuario,
    3 => NavegacaoCarga::Cabotagem,
    4 => NavegacaoCarga::ApoioMaritimo,
    5 => NavegacaoCarga::LongoCurso,
});

impl_try_from_u32!(NavegacaoAtracacao {
    1 => NavegacaoAtracacao::NavegacaoInterior,
    2 => NavegacaoAtracacao::ApoioPortuario,
    3 => NavegacaoAtracacao::Cabotagem,
    4 => NavegacaoAtracacao::ApoioMaritimo,
    5 => NavegacaoAtracacao::LongoCurso,
});

impl_try_from_u32!(TipoOperacao {
    1 => TipoOperacao::MovimentacaoDeCarga,
    2 => TipoOperacao::Passageiros,
    3 => TipoOperacao::Apoio,
    4 => TipoOperacao::Marinha,
    5 => TipoOperacao::Abastecimento,
    6 => TipoOperacao::ReparoManutencao,
    7 => TipoOperacao::Misto,
    8 => TipoOperacao::RetiradaDeResiduos,
});

impl_try_from_u32!(TipoOperacaoCarga {
    3 => TipoOperacaoCarga::Apoio,
    5 => TipoOperacaoCarga::Abastecimento,
    8 => TipoOperacaoCarga::Safamento,
    10 => TipoOperacaoCarga::RemocaoABordo,
    11 => TipoOperacaoCarga::OperacaoIntermediaria,
    12 => TipoOperacaoCarga::TransferenciaInterna,
    13 => TipoOperacaoCarga::LongoCursoExportacao,
    14 => TipoOperacaoCarga::LongoCursoImportacao,
    15 => TipoOperacaoCarga::LongoCursoExportacaoComBaldeacaoDeCargaEstrangeira,
    16 => TipoOperacaoCarga::LongoCursoImportacaoComBaldeacaoDeCargaEstrangeira,
    17 => TipoOperacaoCarga::Cabotagem,
    18 => TipoOperacaoCarga::Interior,
    19 => TipoOperacaoCarga::BaldeacaoDeCargaNacional,
    20 => TipoOperacaoCarga::BaldeacaoDeCargaEstrangeiraDePassagem,
});

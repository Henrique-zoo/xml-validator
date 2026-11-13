//! Catálogos imutáveis incorporados ao executável.
//!
//! Esta camada decodifica os arquivos de referência uma única vez ao construir
//! [`DadosReferencia`](crate::reference_data::DadosReferencia). A validação consulta somente os índices em memória e
//! não realiza I/O durante o processamento dos documentos XML.
//!
//! O CSV é incluído na compilação. O catálogo armazena cada código como um
//! array `[u8; 4]` de bytes ASCII, de posse do conjunto e independente do interner.
//! Cada consulta aceita `&str`, sem alocação. Não há atualização automática nem
//! garantia de equivalência com o cadastro vigente da ANTAQ.

use std::{
    collections::{HashMap, HashSet},
    fmt::{self, Display},
};

/// Conjunto pesquisável dos códigos ISO de contêiner aceitos pelo SDP.
///
/// Contém somente os códigos do recurso distribuído com esta versão. Cada
/// código é um array `[u8; 4]` com quatro bytes ASCII, armazenado diretamente no
/// conjunto, sem alocação individual nem empréstimo dos registros do leitor CSV.
/// A tabela hash possui seu próprio custo de alocação e de armazenamento; os
/// quatro bytes representam apenas o tamanho de cada chave.
pub struct CatalogoCodigosConteiner {
    /// Códigos distintos de quatro bytes ASCII, comparados sem normalizar caixa.
    codigos: HashSet<[u8; 4]>,
}

/// Conjuntos de referência imutáveis usados pela validação semântica.
///
/// O agrupamento mantém a API do validador estável quando novos catálogos
/// incorporados forem adicionados ao projeto.
pub struct DadosReferencia {
    /// Códigos ISO que identificam cargas conteinerizadas.
    codigos_conteiner: CatalogoCodigosConteiner,
}

impl DadosReferencia {
    /// Carrega todos os conjuntos de referência incorporados ao executável.
    ///
    /// # Errors
    ///
    /// Propaga a primeira falha de estrutura ou conteúdo de um catálogo. Não
    /// realiza acesso ao sistema de arquivos nem à rede.
    pub fn incorporados() -> Result<Self, CatalogoError> {
        Ok(Self {
            codigos_conteiner: CatalogoCodigosConteiner::incorporado()?,
        })
    }

    /// Informa se `codigo` consta no catálogo ISO de contêineres.
    pub(crate) fn codigo_conteiner_existe(&self, codigo: &str) -> bool {
        self.codigos_conteiner.contem(codigo)
    }
}

/// Falha ao construir o catálogo de códigos ISO de contêiner.
#[derive(Debug)]
pub enum CatalogoError {
    /// O conteúdo do catálogo não pôde ser interpretado como CSV.
    Csv(csv::Error),
    /// A coluna necessária não existe no cabeçalho.
    ColunaAusente {
        /// Cabeçalho esperado, comparado literalmente.
        nome: &'static str,
    },
    /// Uma linha possui a coluna de código, mas seu valor está vazio.
    CodigoVazio {
        /// Linha física do registro, iniciada em 1 e contando o cabeçalho.
        linha: u64,
    },
    /// O código não possui o formato lexical esperado.
    CodigoInvalido {
        /// Linha física do registro inválido, iniciada em 1.
        linha: u64,
        /// Código preservado para explicar a falha, sem normalização.
        codigo: String,
        /// Restrição lexical que o código violou.
        motivo: ErroCodigoConteiner,
    },
    /// O mesmo código apareceu mais de uma vez.
    CodigoDuplicado {
        /// Código encontrado nas duas ocorrências.
        codigo: String,
        /// Linha física em que o código foi registrado inicialmente.
        primeira_linha: u64,
        /// Linha física em que a repetição foi detectada.
        segunda_linha: u64,
    },
    /// O CSV possui cabeçalho, mas nenhum código.
    CatalogoVazio,
}

impl Display for CatalogoError {
    /// Descreve a falha do CSV, incluindo linha e coluna quando disponíveis.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Csv(error) => {
                write!(
                    formatter,
                    "não foi possível interpretar o catálogo como CSV: {error}"
                )
            }
            Self::ColunaAusente { nome } => {
                write!(
                    formatter,
                    "o catálogo não contém a coluna obrigatória `{nome}`"
                )
            }
            Self::CodigoVazio { linha } => {
                write!(
                    formatter,
                    "a linha {linha} não informa um código de contêiner"
                )
            }
            Self::CodigoInvalido {
                linha,
                codigo,
                motivo,
            } => write!(
                formatter,
                "código de contêiner `{codigo}` inválido na linha {linha}: {motivo}"
            ),
            Self::CodigoDuplicado {
                codigo,
                primeira_linha,
                segunda_linha,
            } => write!(
                formatter,
                "o código de contêiner `{codigo}` está duplicado nas linhas {primeira_linha} e {segunda_linha}"
            ),
            Self::CatalogoVazio => {
                formatter.write_str("o catálogo não contém nenhum código de contêiner")
            }
        }
    }
}

impl std::error::Error for CatalogoError {
    /// Expõe a falha CSV ou lexical subjacente; erros sem causa aninhada retornam `None`.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Csv(error) => Some(error),
            Self::CodigoInvalido { motivo, .. } => Some(motivo),
            _ => None,
        }
    }
}

/// Motivo pelo qual um código do catálogo não possui o formato esperado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErroCodigoConteiner {
    /// O código não contém exatamente quatro caracteres.
    Comprimento {
        /// Número de caracteres Unicode observado, não de bytes.
        encontrado: usize,
    },

    /// Um caractere não é um algarismo ASCII nem uma letra ASCII maiúscula.
    CaractereInvalido {
        /// Posição do primeiro caractere inválido, iniciada em 1.
        posicao: usize,
        /// Caractere original rejeitado.
        encontrado: char,
    },
}

impl Display for ErroCodigoConteiner {
    /// Descreve o comprimento ou o caractere que invalida o código ISO.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Comprimento { encontrado } => write!(
                formatter,
                "o código deve conter exatamente quatro caracteres; foram encontrados {encontrado}"
            ),
            Self::CaractereInvalido {
                posicao,
                encontrado,
            } => write!(
                formatter,
                "o caractere `{encontrado}` na posição {posicao} deve ser um algarismo ASCII ou uma letra ASCII maiúscula"
            ),
        }
    }
}

impl std::error::Error for ErroCodigoConteiner {}

impl CatalogoCodigosConteiner {
    /// Conteúdo UTF-8 incorporado; não é lido do disco em tempo de execução.
    const CSV: &str = include_str!("../resources/isocode_conteiner.csv");
    /// Nome exato da coluna que contém os códigos consultados pela validação.
    const COLUNA_CODIGO: &str = "Código Posição SH4";

    /// Constrói o catálogo a partir do CSV incorporado ao executável.
    ///
    /// # Errors
    ///
    /// Retorna erros de estrutura CSV ou de conteúdo. O catálogo é reconstruído
    /// a cada chamada; reutilize [`DadosReferencia`] ou o validador entre arquivos.
    pub fn incorporado() -> Result<Self, CatalogoError> {
        Self::from_csv(Self::CSV)
    }

    /// Decodifica e valida o conteúdo textual de um catálogo CSV.
    ///
    /// Usa ponto e vírgula como separador, exige cabeçalho e largura uniforme
    /// dos registros. Os códigos não são aparados nem convertidos para maiúsculas.
    ///
    /// # Errors
    ///
    /// Interrompe na primeira falha: CSV inválido, coluna ausente, código vazio,
    /// formato inválido, duplicidade ou catálogo sem registros. As demais colunas
    /// são lidas pelo parser CSV, mas seu conteúdo não é validado.
    fn from_csv(csv: &str) -> Result<Self, CatalogoError> {
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(b';')
            .has_headers(true)
            .flexible(false)
            .from_reader(csv.as_bytes());

        let indice_codigo = reader
            .headers()
            .map_err(CatalogoError::Csv)?
            .iter()
            .position(|cabecalho| cabecalho == Self::COLUNA_CODIGO)
            .ok_or(CatalogoError::ColunaAusente {
                nome: Self::COLUNA_CODIGO,
            })?;

        let mut encontrados: HashMap<[u8; 4], u64> = HashMap::new();

        for (indice, resultado) in reader.records().enumerate() {
            let registro = resultado.map_err(CatalogoError::Csv)?;
            let linha = registro
                .position()
                .map_or(indice as u64 + 2, |posicao| posicao.line());

            let Some(codigo) = registro.get(indice_codigo) else {
                return Err(CatalogoError::CodigoVazio { linha });
            };

            if codigo.is_empty() {
                return Err(CatalogoError::CodigoVazio { linha });
            }

            if let Err(motivo) = validar_formato_codigo_conteiner(codigo) {
                return Err(CatalogoError::CodigoInvalido {
                    linha,
                    codigo: codigo.to_owned(),
                    motivo,
                });
            }

            if let Some(&primeira_linha) = encontrados.get(codigo.as_bytes()) {
                return Err(CatalogoError::CodigoDuplicado {
                    codigo: codigo.to_owned(),
                    primeira_linha,
                    segunda_linha: linha,
                });
            }

            encontrados.insert(
                <[u8; 4]>::try_from(codigo.as_bytes())
                    .expect("o código já foi validado como quatro caracteres ASCII"),
                linha,
            );
        }

        if encontrados.is_empty() {
            return Err(CatalogoError::CatalogoVazio);
        }

        Ok(Self {
            codigos: encontrados.into_keys().collect(),
        })
    }

    /// Informa se `codigo` consta no catálogo.
    ///
    /// A consulta tem custo médio constante no número de entradas, além do hash
    /// do texto recebido, e não aloca nem valida o formato de códigos ausentes.
    pub fn contem(&self, codigo: &str) -> bool {
        self.codigos.contains(codigo.as_bytes())
    }

    /// Retorna a quantidade de códigos distintos do catálogo.
    pub fn len(&self) -> usize {
        self.codigos.len()
    }

    /// Informa se o catálogo não possui nenhum código.
    pub fn is_empty(&self) -> bool {
        self.codigos.is_empty()
    }
}

/// Verifica quatro caracteres ASCII, cada um algarismo ou letra maiúscula.
///
/// # Errors
///
/// Retorna primeiro o comprimento incorreto; se ele for válido, retorna a
/// posição do primeiro caractere inválido. Não consulta o catálogo ISO.
fn validar_formato_codigo_conteiner(codigo: &str) -> Result<(), ErroCodigoConteiner> {
    let comprimento = codigo.chars().count();

    if comprimento != 4 {
        return Err(ErroCodigoConteiner::Comprimento {
            encontrado: comprimento,
        });
    }

    for (indice, caractere) in codigo.chars().enumerate() {
        if !(caractere.is_ascii_digit() || caractere.is_ascii_uppercase()) {
            return Err(ErroCodigoConteiner::CaractereInvalido {
                posicao: indice + 1,
                encontrado: caractere,
            });
        }
    }

    Ok(())
}

/// Testes de carga do catálogo incorporado, consulta e causas de erro.
#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    /// Exercita a leitura do CSV incorporado e a consulta de códigos conhecidos.
    #[test]
    fn carrega_o_catalogo_incorporado() {
        let catalogo = CatalogoCodigosConteiner::incorporado()
            .expect("o catálogo distribuído com o programa deve ser válido");

        assert_eq!(catalogo.len(), 107);
        assert!(!catalogo.is_empty());
        assert!(catalogo.contem("22G1"));
        assert!(!catalogo.contem("ZZZZ"));
    }

    /// Confere a consulta ao catálogo pela fachada de dados de referência.
    #[test]
    fn agrega_o_catalogo_nos_dados_de_referencia() {
        let dados = DadosReferencia::incorporados()
            .expect("os dados distribuídos com o programa devem ser válidos");

        assert!(dados.codigo_conteiner_existe("22G1"));
        assert!(!dados.codigo_conteiner_existe("22Z9"));
    }

    /// Confere a mensagem do erro de catálogo e o encadeamento da causa lexical.
    #[test]
    fn descreve_codigo_invalido_e_preserva_sua_causa() {
        let erro = CatalogoError::CodigoInvalido {
            linha: 7,
            codigo: String::from("22-g"),
            motivo: ErroCodigoConteiner::CaractereInvalido {
                posicao: 3,
                encontrado: '-',
            },
        };

        assert_eq!(
            erro.to_string(),
            "código de contêiner `22-g` inválido na linha 7: o caractere `-` na posição 3 deve ser um algarismo ASCII ou uma letra ASCII maiúscula"
        );
        assert!(erro.source().is_some());
    }
}

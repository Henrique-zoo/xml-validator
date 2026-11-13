//! Validação semântica dos dados já decodificados do XML.
//!
//! Este módulo não verifica se o XML está bem-formado nem sua estrutura. Ele
//! recebe o resultado do reader e aplica as regras semânticas e de adequação
//! ao leiaute que dependem dos valores decodificados, acumulando todos os
//! problemas encontrados.
//!
//! # Organização das regras
//!
//! [`Validador::validar_atracacoes`](crate::validation::Validador::validar_atracacoes)
//! cria um `ValidationContext` interno e
//! percorre o modelo junto aos mapas tipados produzidos pelo reader. Funções
//! de entidade, como `atracacao::validar_atracacao`, orquestram classes de
//! regras. Enquanto uma classe for pequena e exclusiva da entidade, sua função
//! agregadora e suas regras específicas permanecem no mesmo módulo da entidade.
//!
//! A unidade de responsabilidade é a regra de negócio, não cada campo isolado
//! nem cada código de operação. A orquestração decide a aplicabilidade com os
//! dados declarados; os validadores específicos verificam as condições de sua
//! regra. Um erro não desliga regras independentes. Proibições conjuntas têm
//! um único responsável e podem gerar um diagnóstico com várias labels.
//!
//! As assinaturas seguem três níveis, sem exigir uma interface uniforme:
//!
//! - **Orquestração de entidade ou classe:** recebe modelo e mapa, seleciona
//!   regras aplicáveis, extrai campos opcionais e compartilha cálculos;
//! - **Regra específica com diagnóstico:** recebe o contexto e somente os
//!   valores e IDs necessários àquela regra. Um ID do pai pode ser necessário
//!   para apontar onde inserir um campo ausente;
//! - **Helper puro:** recebe valores e devolve um resultado, sem depender de
//!   contexto, modelo XML ou posições na fonte.
//!
//! Regras que cruzam muitos campos, como cronologia e passageiros, podem
//! conservar o modelo e o mapa para evitar listas extensas de argumentos.
//! As regras de presença também podem receber IDs opcionais: ausência é o
//! próprio dado que elas verificam. Não se deve descartá-la na orquestração.
//! Já a validação lexical de um campo opcional só é chamada quando seu valor
//! e seu ID existem. Essa seleção não suprime outros erros independentes.
//!
//! A especialização explicita dependências; não muda as regras nem comprova
//! ganho de desempenho. Os modelos continuam emprestados, não clonados. Na
//! carga, a consulta ao catálogo é compartilhada pela classificação pura e
//! pela verificação do código, evitando repetir a mesma busca.
//!
//! ```text
//! validar_atracacoes
//! ├── validar_atracacao
//! │   └── validar_conformidade_leiaute
//! │       └── validar_conformidade_identificador_instalacao
//! │           ├── compatibilidade da tag com o leiaute
//! │           ├── presença do identificador exigido
//! │           └── formato do identificador existente
//! └── para cada carga
//!     ├── validar_carga
//!     │   ├── regras independentes de CNPJ e NCM
//!     │   ├── validar_regras_da_operacao
//!     │   ├── validar_movimentacao
//!     │   │   ├── omissão conjunta dos campos, quando proibidos
//!     │   │   └── navegação e instalações, quando aplicáveis
//!     │   └── validar_representacao_carga
//!     └── para cada detalhamento: validar_carga_conteinerizada
//! ```
//!
//! Regras não recebem [`ParsedAtracacoes`](crate::xml::reader::ParsedAtracacoes) diretamente. O contexto fornece
//! somente o leiaute, o catálogo local, a resolução `ElementId → Span` e a acumulação de erros,
//! reduzindo o acoplamento com o reader.
//!
//! # Fonte e convenções dos checklists
//!
//! As regras documentadas nesta camada derivam do *Manual do Usuário Externo
//! do sistema SDP*, 5ª edição, versão 1.0, dezembro de 2025. Cada módulo indica
//! as seções e páginas usadas. `[x]` identifica uma regra já alcançada pela
//! orquestração; `[ ]`, uma regra ainda pendente. Observações junto aos itens
//! distinguem garantias do reader, adaptações do projeto e revisões ainda não
//! executadas. Um checklist descreve o trabalho, não configura a execução.
//!
//! Os checklists contêm somente regras verificáveis com o próprio documento e
//! conjuntos fechados incorporados ao programa. Exigências do Manual que
//! dependeriam da base operacional ou das tabelas vigentes da ANTAQ ficam fora
//! do contrato deste validador e não aparecem como tarefas pendentes.
//! Restrições que não aparecem no Manual também não são mantidas como tarefas
//! implícitas.
//!
//! No nível do documento, o Manual estabelece:
//!
//! - [x] rejeitar, ao menos dentro do arquivo recebido, outro número de
//!   atracação igual para o mesmo ano e local de atracação (Manual §2.2.1,
//!   pp. 9–10);
//! - [x] consolidar em um único registro as cargas de uma mesma atracação com
//!   características idênticas, salvo o peso, somando os pesos (Manual §2.1,
//!   p. 9).
//!
//! O Manual não define neste contexto unicidade de `NREscala`, igualdade do
//! identificador informante entre todas as atracações, proibição de documento
//! sem atracações ou duplicidade integral de atracações. Essas hipóteses foram
//! removidas do checklist até que outra fonte normativa seja adotada.
//!
//! XML bem-formado, namespace reconhecido, cardinalidade, enumerações
//! numéricas e conversão lexical pertencem a `xml::reader`. O reader também
//! exige as tags comuns e incondicionais. A presença de uma tag que depende do
//! leiaute ou de outro campo pertence a esta camada, pois requer contexto
//! semântico para decidir qual elemento é obrigatório.
//!
//! # Limites dos dados de referência
//!
//! A validação é deliberadamente local: não consulta bancos de dados, arquivos
//! auxiliares, APIs nem serviços externos. Ela percorre os dados já mantidos em
//! memória e verifica formatos, relações entre campos e conjuntos fechados
//! incorporados à versão suportada do leiaute. Consequentemente, não afirma
//! que porto, TUP, local, área, NCM, IMDG ou outro identificador exista em um
//! cadastro oficial vigente. Essa limitação é parte permanente do contrato,
//! preservando um processamento previsível e adequado a XMLs grandes.
//!
//! # Diagnósticos
//!
//! Os enunciados gerais ficam em [`regras_de_negocio`](crate::validation::regras_de_negocio), separados da execução.
//! `details` empresta o texto estático da regra; mensagem e labels descrevem a
//! ocorrência concreta. As constantes registram fontes e adaptações do projeto,
//! sem conectar automaticamente as regras pendentes dos checklists.
//!
//! Um [`ValidationError`](crate::validation::ValidationError) contém mensagem, detalhes e labels de alvos distintos.
//! [`ValidationError::relacionados`](crate::validation::ValidationError::relacionados) agrupa problemas simultâneos;
//! [`ValidationError::alternativas`](crate::validation::ValidationError::alternativas) contém interpretações mutuamente exclusivas.
//! A GUI pode expandir o ramo escolhido sem tratar todos os ramos como erros
//! cumulativos. As regras não recebem confiança nem IDs causais: um contexto
//! coletor executa regras usuais dentro de cada ramo apenas quando necessário.
//!
//! Declarações explícitas de contêiner selecionam suas regras mesmo que outros
//! campos estejam inválidos. A ausência de um dado suspende apenas as regras
//! que precisam dele. O reader ainda interrompe no primeiro erro estrutural
//! ou de conversão; a acumulação ocorre após a decodificação bem-sucedida.

/// Regras semânticas aplicáveis a uma atracação.
mod atracacao;
/// Regras lexicais com diagnósticos reutilizadas entre entidades.
mod campos;
/// Regras semânticas aplicáveis a uma carga.
mod carga;
/// Regras semânticas aplicáveis ao detalhamento de um contêiner.
mod carga_conteinerizada;
/// Estado e serviços compartilhados pelas regras.
mod context;
/// Unicidade, consolidação e limites que abrangem vários registros.
mod documento;
/// Diagnósticos produzidos pelas regras semânticas.
mod error;
/// Funções puras compartilhadas por regras de diferentes entidades.
mod helper;
/// Enunciados compartilhados das regras, consultáveis pela CLI e por interfaces.
pub mod regras_de_negocio;
/// Certificados de retirada e itens de resíduos.
mod residuos;
/// Regras semânticas aplicáveis a um intervalo de tempo descontado.
mod tempos_descontos;

pub use error::{AlternativaDiagnostico, DiagnosticoColorido, ValidationError, ValidationLabel};

use atracacao::{InstalacaoInformante, TipoOperacaoAtracacao, validar_atracacao};
use carga::validar_carga;
use carga_conteinerizada::validar_carga_conteinerizada;
use context::ValidationContext;

use crate::{
    interner::Interner,
    reference_data::{CatalogoError, DadosReferencia},
    xml::reader::ParsedAtracacoes,
};

/// Validador semântico configurado com os dados de referência do SDP.
///
/// O objeto possui os catálogos imutáveis e pode ser reutilizado para validar
/// vários documentos sem reconstruí-los a cada arquivo.
pub struct Validador {
    /// Conjuntos locais consultados pelas regras semânticas.
    dados_referencia: DadosReferencia,
}

impl Validador {
    /// Constrói um validador com os catálogos incorporados ao executável.
    ///
    /// # Errors
    ///
    /// Retorna [`CatalogoError`] se algum recurso incorporado estiver
    /// estrutural ou semanticamente inválido.
    pub fn com_dados_incorporados() -> Result<Self, CatalogoError> {
        DadosReferencia::incorporados().map(Self::com_dados_referencia)
    }

    /// Constrói um validador com dados de referência previamente carregados.
    pub const fn com_dados_referencia(dados_referencia: DadosReferencia) -> Self {
        Self { dados_referencia }
    }

    /// Aplica todas as classes de regras às atracações e acumula os erros.
    ///
    /// Este método pressupõe que o reader já verificou a sintaxe e a estrutura
    /// do documento e realizou as conversões dos valores.
    ///
    /// # Errors
    ///
    /// Retorna todos os erros semânticos encontrados. Atualmente isso inclui
    /// tags de instalação incompatíveis com o leiaute, identificadores
    /// obrigatórios, formatos de identificadores e compatibilidade dos
    /// responsáveis informados, além das regras implementadas de operação,
    /// movimentação e representação das cargas. O checklist não é executado
    /// automaticamente: itens pendentes não são regras implícitas.
    ///
    /// `Ok(())` significa apenas que as regras implementadas não encontraram
    /// incompatibilidades; não garante aceitação pelo SDP oficial. Os erros
    /// emprestam o texto original por `'xml`, mas não o validador nem os mapas.
    /// A chamada não altera o modelo e não faz I/O.
    ///
    /// # Panics
    ///
    /// Entra em pânico se os intervalos de cargas ou contêineres estiverem fora
    /// dos mapas, ou se suas quantidades divergirem das do modelo. IDs inválidos
    /// também podem causar pânico. Preserve o alinhamento produzido pelo reader
    /// ao modificar manualmente um [`ParsedAtracacoes`].
    pub fn validar_atracacoes<'xml>(
        &self,
        parsed: &ParsedAtracacoes<'xml>,
        interner: &Interner<'xml>,
    ) -> Result<(), Vec<ValidationError<'xml>>> {
        let mut context = ValidationContext::new(parsed, interner, &self.dados_referencia);

        for (atracacao, atracacao_source_map) in parsed.atracacoes_with_source() {
            validar_atracacao(&mut context, atracacao, atracacao_source_map);
            tempos_descontos::validar_tempos_descontos(
                &mut context,
                atracacao,
                atracacao_source_map,
                &parsed.sdp_source_map.tempos_descontos
                    [atracacao_source_map.tempos_descontos.range()],
            );
            let operacao_atracacao = TipoOperacaoAtracacao {
                tipo_operacao: atracacao.tipo_operacao,
                tipo_operacao_id: atracacao_source_map.tipo_operacao,
            };

            let informante = match (
                atracacao
                    .trigrama_porto
                    .zip(atracacao_source_map.trigrama_porto),
                atracacao.codigo_tup.zip(atracacao_source_map.codigo_tup),
            ) {
                (Some((trigrama, elemento)), None) => {
                    Some(InstalacaoInformante::Porto { trigrama, elemento })
                }
                (None, Some((codigo, elemento))) => {
                    Some(InstalacaoInformante::Tup { codigo, elemento })
                }
                _ => None,
            };

            let cargas_source_map =
                &parsed.sdp_source_map.cargas[atracacao_source_map.cargas.range()];
            assert_eq!(
                atracacao.cargas.len(),
                cargas_source_map.len(),
                "o reader deve manter cargas e mapas de origem alinhados"
            );

            for (carga, carga_source_map) in atracacao.cargas.iter().zip(cargas_source_map) {
                let conteineres = carga.carga_conteinerizada.as_deref().unwrap_or(&[]);
                let conteineres_source_map =
                    &parsed.sdp_source_map.conteineres[carga_source_map.conteineres.range()];
                assert_eq!(
                    conteineres.len(),
                    conteineres_source_map.len(),
                    "o reader deve manter contêineres e mapas de origem alinhados"
                );

                let estado_container = validar_carga(
                    &mut context,
                    informante,
                    operacao_atracacao,
                    carga,
                    carga_source_map,
                    conteineres_source_map,
                );

                for (conteiner, conteiner_source_map) in
                    conteineres.iter().zip(conteineres_source_map)
                {
                    validar_carga_conteinerizada(
                        &mut context,
                        conteiner,
                        conteiner_source_map,
                        estado_container,
                    );
                }
            }
        }

        documento::validar_documento(&mut context, parsed);
        residuos::validar_residuos(&mut context, parsed);
        context.finalizar()
    }
}

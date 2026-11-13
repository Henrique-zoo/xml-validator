//! Regras semânticas aplicáveis a uma ocorrência `<Carga>`.
//!
//! Os enunciados gerais dos diagnósticos são compartilhados por
//! [`crate::validation::regras_de_negocio`]; mensagens e labels descrevem cada ocorrência.
//!
//! # Organização por regra de negócio
//!
//! [`validar_carga`] orquestra verificações independentes e três classes:
//! `validar_regras_da_operacao` relaciona operações, sentido e código;
//! `validar_movimentacao` escolhe entre omitir campos ou validar navegação
//! e instalações; `validar_representacao_carga` seleciona as regras da
//! representação declarada. A classificação é calculada uma única vez.
//!
//! A aplicabilidade vem dos dados, nunca do sucesso de uma validação anterior.
//! Cada regra possui um único responsável: a omissão conjunta gera um único
//! diagnóstico com todas as tags indevidas, e não é repetida pelos validadores
//! de navegação, origem ou destino. Regras independentes continuam acumulando
//! seus próprios erros. Os checklists indicam as garantias locais.
//!
//! As orquestradoras recebem o modelo e seu mapa paralelo; regras locais
//! recebem somente os valores e IDs necessários, além do contexto de diagnóstico.
//! A classificação é pura e recebe o resultado da consulta ao catálogo, que
//! também é usado para validar o código ISO sem uma segunda busca. Regras com
//! muitos campos, como as de instalações e de soma dos detalhamentos, conservam
//! o acesso ao modelo ou ao mapa correspondente. Nenhuma regra corrige valores.
//!
//! # Fonte normativa
//!
//! O checklist deriva do *Manual do Usuário Externo do sistema SDP*, 5ª
//! edição, versão 1.0, dezembro de 2025: seções 2.1 (pp. 8–9), 2.2.4
//! (pp. 13–16), 2.2.5 (pp. 16–18), 3.2 (pp. 20–21) e 10 (pp. 43–45). Regras
//! que exigiriam cadastros auxiliares não pertencem ao validador local.
//!
//! # Checklist normativo
//!
//! ## Operador, mercadoria e classificação — Manual §2.2.4
//!
//! - [x] exigir CNPJ estruturalmente válido em `CnpjOperador`;
//! - [x] reconhecer conteinerização por qualquer declaração explícita:
//!   `CargaConteinerizada`, `CheioVazio` ou código ISO incorporado;
//! - [x] oferecer alternativas somente para código não reconhecido sem
//!   indicadores de representação e fora de operação de apoio;
//! - [x] sem `CheioVazio` e fora de operação de apoio, tratar `CodigoCarga`
//!   como NCMSH4 e exigir exatamente quatro algarismos ASCII;
//! - [x] quando presente, exigir oito algarismos ASCII em `CodigoNCM`, que
//!   representa o código NCM/SH8; -- numéricos
//! - [x] aceitar em `CheioVazio` somente `C` (cheio) ou `V` (vazio); essa
//!   conversão pertence ao reader;
//! - [x] exigir `NaturezaCarga` igual a carga geral (3) para contêineres;
//! - [x] aceitar CE-Mercante opcional de quinze caracteres, conforme a revisão
//!   do projeto, mantendo sua proibição para contêiner vazio;
//!
//! `CodigoNCM` é uma adição posterior à edição do Manual usada como fonte
//! principal deste checklist. Sua semântica NCM/SH8 integra o leiaute corrente
//! adotado pelo projeto.
//!
//! Em operações de apoio, um código presente no catálogo ainda identifica um
//! contêiner. Os demais códigos permanecem opacos: como o Manual não fornece
//! nesse ponto um conjunto fechado nem um formato lexical suficiente, o
//! validador não tenta reconhecer caminhões ou outras cargas de apoio.
//!
//! ## Sentido, navegação, origem e destino — Manual §2.2.4
//!
//! - [x] exigir `Sentido` 0 ou não informado em abastecimento, safamento e remoção a bordo;
//! - [x] exigir `Sentido` 1 (desembarque) ou 2 (embarque) nas demais operações
//!   em que ele se aplica;
//! - [x] exigir navegação 1 a 5 quando aplicável e omitir a tag nas operações
//!   auxiliares enumeradas pela regra textual da seção 2.2.5;
//! - [x] completar a representação de um porto por `BigramaPais` junto de `TrigramaPorto` e uma
//!   instalação privada nacional exclusivamente por `CodigoTUP` de sete
//!   caracteres;
//! - [x] na outra extremidade, rejeitar `CodigoTUP` junto da dupla
//!   `BigramaPais` e `TrigramaPorto`; identificadores incompletos e seus
//!   formatos são verificados independentemente, inclusive em representação parcial;
//! - [x] na extremidade correspondente ao informante (origem no embarque,
//!   destino no desembarque), exigir bigrama e trigrama para porto e omitir
//!   código TUP; para TUP, exigir código TUP e omitir bigrama e trigrama;
//! - [x] comparar o trigrama ou código TUP dessa extremidade com o do informante;
//!   o país dessa extremidade deve ser BR para porto brasileiro;
//! - [x] exigir `CodigoAreaOrigem` para embarque (2) e usar `9999` no embarque direto;
//! - [x] exigir `CodigoAreaDestino` para desembarque (1) e usar `9999` no desembarque direto.
//!
//! O Manual não determina que origem e destino diferentes sejam uma regra
//! geral; portanto, essa restrição não integra o checklist.
//!
//! ## Tipo de operação da carga — Manual §2.2.5
//!
//! - [x] permitir os códigos 13 a 20 somente quando `TipoOperacao` da
//!   atracação for movimentação de carga (1) ou misto (7);
//! - [x] quando a atracação for apoio (3) ou abastecimento (5), exigir o tipo
//!   de operação da carga de mesmo nome;
//! - [x] em safamento (8) e remoção a bordo (10), verificar os intervalos de
//!   código 8701–8716, 8401–8429 e 4401–4421 para a representação não
//!   conteinerizada; códigos de contêiner são tratados por suas regras próprias;
//! - [x] exigir natureza, quantidade e peso bruto em safamento e remoção,
//!   inclusive quantidade de contêineres;
//! - [x] seguir a omissão conjunta de navegação, origem e destino da §2.2.5
//!   (p. 18), também para intermediária e transferência interna; a referência
//!   a zero na p. 14 é divergente e não foi adotada;
//! - [x] safamento (8), remoção a bordo (10) e abastacimento (5) não têm código de área.
//!
//! ## Quantidade, peso e IMDG — Manual §2.1, §2.2.4 e §10
//!
//! - [x] exigir `Quantidade` para toda carga de natureza carga geral (3);
//! - [x] limitar a soma de contêineres da atracação a 32.767 unidades e as
//!   demais cargas a 99.999.999 unidades;
//! - [x] o valor do `PesoCargaBruta` não pode exceder 564.650 toneladas nos
//!   ramos de representação conteinerizada; não há limite geral conectado.
//! - [x] o somatório dos pesos líquidos dos conteineres deve ser menor do que
//!   o `PesoCargaBruta`, como revisão local; comparar somente quando todos
//!   os pesos líquidos existem.
//! - [x] exigir `PesoCargaBruta` maior que zero, em toneladas, com até três
//!   casas decimais;
//! - [x] para contêiner, interpretar o peso bruto como carga mais tara e
//!   aplicar o formato de até 13 posições com três casas decimais descrito no
//!   Manual;
//! - [x] somar `PesoCargaLiquida` de todos os detalhamentos da carga e exigir
//!   total positivo quando `CheioVazio = C`, ou total zero quando
//!   `CheioVazio = V`;
//! - [x] para carga de contêiner vazio, limitar `PesoCargaBruta` a 20
//!   toneladas, independentemente da quantidade de detalhamentos;
//! - [x] receber parcelas brutas próprias e de terceiros como opcionais no
//!   leiaute TUP, sem inferir igualdade com o total;
//! - [x] exigir `ImdgCode` em toda carga, usando 0 para produto não perigoso;
//! - [x] delegar cada detalhamento às regras de
//!   `validation::carga_conteinerizada`.
//!
//! ## Consolidação — Manual §2.1 e §3.2
//!
//! - [x] dentro da mesma atracação, detectar cargas com características
//!   idênticas salvo os pesos e exigir sua consolidação; a validação aponta
//!   o grupo e não altera o documento;
//! - [x] permitir registros separados quando ao menos origem ou destino forem
//!   diferentes.

use crate::{
    interner::TextoId,
    validation::{
        atracacao::{InstalacaoInformante, TipoOperacaoAtracacao},
        helper::ListagemOxford,
    },
    xml::{
        models::{
            carga::Carga,
            enums::{NaturezaCarga, NavegacaoCarga, Sentido, TipoOperacao, TipoOperacaoCarga},
            peso::Peso,
        },
        reader::{CargaConteinerizadaSourceMap, CargaSourceMap, ElementId},
    },
};

use super::{
    context::{RotuloDiagnostico, ValidationContext},
    helper::{validar_cnpj, validar_codigo_alfanumerico, validar_codigo_numerico},
    regras_de_negocio as regras,
};

/// Estado declarado para os contêineres detalhados por uma carga.
///
/// O estado é obtido de `<CheioVazio>` na carga pai e compartilhado por todos
/// os seus blocos `<CargaConteinerizada>`. A variante [`Self::NaoInformado`]
/// preserva a ausência da tag para que as regras não suponham arbitrariamente
/// que o contêiner seja cheio ou vazio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EstadoContainer {
    /// A carga pai contém `<CheioVazio>C</CheioVazio>`.
    Cheio,
    /// A carga pai contém `<CheioVazio>V</CheioVazio>`.
    Vazio,
    /// A carga pai não contém `<CheioVazio>`.
    NaoInformado,
}

impl EstadoContainer {
    /// Converte a representação decodificada de `<CheioVazio>` para o estado
    /// usado pelas regras de validação dos detalhamentos.
    pub(super) const fn de_cheio_vazio(cheio_vazio: Option<bool>) -> Self {
        match cheio_vazio {
            Some(true) => Self::Cheio,
            Some(false) => Self::Vazio,
            None => Self::NaoInformado,
        }
    }
}

/// Lado da movimentação ao qual uma regra de instalação se refere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Extremidade {
    /// Instalação de destino da carga.
    Destino,
    /// Instalação de origem da carga.
    Origem,
}

/// Representação selecionada pelas declarações do XML, independentemente de sua validade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClassificacaoCarga {
    /// Bloco, estado ou código catalogado declaram conteinerização.
    Conteinerizada,
    /// Código com formato SH4 e nenhuma declaração de conteinerização.
    NaoConteinerizada,
    /// Código inválido sem declaração que permita escolher uma representação.
    Ambigua,
    /// Operação de apoio cujo código não é reconhecível pelo catálogo local.
    Apoio,
}

/// Classifica sem emitir diagnósticos e sem atribuir probabilidades à intenção.
///
/// Dá prioridade a bloco de contêiner não vazio, `CheioVazio` ou código ISO
/// catalogado. Sem essas declarações, apoio preserva o código opaco; quatro
/// algarismos indicam mercadoria e os demais códigos deixam a intenção ambígua.
/// O resultado seleciona regras, não certifica a validade da carga. É uma
/// decisão pura: a orquestração consulta o catálogo uma vez e fornece o
/// resultado em `codigo_catalogado`, reutilizado pela regra de código ISO.
fn classificar_carga(
    codigo_carga: &str,
    tipo_operacao: TipoOperacaoCarga,
    possui_detalhamento: bool,
    possui_estado: bool,
    codigo_catalogado: bool,
) -> ClassificacaoCarga {
    if possui_detalhamento || possui_estado || codigo_catalogado {
        ClassificacaoCarga::Conteinerizada
    } else if tipo_operacao == TipoOperacaoCarga::Apoio {
        ClassificacaoCarga::Apoio
    } else if validar_codigo_numerico::<4>(codigo_carga).is_ok() {
        ClassificacaoCarga::NaoConteinerizada
    } else {
        ClassificacaoCarga::Ambigua
    }
}

/// Orquestra regras independentes, de movimentação e de representação da carga.
///
/// Retorna o estado informado para validar os detalhamentos. Erros no código
/// não tornam condicionais as regras de uma carga explicitamente conteinerizada.
/// A operação seleciona regras aplicáveis; o sucesso de uma validação não
/// condiciona a execução das demais regras independentes.
///
/// # Panics
///
/// Pode entrar em pânico se os detalhamentos não estiverem alinhados com
/// `conteineres_source_map` ou se IDs não pertencerem ao mapa do contexto.
pub(super) fn validar_carga(
    context: &mut ValidationContext<'_, '_>,
    informante: Option<InstalacaoInformante>,
    operacao_atracacao: TipoOperacaoAtracacao,
    carga: &Carga<'_>,
    source_map: &CargaSourceMap,
    conteineres_source_map: &[CargaConteinerizadaSourceMap],
) -> EstadoContainer {
    validar_cnpj_operador(context, carga.cnpj_operador, source_map.cnpj_operador);
    for id in [
        Some(source_map.peso_carga_bruta),
        source_map.peso_carga_bruta_propria,
        source_map.peso_carga_bruta_terceiros,
    ]
    .into_iter()
    .flatten()
    {
        super::campos::validar_formato_peso(context, id);
    }
    if carga.peso_carga_bruta == Peso::ZERO {
        context.erro_nos_valores(
            "<PesoCargaBruta> deve ser maior que zero",
            regras::pendentes::FORMATO_PESOS,
            [(
                source_map.peso_carga_bruta,
                "informe a massa total em toneladas",
            )],
        );
    }
    validar_presenca_quantidade_carga_geral(
        context,
        carga.natureza_carga,
        source_map.natureza_carga,
        carga.quantidade,
        source_map.element,
    );
    if let Some((codigo, elemento)) = carga.codigo_ncm.zip(source_map.codigo_ncm) {
        validar_codigo_ncm(context, codigo, elemento);
    }
    let codigo_catalogado = context.codigo_conteiner_existe(context.texto(carga.codigo_carga));
    let classificacao = classificar_carga(
        context.texto(carga.codigo_carga),
        carga.tipo_operacao_carga,
        carga
            .carga_conteinerizada
            .as_ref()
            .is_some_and(|itens| !itens.is_empty()),
        carga.cheio_vazio.is_some(),
        codigo_catalogado,
    );
    if classificacao != ClassificacaoCarga::Conteinerizada
        && let Some((quantidade, id)) = carga.quantidade.zip(source_map.quantidade)
        && quantidade > 99_999_999
    {
        context.erro_nos_valores(
            "<Quantidade> ultrapassa 99.999.999 unidades",
            regras::LIMITES_QUANTIDADE,
            [(id, "limite máximo por registro de carga não conteinerizada")],
        );
    }
    validar_regras_da_operacao(
        context,
        carga,
        source_map,
        operacao_atracacao,
        classificacao,
    );
    validar_movimentacao(context, informante, carga, source_map);
    validar_representacao_carga(
        context,
        carga,
        source_map,
        conteineres_source_map,
        classificacao,
        codigo_catalogado,
    );
    EstadoContainer::de_cheio_vazio(carga.cheio_vazio)
}

/// Seleciona as regras de representação e preserva os ramos de intenção ambígua.
///
/// Cada ramo usa as mesmas funções de regra da execução normal. Esta classe
/// não decide se a operação permite navegação ou identificação de instalações.
fn validar_representacao_carga(
    context: &mut ValidationContext<'_, '_>,
    carga: &Carga<'_>,
    source_map: &CargaSourceMap,
    conteineres_source_map: &[CargaConteinerizadaSourceMap],
    classificacao: ClassificacaoCarga,
    codigo_catalogado: bool,
) {
    match classificacao {
        ClassificacaoCarga::Conteinerizada => {
            validar_representacao_conteinerizada(
                context,
                carga,
                source_map,
                conteineres_source_map,
                codigo_catalogado,
            );
            validar_ce_mercante(context, carga, source_map, true);
        }
        ClassificacaoCarga::NaoConteinerizada | ClassificacaoCarga::Apoio => {
            validar_ce_mercante(context, carga, source_map, false);
        }
        ClassificacaoCarga::Ambigua => {
            let conteiner = context.alternativa("Carga conteinerizada", |ramo| {
                validar_representacao_conteinerizada(
                    ramo,
                    carga,
                    source_map,
                    conteineres_source_map,
                    codigo_catalogado,
                );
            });
            let mercadoria = context.alternativa("Mercadoria não conteinerizada", |ramo| {
                validar_codigo_mercadoria(ramo, carga.codigo_carga, source_map.codigo_carga);
                validar_ce_mercante(ramo, carga, source_map, false);
            });
            context.erro_com_alternativas(
                "o código informado e a ausência de indicadores não permitem determinar a representação da carga",
                regras::REPRESENTACAO_CARGA,
                [RotuloDiagnostico::no_valor(source_map.codigo_carga, "escolha a representação pretendida para consultar suas exigências")],
                vec![conteiner, mercadoria],
            );
        }
    }
}

/// Confere o CNPJ do operador, sem consulta a cadastro externo.
fn validar_cnpj_operador<'xml>(
    context: &mut ValidationContext<'_, 'xml>,
    cnpj: TextoId,
    elemento: ElementId,
) {
    if let Err((_, label)) = validar_cnpj(context.texto(cnpj)) {
        context.erro_nos_valores(
            "o valor de <CnpjOperador> não representa um CNPJ válido",
            regras::FORMACAO_CNPJ,
            [(elemento, label)],
        );
    }
}

/// Aplica regras objetivas da representação de contêiner.
///
/// Também pode executar dentro de um ramo alternativo: o contexto coletor
/// organiza os resultados, sem mudar a implementação das regras específicas.
fn validar_representacao_conteinerizada(
    context: &mut ValidationContext<'_, '_>,
    carga: &Carga<'_>,
    source_map: &CargaSourceMap,
    conteineres: &[CargaConteinerizadaSourceMap],
    codigo_catalogado: bool,
) {
    validar_codigo_conteiner(
        context,
        carga.codigo_carga,
        source_map.codigo_carga,
        source_map.cheio_vazio,
        conteineres.first().map(|conteiner| conteiner.element),
        codigo_catalogado,
    );
    if carga.cheio_vazio.is_none() {
        let mut labels = vec![
            RotuloDiagnostico::antes_tag_fechamento(
                source_map.element,
                "informe <CheioVazio> com C (cheio) ou V (vazio)",
            ),
            RotuloDiagnostico::evidencia_no_valor(source_map.codigo_carga),
        ];
        if let Some(primeiro) = conteineres.first() {
            labels.push(RotuloDiagnostico::evidencia_na_tag_abertura(
                primeiro.element,
            ));
        }
        context.erro_com_detalhes(
            "a carga conteinerizada exige a tag <CheioVazio>",
            regras::ESTADO_CONTEINER,
            labels,
        );
    }
    if conteineres.is_empty() {
        context.erro_antes_tags_fechamento(
            "a carga conteinerizada exige detalhamento em <CargaConteinerizada>",
            regras::DETALHAMENTO_CONTEINER,
            [(
                source_map.element,
                "informe os contêineres e, quando cheios, suas mercadorias",
            )],
        );
    }
    validar_natureza_carga_conteinerizada(context, carga.natureza_carga, source_map.natureza_carga);
    validar_adequacao_peso_cheio_vazio(context, carga, source_map, conteineres);
}

/// Exige um código ISO incorporado para uma representação de contêiner.
///
/// `codigo_catalogado` deve ser o resultado da consulta feita para este mesmo
/// código pela orquestração. Os IDs opcionais apenas acrescentam evidências
/// ao diagnóstico; esta função não consulta novamente o catálogo.
fn validar_codigo_conteiner<'xml>(
    context: &mut ValidationContext<'_, 'xml>,
    codigo_carga: TextoId,
    codigo_carga_id: ElementId,
    cheio_vazio_id: Option<ElementId>,
    primeiro_conteiner_id: Option<ElementId>,
    codigo_catalogado: bool,
) {
    if codigo_catalogado {
        return;
    }
    let motivo = validar_codigo_alfanumerico::<4>(context.texto(codigo_carga))
        .err()
        .map_or_else(
            || "o código não consta no catálogo ISO incorporado".into(),
            |erro| erro.to_string(),
        );
    let mut labels = vec![RotuloDiagnostico::no_valor(codigo_carga_id, motivo)];
    if let Some(estado) = cheio_vazio_id {
        labels.push(RotuloDiagnostico::evidencia_na_tag_abertura(estado));
    }
    if let Some(conteiner_id) = primeiro_conteiner_id {
        labels.push(RotuloDiagnostico::evidencia_na_tag_abertura(conteiner_id));
    }
    context.erro_com_detalhes(
        "a carga conteinerizada exige um código ISO reconhecido em <CodigoCarga>",
        regras::CODIGO_CONTEINER,
        labels,
    );
}

/// Exige o formato SH4 sob a interpretação de mercadoria não conteinerizada.
fn validar_codigo_mercadoria<'xml>(
    context: &mut ValidationContext<'_, 'xml>,
    codigo: TextoId,
    elemento: ElementId,
) {
    if let Err(erro) = validar_codigo_numerico::<4>(context.texto(codigo)) {
        context.erro_nos_valores(
            "a mercadoria não conteinerizada exige quatro algarismos em <CodigoCarga>",
            regras::CODIGO_MERCADORIA,
            [(elemento, erro.to_string())],
        );
    }
}

/// Valida a representação lexical do código NCM/SH8 opcional.
///
/// A orquestração já selecionou um valor presente e seu ID. A ausência da tag
/// não é inferida a partir de uma string vazia e não é verificada nesta função.
fn validar_codigo_ncm<'xml>(
    context: &mut ValidationContext<'_, 'xml>,
    codigo_ncm: TextoId,
    elemento: ElementId,
) {
    if let Err(erro) = validar_codigo_numerico::<8>(context.texto(codigo_ncm)) {
        context.erro_nos_valores(
            "o valor de <CodigoNCM> deve conter exatamente oito algarismos ASCII",
            regras::CODIGO_NCM,
            [(elemento, erro.to_string())],
        );
    }
}

/// Orquestra as regras de massa condicionadas por `<CheioVazio>`.
///
/// A classificação cheio/vazio pertence à `<Carga>`, não a uma ocorrência
/// isolada de `<CargaConteinerizada>`. Por isso, a regra soma primeiro todos
/// os valores de `<PesoCargaLiquida>` e só então despacha para as regras de
/// carga cheia ou vazia. Para carga cheia, um peso ausente impede calcular
/// o total; a regra de presença relata a ausência. O limite de peso bruto
/// dos vazios é independente da existência ou validade dos detalhamentos.
///
/// Antes de inspecionar o estado, também compara o bruto com 564.650 toneladas.
/// Esta função só é chamada nos ramos de interpretação conteinerizada; não
/// representa uma verificação geral de peso de todas as cargas.
///
/// # Panics
///
/// Entra em pânico se os detalhamentos e seus mapas tiverem comprimentos distintos.
fn validar_adequacao_peso_cheio_vazio(
    context: &mut ValidationContext<'_, '_>,
    carga: &Carga<'_>,
    source_map: &CargaSourceMap,
    conteineres_source_map: &[CargaConteinerizadaSourceMap],
) {
    /// Limite usado por esta regra de representação, em toneladas.
    const PESO_MAX: Peso = Peso::from_toneladas(564_650);

    if carga.peso_carga_bruta > PESO_MAX {
        context.erro_nos_valores(
            format!("a massa informada na carga \"{}\" excede o limite de peso de carga conteinerizada.", context.texto(carga.codigo_carga)),
            regras::LIMITE_PESO_CONTEINERIZADO,
            [(
                source_map.peso_carga_bruta,
                format!("ajuda: altere esse valor para algo menor do que {PESO_MAX}"),
            )]
        );
    }

    let (Some(cheio), Some(cheio_id)) = (carga.cheio_vazio, source_map.cheio_vazio) else {
        return;
    };

    let conteineres = carga.carga_conteinerizada.as_deref().unwrap_or(&[]);
    assert_eq!(
        conteineres.len(),
        conteineres_source_map.len(),
        "o reader deve manter contêineres e mapas de origem alinhados"
    );

    if !cheio {
        validar_peso_maximo_carga_vazia(
            context,
            carga.peso_carga_bruta,
            source_map.peso_carga_bruta,
            cheio_id,
        );
    }
    if cheio
        && (conteineres.is_empty() || conteineres.iter().any(|c| c.peso_carga_liquida.is_none()))
    {
        return; // A regra de presença relata os pesos ausentes; o total ainda não é conhecido.
    }
    let Some(peso_total) = conteineres
        .iter()
        .filter_map(|c| c.peso_carga_liquida)
        .try_fold(Peso::ZERO, |total, peso| total.checked_add(peso))
    else {
        context.erro_nos_valores(
            "a soma dos pesos líquidos excede o limite de representação",
            regras::REPRESENTACAO_SOMA_PESOS,
            [(
                source_map.peso_carga_bruta,
                "reduza ou corrija os pesos informados",
            )],
        );
        return;
    };

    if cheio {
        validar_massas_carga_cheia(
            context,
            (carga.peso_carga_bruta, source_map.peso_carga_bruta),
            conteineres_source_map,
            cheio_id,
            peso_total,
        );
    } else {
        validar_massas_carga_vazia(context, conteineres_source_map, cheio_id, peso_total);
    }
}

/// Aplica as relações de massa exigidas para uma carga marcada como cheia.
///
/// Recebe o peso bruto com seu ID e a soma previamente calculada. Os mapas dos
/// detalhamentos são usados apenas para selecionar as labels da soma.
fn validar_massas_carga_cheia(
    context: &mut ValidationContext<'_, '_>,
    (peso_bruto, peso_bruto_id): (Peso, ElementId),
    conteineres_source_map: &[CargaConteinerizadaSourceMap],
    cheio_id: ElementId,
    peso_total: Peso,
) {
    if peso_total == Peso::ZERO {
        let mut labels = vec![RotuloDiagnostico::no_valor(
            cheio_id,
            "a soma dos pesos líquidos é 0.000 tonelada; ajuda: use `V` se os contêineres desta carga estiverem vazios",
        )];
        labels.extend(rotulos_primeiros_pesos_liquidos(
            conteineres_source_map,
            "este peso contribui para a soma igual a zero",
        ));

        let message = "<CheioVazio> e o peso líquido total da carga são incompatíveis";
        context.erro_com_detalhes(message, regras::PESO_LIQUIDO_POR_ESTADO, labels);
    }

    if peso_total >= peso_bruto && peso_total != Peso::ZERO {
        let mut labels = vec![RotuloDiagnostico::no_valor(
            peso_bruto_id,
            format!(
                "o peso bruto informado é {peso_bruto}, enquanto a soma dos pesos líquidos é {peso_total} tonelada(s); ajuda: o peso bruto deve superar {peso_total} para incluir a tara"
            ),
        )];
        labels.extend(rotulos_primeiros_pesos_liquidos(
            conteineres_source_map,
            "este peso compõe a soma maior ou igual ao peso bruto",
        ));

        let message =
            "<PesoCargaBruta> não supera a soma dos pesos líquidos da carga conteinerizada";
        context.erro_com_detalhes(message, regras::PESO_BRUTO_E_LIQUIDO, labels);
    }
}

/// Aplica as relações de massa exigidas para uma carga marcada como vazia.
fn validar_massas_carga_vazia<'xml>(
    context: &mut ValidationContext<'_, 'xml>,
    conteineres_source_map: &[CargaConteinerizadaSourceMap],
    cheio_id: ElementId,
    peso_total: Peso,
) {
    if peso_total > Peso::ZERO {
        let mut labels = vec![RotuloDiagnostico::no_valor(
            cheio_id,
            format!(
                "a soma dos pesos líquidos é {peso_total} tonelada(s); ajuda: use `C` se os pesos líquidos informados estiverem corretos"
            ),
        )];
        labels.extend(rotulos_primeiros_pesos_liquidos(
            conteineres_source_map,
            "este peso compõe a soma positiva",
        ));

        let message = "<CheioVazio> e o peso líquido total da carga são incompatíveis";
        context.erro_com_detalhes(message, regras::PESO_LIQUIDO_POR_ESTADO, labels);
    }
}

/// Cria labels para os três primeiros pesos líquidos presentes, no máximo.
///
/// O limite afeta somente a apresentação. A soma usada pela regra continua
/// considerando todos os elementos de `conteineres`. A filtragem das ausências
/// acontece antes do limite; portanto, pode percorrer mais de três detalhamentos
/// para encontrar três pesos. A presença da tag pertence a outra regra.
/// A terceira label indica com `⋮` a existência de detalhamentos não apontados,
/// sem acrescentar informações da ocorrência ao enunciado geral em `details`.
fn rotulos_primeiros_pesos_liquidos(
    conteineres: &[CargaConteinerizadaSourceMap],
    mensagem: &str,
) -> Vec<RotuloDiagnostico> {
    conteineres
        .iter()
        .filter_map(|conteiner| conteiner.peso_carga_liquida)
        .take(conteineres.len().min(3))
        .enumerate()
        .map(|(indice, peso)| {
            if indice == 2 && conteineres.len() > 3 {
                RotuloDiagnostico::no_valor(
                    peso,
                    format!("{mensagem} ⋮ Há outros detalhamentos não apontados."),
                )
            } else {
                RotuloDiagnostico::no_valor(peso, mensagem)
            }
        })
        .collect()
}

/// Limita a 20 toneladas o peso bruto de uma carga de contêiner vazio.
///
/// O limite se aplica ao valor integral de `<PesoCargaBruta>` e independe da
/// quantidade de blocos `<CargaConteinerizada>` existentes no registro.
fn validar_peso_maximo_carga_vazia<'xml>(
    context: &mut ValidationContext<'_, 'xml>,
    peso_bruto: Peso,
    peso_bruto_id: ElementId,
    cheio_id: ElementId,
) {
    let limite = Peso::from_quilogramas(20_000);

    if peso_bruto <= limite {
        return;
    }

    context.erro_com_detalhes(
        "<PesoCargaBruta> ultrapassa o limite de 20 toneladas para carga de contêiner vazio",
        regras::LIMITE_PESO_CONTEINER_VAZIO,
        vec![
            RotuloDiagnostico::no_valor(
                cheio_id,
                "este valor submete a carga ao limite dos contêineres vazios",
            ),
            RotuloDiagnostico::no_valor(
                peso_bruto_id,
                format!("ajuda: informe um peso bruto total de no máximo {limite}"),
            ),
        ],
    );
}

/// Define a aplicabilidade das regras de navegação e instalações.
///
/// O resultado depende apenas da operação declarada, não de erros produzidos
/// por outra regra. O conjunto fica concentrado aqui para que as validações
/// dos campos não repitam a mesma seleção de operações.
fn operacao_omite_navegacao_origem_destino(tipo_operacao: TipoOperacaoCarga) -> bool {
    matches!(
        tipo_operacao,
        TipoOperacaoCarga::Abastecimento
            | TipoOperacaoCarga::Safamento
            | TipoOperacaoCarga::RemocaoABordo
            | TipoOperacaoCarga::OperacaoIntermediaria
            | TipoOperacaoCarga::TransferenciaInterna
    )
}

/// Decide quais regras de movimentação se aplicam à operação declarada.
///
/// A proibição de navegação pertence ao ramo restrito; sua presença e seu valor
/// pertencem ao ramo aplicável. As regras de instalações não repetem esse
/// despacho. A proibição conjunta produz um diagnóstico com todas as tags
/// indevidas, sem executar em paralelo a exigência desses mesmos campos.
fn validar_movimentacao(
    context: &mut ValidationContext<'_, '_>,
    informante: Option<InstalacaoInformante>,
    carga: &Carga<'_>,
    source_map: &CargaSourceMap,
) {
    if operacao_omite_navegacao_origem_destino(carga.tipo_operacao_carga) {
        validar_ausencia_de_campos_tipo_operacao(context, carga.tipo_operacao_carga, source_map);
    } else {
        validar_navegacao(
            context,
            carga.navegacao,
            source_map.navegacao,
            source_map.element,
        );
        validar_formatos_instalacoes(context, carga, source_map);
        validar_area_por_sentido(context, carga.sentido, source_map);
        if let Some(informante) = informante {
            validar_origem_destino(context, carga, source_map, informante);
        } else {
            validar_unicidade_tipo_de_instalacao(context, source_map, Extremidade::Origem);
            validar_unicidade_tipo_de_instalacao(context, source_map, Extremidade::Destino);
        }
    }
}

/// Orquestra as relações entre instalações para uma operação que as admite.
///
/// Mantém a condição preexistente de haver um informante identificável.
/// Primeiro verifica a exclusividade da outra extremidade; depois, a
/// representação e a correspondência da extremidade associada ao informante.
/// Sentido não informado não permite selecionar essas extremidades.
fn validar_origem_destino(
    context: &mut ValidationContext<'_, '_>,
    carga: &Carga<'_>,
    source_map: &CargaSourceMap,
    informante: InstalacaoInformante,
) {
    let Some(sentido) = carga.sentido else {
        return;
    };
    let outra_extremidade = match sentido {
        Sentido::Embarque => Extremidade::Destino,
        Sentido::Desembarque => Extremidade::Origem,
        Sentido::NaoInformado => return,
    };
    validar_unicidade_tipo_de_instalacao(context, source_map, outra_extremidade);
    validar_correspondencia_instalacao_informante(context, carga, source_map, informante);
}

/// Confere a representação e o identificador da extremidade informante.
///
/// No embarque, valida a origem; no desembarque, o destino. A exclusividade
/// da outra extremidade é uma regra independente, chamada pela orquestração
/// de [`validar_origem_destino`], não por esta comparação.
/// Para porto, exige bigrama e trigrama e proíbe código TUP. Para TUP, exige
/// código TUP e proíbe bigrama e trigrama. Os nomes completos das tags são
/// selecionados junto dos campos para preservar a precisão dos diagnósticos.
///
/// Ausências e presenças indevidas são verificadas independentemente. Somente
/// a comparação de identificadores depende de o identificador estar presente.
/// Esta regra não valida a outra extremidade, áreas, formatos ou cadastros.
/// O sentido não informado não permite selecionar uma extremidade.
fn validar_correspondencia_instalacao_informante(
    context: &mut ValidationContext<'_, '_>,
    carga: &Carga<'_>,
    source_map: &CargaSourceMap,
    instalacao: InstalacaoInformante,
) {
    let Some((sentido, sentido_id)) = carga.sentido.zip(source_map.sentido) else {
        return;
    };

    let (
        extremidade,
        bigrama_id,
        trigrama_carga,
        trigrama_id,
        codigo_tup,
        codigo_tup_id,
        tag_bigrama,
        tag_trigrama,
        tag_tup,
    ) = match sentido {
        Sentido::Embarque => (
            "a origem",
            source_map.bigrama_pais_origem,
            carga.trigrama_porto_origem,
            source_map.trigrama_porto_origem,
            carga.codigo_tup_origem,
            source_map.codigo_tup_origem,
            "BigramaPaisOrigem",
            "TrigramaPortoOrigem",
            "CodigoTUPOrigem",
        ),
        Sentido::Desembarque => (
            "o destino",
            source_map.bigrama_pais_destino,
            carga.trigrama_porto_destino,
            source_map.trigrama_porto_destino,
            carga.codigo_tup_destino,
            source_map.codigo_tup_destino,
            "BigramaPaisDestino",
            "TrigramaPortoDestino",
            "CodigoTUPDestino",
        ),
        Sentido::NaoInformado => return,
    };

    let (informante_id, esperado, identificador) = match instalacao {
        InstalacaoInformante::Porto { trigrama, elemento } => {
            let pais = match sentido {
                Sentido::Embarque => carga.bigrama_pais_origem,
                _ => carga.bigrama_pais_destino,
            };
            if let Some((pais, id)) = pais.zip(bigrama_id)
                && context.texto(pais) != "BR"
            {
                context.erro_com_detalhes(
                    "o país da instalação informante deve ser BR",
                    regras::INSTALACAO_POR_SENTIDO,
                    [
                        RotuloDiagnostico::no_valor(
                            id,
                            "ajuda: use BR se o informante e o sentido estiverem corretos",
                        ),
                        RotuloDiagnostico::no_valor(elemento, "porto brasileiro informante"),
                        RotuloDiagnostico::no_valor(
                            sentido_id,
                            "sentido que seleciona esta extremidade",
                        ),
                    ],
                );
            }
            validar_presenca_campos_instalacao(
                context,
                source_map.element,
                sentido_id,
                elemento,
                "porto",
                &[(tag_bigrama, bigrama_id), (tag_trigrama, trigrama_id)],
            );
            validar_ausencia_campos_instalacao(
                context,
                sentido_id,
                elemento,
                "porto",
                &[(tag_tup, codigo_tup_id)],
            );
            (elemento, trigrama, trigrama_carga.zip(trigrama_id))
        }
        InstalacaoInformante::Tup { codigo, elemento } => {
            validar_presenca_campos_instalacao(
                context,
                source_map.element,
                sentido_id,
                elemento,
                "TUP",
                &[(tag_tup, codigo_tup_id)],
            );
            validar_ausencia_campos_instalacao(
                context,
                sentido_id,
                elemento,
                "TUP",
                &[(tag_bigrama, bigrama_id), (tag_trigrama, trigrama_id)],
            );
            (elemento, codigo, codigo_tup.zip(codigo_tup_id))
        }
    };

    if let Some((valor, elemento)) = identificador
        && valor != esperado
    {
        context.erro_com_detalhes(
            format!("a instalação informante e {extremidade} da carga são incompatíveis"),
            regras::INSTALACAO_POR_SENTIDO,
            [
                RotuloDiagnostico::no_valor(informante_id, "esta é a instalação informante"),
                RotuloDiagnostico::no_valor(
                    elemento,
                    format!(
                        "ajuda: use \"{}\" se a instalação informante e o sentido estiverem corretos",
                        context.texto(esperado)
                    ),
                ),
                RotuloDiagnostico::no_valor(
                    sentido_id,
                    format!(
                        "ajuda: confira o sentido; {} exige que {extremidade} seja o informante",
                        sentido,
                    ),
                ),
            ],
        );
    }
}

/// Exige uma representação completa e exclusiva na extremidade não informante.
/// Ausências são ancoradas antes de `</Carga>`, sem presumir porto ou TUP.
fn validar_unicidade_tipo_de_instalacao(
    context: &mut ValidationContext<'_, '_>,
    source_map: &CargaSourceMap,
    extremidade: Extremidade,
) {
    let (nome, tup, pais, porto) = match extremidade {
        Extremidade::Origem => (
            "origem",
            source_map.codigo_tup_origem,
            source_map.bigrama_pais_origem,
            source_map.trigrama_porto_origem,
        ),
        Extremidade::Destino => (
            "destino",
            source_map.codigo_tup_destino,
            source_map.bigrama_pais_destino,
            source_map.trigrama_porto_destino,
        ),
    };
    if let Some(tup) = tup {
        let outros: Vec<_> = [pais, porto].into_iter().flatten().collect();
        if !outros.is_empty() {
            let mut labels = vec![RotuloDiagnostico::na_tag_abertura(
                tup,
                "ajuda: remova o código TUP se a instalação for um porto",
            )];
            labels.extend(outros.into_iter().map(|id| {
                RotuloDiagnostico::na_tag_abertura(
                    id,
                    "ajuda: remova esta tag se a instalação for um TUP",
                )
            }));
            context.erro_com_detalhes(
                format!("identificadores de porto e TUP são incompatíveis na {nome}"),
                regras::REPRESENTACAO_INSTALACAO,
                labels,
            );
        }
    } else if pais.is_none() || porto.is_none() {
        let mut labels = vec![RotuloDiagnostico::antes_tag_fechamento(
            source_map.element,
            "ajuda: informe a dupla de país e porto ou o código TUP da instalação",
        )];
        labels.extend(
            [pais, porto]
                .into_iter()
                .flatten()
                .map(|id| RotuloDiagnostico::na_tag_abertura(id, "representação incompleta")),
        );
        context.erro_com_detalhes(
            format!("a identificação da instalação de {nome} está incompleta"),
            regras::REPRESENTACAO_INSTALACAO,
            labels,
        );
    }
}

/// Valida os formatos presentes nas duas extremidades, sem consultar cadastros.
fn validar_formatos_instalacoes(
    context: &mut ValidationContext<'_, '_>,
    c: &Carga<'_>,
    m: &CargaSourceMap,
) {
    for (valor, id, tag, tamanho, alfabetico) in [
        (
            c.bigrama_pais_origem,
            m.bigrama_pais_origem,
            "BigramaPaisOrigem",
            2,
            true,
        ),
        (
            c.bigrama_pais_destino,
            m.bigrama_pais_destino,
            "BigramaPaisDestino",
            2,
            true,
        ),
        (
            c.trigrama_porto_origem,
            m.trigrama_porto_origem,
            "TrigramaPortoOrigem",
            3,
            false,
        ),
        (
            c.trigrama_porto_destino,
            m.trigrama_porto_destino,
            "TrigramaPortoDestino",
            3,
            false,
        ),
        (
            c.codigo_tup_origem,
            m.codigo_tup_origem,
            "CodigoTUPOrigem",
            7,
            false,
        ),
        (
            c.codigo_tup_destino,
            m.codigo_tup_destino,
            "CodigoTUPDestino",
            7,
            false,
        ),
    ] {
        let Some((valor, id)) = valor.zip(id) else {
            continue;
        };
        let texto = context.texto(valor);
        let erro = if texto.chars().count() != tamanho {
            Some(format!(
                "este valor contém {} caracteres; eram esperados exatamente {tamanho}",
                texto.chars().count()
            ))
        } else if let Some((i, ch)) = texto
            .chars()
            .enumerate()
            .find(|(_, ch)| !(ch.is_ascii_uppercase() || (!alfabetico && ch.is_ascii_digit())))
        {
            Some(format!(
                "o caractere `{ch}` na posição {} não é permitido; use letras ASCII maiúsculas{}",
                i + 1,
                if alfabetico { "" } else { " ou algarismos" }
            ))
        } else if tamanho == 7
            && !(texto.starts_with("BR")
                && texto.as_bytes()[2..4].iter().all(u8::is_ascii_uppercase)
                && texto.as_bytes()[4..].iter().all(u8::is_ascii_digit))
        {
            Some("use BR, duas letras maiúsculas de UF e três algarismos, como BRSP008".to_owned())
        } else {
            None
        };
        if let Some(erro) = erro {
            context.erro_nos_valores(
                format!("o formato de <{tag}> é inválido"),
                regras::REPRESENTACAO_INSTALACAO,
                [(id, erro)],
            );
        }
    }
}

/// Exige a área da instalação informante e verifica as áreas presentes.
fn validar_area_por_sentido(
    context: &mut ValidationContext<'_, '_>,
    sentido: Option<Sentido>,
    m: &CargaSourceMap,
) {
    for (tag, id) in [
        ("CodigoAreaOrigem", m.codigo_area_origem),
        ("CodigoAreaDestino", m.codigo_area_destino),
    ] {
        if let Some(id) = id
            && let Err(erro) = validar_codigo_alfanumerico::<4>(context.valor_original(id))
        {
            context.erro_nos_valores(
                format!("o formato de <{tag}> é inválido"),
                regras::pendentes::AREA_POR_SENTIDO,
                [(id, erro.to_string())],
            );
        }
    }
    let (tag, id) = match sentido {
        Some(Sentido::Embarque) => ("CodigoAreaOrigem", m.codigo_area_origem),
        Some(Sentido::Desembarque) => ("CodigoAreaDestino", m.codigo_area_destino),
        _ => return,
    };
    if id.is_none() {
        context.erro_com_detalhes(
            format!("o sentido exige a presença de <{tag}>"),
            regras::pendentes::AREA_POR_SENTIDO,
            [
                RotuloDiagnostico::no_valor(
                    m.sentido.expect("sentido presente no modelo e no mapa"),
                    "sentido da movimentação",
                ),
                RotuloDiagnostico::antes_tag_fechamento(
                    m.element,
                    format!("ajuda: informe <{tag}>; use 9999 se a operação for direta"),
                ),
            ],
        );
    }
}

/// Exige cada campo da representação da extremidade correspondente ao informante.
///
/// Usa a presença no mapa de origem, alinhado ao modelo pelo reader. Cada
/// ausência aponta para o informante, o sentido e um ponto de inserção antes
/// de `</Carga>`; não interrompe a verificação dos demais campos.
/// Recebe os IDs já selecionados pela regra de correspondência, sem acessar
/// outros campos da carga.
fn validar_presenca_campos_instalacao<'xml>(
    context: &mut ValidationContext<'_, 'xml>,
    carga_id: ElementId,
    sentido_id: ElementId,
    informante_id: ElementId,
    tipo_instalacao: &str,
    campos: &[(&str, Option<ElementId>)],
) {
    for &(tag, elemento) in campos {
        if elemento.is_none() {
            context.erro_com_detalhes(
                format!(
                    "a ausência de <{tag}> é incompatível com o tipo da instalação informante e o sentido da carga",
                ),
                regras::INSTALACAO_POR_SENTIDO,
                [
                    RotuloDiagnostico::no_valor(
                        informante_id,
                        format!("a instalação informante é um {tipo_instalacao}"),
                    ),
                    RotuloDiagnostico::no_valor(
                        sentido_id,
                        "este sentido determina qual extremidade deve corresponder ao informante",
                    ),
                    RotuloDiagnostico::antes_tag_fechamento(
                        carga_id,
                        format!(
                            "ajuda: insira <{tag}> antes deste ponto se a instalação informante e o sentido estiverem corretos",
                        ),
                    ),
                ],
            );
        }
    }
}

/// Proíbe os campos da outra representação na extremidade informante.
///
/// Cada campo indevido recebe uma label em sua tag de abertura, junto dos
/// valores do informante e do sentido que justificam a incompatibilidade.
/// A regra continua aplicável mesmo quando os campos obrigatórios faltam.
fn validar_ausencia_campos_instalacao<'xml>(
    context: &mut ValidationContext<'_, 'xml>,
    sentido_id: ElementId,
    informante_id: ElementId,
    tipo_instalacao: &str,
    campos: &[(&str, Option<ElementId>)],
) {
    for &(tag, elemento) in campos {
        if let Some(elemento) = elemento {
            context.erro_com_detalhes(
                format!(
                    "a presença de <{tag}> é incompatível com o tipo da instalação informante e o sentido da carga",
                ),
                regras::INSTALACAO_POR_SENTIDO,
                [
                    RotuloDiagnostico::no_valor(
                        informante_id,
                        format!("a instalação informante é um {tipo_instalacao}"),
                    ),
                    RotuloDiagnostico::no_valor(
                        sentido_id,
                        "este sentido determina qual extremidade deve corresponder ao informante",
                    ),
                    RotuloDiagnostico::na_tag_abertura(
                        elemento,
                        "ajuda: remova esta tag se a instalação informante e o sentido estiverem corretos",
                    ),
                ],
            );
        }
    }
}

/// Exige carga geral para a representação de contêiner.
fn validar_natureza_carga_conteinerizada(
    context: &mut ValidationContext<'_, '_>,
    natureza: NaturezaCarga,
    elemento: ElementId,
) {
    if natureza == NaturezaCarga::CargaGeral {
        return;
    }

    let message = format!(
        "a natureza da carga conteinerizada deveria ser {}",
        NaturezaCarga::CargaGeral
    );
    let labels = [RotuloDiagnostico::no_valor(
        elemento,
        format!(
            "ajuda: use \"{}\" para carga conteinerizada",
            NaturezaCarga::CargaGeral
        ),
    )];

    context.erro_com_detalhes(message, regras::NATUREZA_CONTEINER, labels);
}

/// Verifica CE opcional com quinze caracteres e sua ausência no contêiner vazio.
/// A opcionalidade geral segue a revisão de leiaute registrada pelo projeto.
fn validar_ce_mercante(
    context: &mut ValidationContext<'_, '_>,
    carga: &Carga<'_>,
    mapa: &CargaSourceMap,
    conteinerizada: bool,
) {
    if let Some((valor, id)) = carga.ce_mercante.zip(mapa.ce_mercante) {
        if conteinerizada && carga.cheio_vazio == Some(false) {
            context.erro_nas_tags_abertura(
                "<CEMercante> não se aplica à carga de contêiner vazio",
                regras::CE_MERCANTE,
                [(
                    id,
                    "remova o conhecimento se o estado vazio estiver correto",
                )],
            );
        } else {
            super::campos::validar_ce_mercante(context, valor, id);
        }
    }
}

/// Exige navegação informada, de 1 a 5, quando a operação admite essa informação.
///
/// A aplicabilidade pertence a [`validar_movimentacao`]. Esta função não
/// reavalia o tipo de operação nem emite o diagnóstico de omissão.
fn validar_navegacao(
    context: &mut ValidationContext<'_, '_>,
    navegacao: Option<NavegacaoCarga>,
    navegacao_id: Option<ElementId>,
    carga_id: ElementId,
) {
    match navegacao_id {
        None => context.erro_antes_tags_fechamento(
            "a operação exige <Navegacao>",
            regras::NAVEGACAO_ORIGEM_DESTINO,
            [(carga_id, "informe a modalidade de navegação")],
        ),
        Some(elemento) if navegacao == Some(NavegacaoCarga::NaoInformada) => context
            .erro_nos_valores(
                "a navegação deve usar um código de 1 a 5",
                regras::NAVEGACAO_ORIGEM_DESTINO,
                [(elemento, "0 não identifica uma modalidade")],
            ),
        _ => (),
    }
}

/// Orquestra as relações já implementadas entre operações, sentido e código.
///
/// As regras são independentes: uma incompatibilidade com a atracação não
/// impede verificar o sentido ou o código. A classificação é calculada uma
/// única vez e compartilhada com as regras de representação da carga.
fn validar_regras_da_operacao(
    context: &mut ValidationContext<'_, '_>,
    carga: &Carga<'_>,
    source_map: &CargaSourceMap,
    operacao_atracacao: TipoOperacaoAtracacao,
    classificacao: ClassificacaoCarga,
) {
    validar_adequacao_tipo_de_operacao_carga_atracacao(
        context,
        carga.tipo_operacao_carga,
        source_map.tipo_operacao_carga,
        carga.codigo_carga,
        operacao_atracacao,
    );
    validar_sentido_tipo_de_operacao(
        context,
        carga.sentido.zip(source_map.sentido),
        (carga.tipo_operacao_carga, source_map.tipo_operacao_carga),
        source_map,
    );
    if matches!(
        carga.tipo_operacao_carga,
        TipoOperacaoCarga::Safamento | TipoOperacaoCarga::RemocaoABordo
    ) {
        if classificacao != ClassificacaoCarga::Conteinerizada {
            validar_codigo_em_safamento_ou_remocao(
                context,
                carga.tipo_operacao_carga,
                source_map.tipo_operacao_carga,
                carga.codigo_carga,
                source_map.codigo_carga,
            );
        }
        if carga.natureza_carga != NaturezaCarga::CargaGeral {
            validar_quantidade_em_safamento_ou_remocao(
                context,
                carga.tipo_operacao_carga,
                source_map.tipo_operacao_carga,
                carga.quantidade,
                source_map.element,
            );
        }
    }
}

/// Relaciona a operação da carga à operação declarada na atracação.
///
/// Aplica as restrições existentes para operações principais, apoio e
/// abastecimento, anotando os dois valores envolvidos na incompatibilidade.
fn validar_adequacao_tipo_de_operacao_carga_atracacao(
    context: &mut ValidationContext<'_, '_>,
    operacao_carga: TipoOperacaoCarga,
    operacao_carga_id: ElementId,
    codigo_carga: TextoId,
    operacao_atracacao: TipoOperacaoAtracacao,
) {
    // **ideia**: validar isso para todas as cargas em sequência! Dessa forma será possível reutilizar a mensagem de erro e a ajuda para o caso em que tipo de operação da atracação está errada
    // Seria necessário separar a validação para tipos Misto/Movimentação de Carga e Abastecimento/Apoio
    match (operacao_carga, operacao_atracacao) {
        (
            _,
            TipoOperacaoAtracacao {
                tipo_operacao: TipoOperacao::Misto | TipoOperacao::MovimentacaoDeCarga,
                ..
            },
        ) => return,
        (tipo_operacao_carga, tipo_operacao) if tipo_operacao_carga.eh_principal() => {
            context.erro_com_detalhes(
                format!("o tipo de operação de carga \"{}\" não é compatível com o tipo de operação da atracação \"{}\"", tipo_operacao_carga, tipo_operacao.tipo_operacao),
                regras::OPERACAO_CARGA_ATRACACAO,
                [
                    RotuloDiagnostico::no_valor(
                        tipo_operacao.tipo_operacao_id,
                        format!("ajuda: se o tipo de operação da carga de código {} estiver correto, altere esse valor para {} ou {}", context.texto(codigo_carga), TipoOperacao::MovimentacaoDeCarga, TipoOperacao::Misto)
                    ),
                    RotuloDiagnostico::no_valor(
                        operacao_carga_id,
                        format!("ajuda: se o tipo de operação da atracação ({}) estiver correto, altere esse valor para um tipo de operação auxiliar (3, 5, 8, 10, 11 e 12)", tipo_operacao.tipo_operacao)
                    )
                ]
            );
        }
        (
            tipo_operacao_carga,
            TipoOperacaoAtracacao {
                tipo_operacao,
                tipo_operacao_id,
            },
        ) if [TipoOperacao::Abastecimento, TipoOperacao::Apoio].contains(&tipo_operacao)
            && tipo_operacao_carga != tipo_operacao =>
        {
            context.erro_com_detalhes(
                format!("o tipo de operação de carga \"{}\" é incompatível com o tipo de operação da atracação \"{}\"", tipo_operacao_carga, tipo_operacao),
                regras::OPERACAO_CARGA_ATRACACAO,
                [
                    RotuloDiagnostico::no_valor(tipo_operacao_id, format!("ajuda: se o tipo de operação da carga de código {} estiver correto, altere esse valor", context.texto(codigo_carga))),
                    RotuloDiagnostico::no_valor(operacao_carga_id, format!("ajuda: se o tipo de operação da atração ({}) estiver correto, altere esse valor para {}", tipo_operacao, TipoOperacaoCarga::try_from(tipo_operacao).unwrap())),
                ]
            );
        }
        (_, _) => return,
    }
}

/// Confere os intervalos do código numérico em safamento ou remoção a bordo.
///
/// A orquestração dispensa contêineres, conforme a implementação existente.
/// Um código não convertível não é comparado a intervalos: seu diagnóstico
/// de formato pertence às regras de representação, sem interromper outras regras.
fn validar_codigo_em_safamento_ou_remocao(
    context: &mut ValidationContext<'_, '_>,
    tipo_operacao: TipoOperacaoCarga,
    tipo_operacao_id: ElementId,
    codigo: TextoId,
    codigo_id: ElementId,
) {
    let codigo = context.texto(codigo);
    if let Ok(codigo_carga) = codigo.parse::<u32>()
        && !matches!(codigo_carga, 8701..=8716 | 8401..=8429 | 4401..=4421)
    {
        context.erro_com_detalhes(
            format!("carga com tipo de operação \"{}\" incompatível com código de carga \"{}\"", tipo_operacao, codigo),
            regras::SAFAMENTO_REMOCAO,
            [
                RotuloDiagnostico::no_valor(
                    tipo_operacao_id,
                    format!("ajuda: caso o código {} esteja correto, corrija esse valor para algo diferente de {} e {}", codigo, TipoOperacaoCarga::Safamento, TipoOperacaoCarga::RemocaoABordo),
                ),
                RotuloDiagnostico::no_valor(
                    codigo_id,
                    format!("ajuda: caso o tipo de operação de carga \"{}\" esteja correto, corrija para um número dentro dos intervalos 8701-8716, 8401-8429, 4401-4421", tipo_operacao),
                ),
            ],
        );
    }
}

/// Exige quantidade nos mesmos casos já cobertos pela restrição de código.
///
/// Sua aplicabilidade é decidida por [`validar_regras_da_operacao`]. Para carga
/// geral, a regra comum já exige a quantidade e evita emitir o mesmo erro duas vezes.
/// A ausência é relatada mesmo que o código informado também seja inválido.
fn validar_quantidade_em_safamento_ou_remocao(
    context: &mut ValidationContext<'_, '_>,
    tipo_operacao: TipoOperacaoCarga,
    tipo_operacao_id: ElementId,
    quantidade: Option<u32>,
    carga_id: ElementId,
) {
    if quantidade.is_none() {
        context.erro_com_detalhes(
            "o tipo de operação de carga é incompatível com a ausência da tag <Quantidade>",
            regras::SAFAMENTO_REMOCAO,
            [
                RotuloDiagnostico::no_valor(
                    tipo_operacao_id,
                    "ajuda: se a ausência da tag <Quantidade> estiver correta, altere esse valor",
                ),
                RotuloDiagnostico::antes_tag_fechamento(
                    carga_id,
                    format!("ajuda: se o tipo de operação de carga ({}) estiver correto, adicione a tag <Quantidade> antes desse ponto", tipo_operacao),
                ),
            ],
        );
    }
}

/// Reúne em um único diagnóstico as tags proibidas pela operação.
///
/// A seleção das operações é responsabilidade de [`validar_movimentacao`].
/// Esta regra não é chamada em conjunto com as regras de presença/valor dos
/// mesmos campos. Os valores não são necessários: bastam os IDs das tags.
fn validar_ausencia_de_campos_tipo_operacao(
    context: &mut ValidationContext<'_, '_>,
    tipo_operacao: TipoOperacaoCarga,
    source_map: &CargaSourceMap,
) {
    let campos = [
        ("Navegacao", source_map.navegacao),
        ("CodigoTUPOrigem", source_map.codigo_tup_origem),
        ("CodigoTUPDestino", source_map.codigo_tup_destino),
        ("TrigramaPortoOrigem", source_map.trigrama_porto_origem),
        ("TrigramaPortoDestino", source_map.trigrama_porto_destino),
        ("BigramaPaisOrigem", source_map.bigrama_pais_origem),
        ("BigramaPaisDestino", source_map.bigrama_pais_destino),
        ("CodigoAreaOrigem", source_map.codigo_area_origem),
        ("CodigoAreaDestino", source_map.codigo_area_destino),
    ];

    let mut tags = Vec::new();
    let mut labels = Vec::new();
    for (tag, elemento) in campos {
        let Some(elemento) = elemento else {
            continue;
        };
        tags.push(tag);
        labels.push(RotuloDiagnostico::na_tag_abertura(
            elemento,
            "ajuda: se o tipo de operação de carga estiver correto, remova esta tag",
        ));
    }

    if labels.is_empty() {
        return;
    }
    labels.push(RotuloDiagnostico::no_valor(
        source_map.tipo_operacao_carga,
        "ajuda: se os campos informados estiverem corretos, reveja o tipo de operação",
    ));
    context.erro_com_detalhes(
        format!(
            "o tipo de operação {tipo_operacao} é incompatível com a presença das tags {}",
            tags.join_oxford(),
        ),
        regras::NAVEGACAO_ORIGEM_DESTINO,
        labels,
    );
}

/// Exige quantidade em cargas de natureza geral.
///
/// Chamada por [`validar_carga`] independentemente da representação inferida.
/// Recebe somente a natureza, a quantidade e os alvos necessários ao diagnóstico.
fn validar_presenca_quantidade_carga_geral(
    context: &mut ValidationContext<'_, '_>,
    natureza: NaturezaCarga,
    natureza_id: ElementId,
    quantidade: Option<u32>,
    carga_id: ElementId,
) {
    if matches!(natureza, NaturezaCarga::CargaGeral)
        && let None = quantidade
    {
        context.erro_com_detalhes(
            format!(
                "a carga de natureza {} é incompatível com a ausência da tag <Quantidade>",
                NaturezaCarga::CargaGeral
            ),
            regras::QUANTIDADE_CARGA_GERAL,
            [
                RotuloDiagnostico::no_valor(
                    natureza_id,
                    format!("ajuda: se a natureza não for {}, altere esse valor", NaturezaCarga::CargaGeral),
                ),
                RotuloDiagnostico::antes_tag_fechamento(
                    carga_id,
                    format!("ajuda: se a natureza for {}, adicione a tag <Quantidade> antes desse ponto", NaturezaCarga::CargaGeral),
                )
            ],
        );
    }
}

/// Valida sentido por operação, preservando a diferença entre zero e tag ausente.
/// Abastecimento, safamento e remoção aceitam ausência/zero; as demais exigem 1/2.
fn validar_sentido_tipo_de_operacao(
    context: &mut ValidationContext<'_, '_>,
    sentido: Option<(Sentido, ElementId)>,
    (operacao, operacao_id): (TipoOperacaoCarga, ElementId),
    mapa: &CargaSourceMap,
) {
    let sem_sentido = matches!(
        operacao,
        TipoOperacaoCarga::Abastecimento
            | TipoOperacaoCarga::Safamento
            | TipoOperacaoCarga::RemocaoABordo
    );
    let (valor, id) = sentido.unzip();
    let valor = valor.unwrap_or_default();
    if (valor == Sentido::NaoInformado) == sem_sentido {
        return;
    }
    let ajuda = if sem_sentido {
        "ajuda: omita <Sentido> ou informe 0"
    } else {
        "ajuda: informe <Sentido> com valor 1 (desembarque) ou 2 (embarque)"
    };
    let alvo = match id {
        Some(id) => RotuloDiagnostico::no_valor(id, ajuda),
        None => RotuloDiagnostico::antes_tag_fechamento(mapa.element, ajuda),
    };
    let mensagem = if id.is_some() {
        format!("o tipo de operação \"{operacao}\" e o sentido \"{valor}\" são incompatíveis")
    } else {
        format!("o tipo de operação \"{operacao}\" é incompatível com a ausência da tag <Sentido>")
    };
    context.erro_com_detalhes(
        mensagem,
        regras::SENTIDO_POR_OPERACAO,
        [
            RotuloDiagnostico::no_valor(
                operacao_id,
                "ajuda: reveja a operação se o sentido estiver correto",
            ),
            alvo,
        ],
    );
}

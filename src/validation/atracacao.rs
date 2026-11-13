//! Regras semânticas aplicáveis a uma atracação.
//!
//! Os enunciados gerais dos diagnósticos são compartilhados por
//! [`crate::validation::regras_de_negocio`]; mensagens e labels descrevem cada ocorrência.
//!
//! [`validar_atracacao`] é a função orquestradora da entidade. Cada classe de
//! validação permanece agrupada em uma função agregadora privada, seguida de
//! suas regras específicas.
//! As agregadoras recebem a atracação e seu mapa; as regras locais de formato
//! recebem apenas texto e `ElementId`. Regras relacionais pequenas recebem os
//! campos envolvidos, enquanto cronologia e passageiros conservam os modelos
//! para reunir seus vários campos sem assinaturas extensas. Helpers de formato
//! retornam erros e não registram diagnósticos por conta própria.
//!
//! As funções registram violações em [`ValidationContext`] e não modificam os
//! modelos. Valores e mapas devem pertencer ao mesmo documento e continuar
//! alinhados. A passagem pelas cargas é feita por [`Validador`](crate::validation::Validador), não
//! por este módulo. Paralisações e resíduos têm validadores próprios.
//!
//! # Fonte normativa
//!
//! Este checklist reproduz o *Manual do Usuário Externo do sistema SDP*, 5ª
//! edição, versão 1.0, dezembro de 2025: seções 2.2.1 (pp. 9–11), 2.2.2
//! (pp. 11–12), 3.1–3.2 (pp. 19–20) e 9.2.1 (pp. 40–42). O checklist contém
//! somente regras comprováveis a partir do XML e dos conjuntos fechados
//! incorporados ao programa.
//!
//! # Checklist normativo
//!
//! ## Instalação informante — Manual §2.1 e §2.2.1
//!
//! - [x] usar `<TrigramaPorto>` no leiaute de porto organizado e `<CodigoTUP>`
//!   no leiaute de instalação autorizada;
//! - [x] exigir três caracteres em `TrigramaPorto`;
//! - [x] exigir que os três caracteres do trigrama sejam letras ASCII maiúsculas;
//! - [x] exigir em `CodigoTUP` o bigrama `BR`, a UF e três algarismos, como
//!   `BRCE001`.
//!
//! ## Atracação, local e embarcação — Manual §2.2.1
//!
//! - [x] exigir no máximo 15 caracteres alfanuméricos em `NumeroAtracacao`;
//! - [x] rejeitar outro `NumeroAtracacao` igual no mesmo ano e local de
//!   atracação dentro do documento;
//! - [x] exigir exatamente quatro caracteres alfanuméricos em
//!   `LocalAtracacao`;
//! - [x] aceitar em `IMON` somente `0` ou sete algarismos;
//! - [x] exigir ao menos um identificador da embarcação: IMO diferente de zero
//!   e/ou `NumeroInscricaoCapitania` presente;
//! - [x] exigir dez algarismos em `NumeroInscricaoCapitania` e representar sua
//!   ausência removendo a tag;
//! - [x] representar IMO não informado pelo valor canônico `0`, mantendo a tag;
//!
//! O Manual exige apenas sete dígitos para o IMO e não menciona dígito
//! verificador. Ele também não descreve `NREscala` na seção de atracação;
//! portanto, nenhuma dessas duas regras deve ser inferida nesta fonte.
//!
//! ## Navegação, operação e responsáveis — Manual §2.2.1 e §3.1
//!
//! - [x] exigir `CnpjAgente` sem máscara e com 14 caracteres;
//! - [x] aceitar em `NacionalidadeArmador` somente brasileiro (1) ou
//!   estrangeiro (2), o que já é garantido pelo reader;
//! - [x] exigir CNPJ para armador brasileiro e código de armador para
//!   estrangeiro.
//!
//! ## Passageiros — Manual §2.2.1
//!
//! - [x] permitir as três tags de quantidade apenas em operações de
//!   passageiros (2) ou mistas (7);
//! - [x] nessas operações, exigir que ao menos uma das quantidades de
//!   passageiros embarcados, desembarcados ou em trânsito esteja presente e
//!   seja maior que zero. A exigência de positividade é uma decisão explícita
//!   do projeto; o Manual usa apenas a formulação “e/ou”.
//!
//! ## Dados temporais — Manual §2.2.2
//!
//! - [x] garantir `chegada <= atracação < início da operação < término da
//!   operação < desatracação`, conforme a revisão local anotada no projeto;
//! - [x] exigir início e término da operação para movimentação de carga (1) e
//!   operação mista (7);
//! - [x] em apoio (3) e abastecimento (5), permitir início e término somente
//!   quando houver cargas movimentadas;
//! - [x] delegar as paralisações a `validation::tempos_descontos`.
//!
//! ## Cargas associadas — Manual §3.1–3.2
//!
//! - [x] exigir ao menos uma carga para movimentação de carga (1) e operação
//!   mista (7);
//! - [x] em apoio (3) e abastecimento (5), exigir carga quando forem
//!   informados início e término da operação;
//! - [x] percorrer as cargas e seus mapas laterais chamando as regras de
//!   `validation::carga`.
//!
//! ## Coleta de resíduos — Manual §2.2.1 e §9.2.1
//!
//! - [x] aceitar em `FlagPossuiColetaResiduo` exclusivamente `0` ou `1`;
//! - [x] quando a flag for `1`, exigir ao menos um bloco completo
//!   `<ColetaResiduo>`; quando for `0`, proibir esses blocos;
//! - [x] modelar e validar os campos obrigatórios de `<ColetaResiduo>` e ao
//!   menos um `<Residuo>` por coleta;
//! - [x] não relacionar a flag a `TipoOperacao::RetiradaDeResiduos`: o Manual
//!   descreve a flag como ocorrência de coleta durante qualquer atracação.

use std::fmt;

use crate::{
    interner::{Interner, TextoId},
    validation::helper::validar_codigo_alfabetico,
    xml::{
        models::{
            atracacao::Atracacao,
            documento::LeiauteSdp,
            enums::{NacionalidadeArmador, TipoOperacao},
        },
        reader::{AtracacaoSourceMap, ElementId},
    },
};

use super::{
    context::{RotuloDiagnostico, ValidationContext},
    helper::{
        ErroFormatoTexto, validar_alfanumerico_com_comprimento_maximo, validar_cnpj,
        validar_codigo_alfanumerico, validar_codigo_numerico,
    },
    regras_de_negocio as regras,
};

/// Operação da atracação e sua origem no XML, compartilhadas com as regras da carga.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TipoOperacaoAtracacao {
    /// Valor declarado na tag `<TipoOperacao>` da atracação.
    pub tipo_operacao: TipoOperacao,
    /// Elemento do valor usado em diagnósticos de compatibilidade entre operações.
    pub tipo_operacao_id: ElementId,
}

/// Identificador único declarado pela atracação, acompanhado da origem no XML.
///
/// Não certifica a validade do código nem sua conformidade com o leiaute. A
/// orquestração só constrói este valor quando exatamente um identificador existe.
/// Os IDs não emprestam o interner nem o XML. A apresentação textual requer o
/// interner de origem, fornecido a [`Self::exibir`], em vez de um `Display`
/// implementado diretamente para a enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InstalacaoInformante {
    /// Porto organizado identificado pelo trigrama e pelo elemento de origem.
    Porto {
        /// Trigrama declarado em `<TrigramaPorto>`.
        trigrama: TextoId,
        /// Elemento `<TrigramaPorto>` correspondente.
        elemento: ElementId,
    },
    /// Terminal de uso privado identificado pelo código e pelo elemento de origem.
    Tup {
        /// Código declarado em `<CodigoTUP>`.
        codigo: TextoId,
        /// Elemento `<CodigoTUP>` correspondente.
        elemento: ElementId,
    },
}

impl InstalacaoInformante {
    /// Cria um adaptador que exibe `Porto <trigrama>` ou `TUP <codigo>`.
    ///
    /// Use `format!("{}", informante.exibir(&interner))` para materializar a
    /// mensagem, ou `write!` para escrever diretamente no destino. A criação
    /// do adaptador resolve o ID uma vez e não aloca nem copia o texto.
    /// Valores com formato inválido são preservados para os diagnósticos.
    ///
    /// O retorno captura somente o texto emprestado por `'xml`, não `self`
    /// nem o empréstimo do interner. Assim, a tabela pode receber novos textos
    /// enquanto o adaptador existir. `use<'xml>` explicita essa captura no
    /// retorno opaco, inclusive na edição 2024.
    ///
    /// # Panics
    ///
    /// Entra em pânico se o ID não existir na tabela. O chamador deve fornecer
    /// o interner que gerou o ID; um ID numericamente válido de outra tabela
    /// pode resolver outro texto sem que essa troca seja detectada.
    pub(super) fn exibir<'xml>(
        &self,
        interner: &Interner<'xml>,
    ) -> impl fmt::Display + 'xml + use<'xml> {
        let (tipo, id) = match *self {
            Self::Porto { trigrama, .. } => ("Porto", trigrama),
            Self::Tup { codigo, .. } => ("TUP", codigo),
        };
        let texto = interner.resolve(id);
        fmt::from_fn(move |formatter| write!(formatter, "{tipo} {texto}"))
    }
}

/// Executa todas as classes de validação aplicáveis a uma atracação.
///
/// `atracacao` fornece os valores tipados; `source_map` associa seus campos a
/// elementos XML; `context` resolve esses elementos e acumula diagnósticos.
///
/// Não retorna um modelo validado nem interrompe a execução na primeira
/// violação. Campos ausentes impedem somente comparações que dependem deles.
///
/// # Panics
///
/// Pode entrar em pânico se campos presentes perderem seus IDs de origem.
/// Essa inconsistência indica alteração indevida do pareamento criado pelo reader.
pub(super) fn validar_atracacao(
    context: &mut ValidationContext<'_, '_>,
    atracacao: &Atracacao<'_>,
    source_map: &AtracacaoSourceMap,
) {
    validar_conformidade_leiaute(context, atracacao, source_map);
    validar_identificacao_embarcacao(context, atracacao, source_map);
    validar_responsaveis(context, atracacao, source_map);
    validar_passageiros(context, atracacao, source_map);
    validar_numero_atracacao(
        context,
        atracacao.numero_atracacao,
        source_map.numero_atracacao,
    );
    validar_local_atracacao(
        context,
        atracacao.local_atracacao,
        source_map.local_atracacao,
    );
    validar_presenca_carga(
        context,
        TipoOperacaoAtracacao {
            tipo_operacao: atracacao.tipo_operacao,
            tipo_operacao_id: source_map.tipo_operacao,
        },
        !atracacao.cargas.is_empty(),
        source_map.element,
    );
    validar_presenca_datas_operacao(context, atracacao, source_map);
    validar_cronologia(context, atracacao, source_map);
}

/// Executa as regras condicionadas pelo leiaute declarado em `<Movimento>`.
///
/// Esta função agrupa as classes de conformidade do leiaute sem interromper
/// as demais classes quando uma delas registra um erro. Cada regra dependente
/// de um campo ausente é responsável por se omitir localmente.
fn validar_conformidade_leiaute(
    context: &mut ValidationContext<'_, '_>,
    atracacao: &Atracacao<'_>,
    source_map: &AtracacaoSourceMap,
) {
    validar_conformidade_identificador_instalacao(context, atracacao, source_map);
}

/// Valida a conformidade do identificador da instalação informante.
///
/// A classe verifica tags incompatíveis, presença da tag exigida e o formato
/// dos identificadores existentes. As regras são executadas de forma
/// independente para acumular todos os problemas objetivos do leiaute.
fn validar_conformidade_identificador_instalacao(
    context: &mut ValidationContext<'_, '_>,
    atracacao: &Atracacao<'_>,
    source_map: &AtracacaoSourceMap,
) {
    validar_adequacao_identificador_instalacao_ao_leiaute(
        context,
        source_map.trigrama_porto,
        source_map.codigo_tup,
    );
    validar_presenca_identificador_instalacao(
        context,
        source_map.element,
        source_map.trigrama_porto,
        source_map.codigo_tup,
    );
    if let Some((trigrama, elemento)) = atracacao.trigrama_porto.zip(source_map.trigrama_porto) {
        validar_trigrama_porto(context, trigrama, elemento);
    }
    if let Some((codigo, elemento)) = atracacao.codigo_tup.zip(source_map.codigo_tup) {
        validar_codigo_tup(context, codigo, elemento);
    }
}

/// Detecta identificadores mutuamente exclusivos ou incompatíveis com o leiaute.
///
/// O diagnóstico não escolhe qual declaração deve ser alterada. Quando os
/// dois identificadores aparecem, anota ambas as tags; quando apenas o
/// identificador do outro leiaute aparece, anota-o junto da tag raiz que
/// declara o namespace.
fn validar_adequacao_identificador_instalacao_ao_leiaute(
    context: &mut ValidationContext<'_, '_>,
    trigrama_porto: Option<ElementId>,
    codigo_tup: Option<ElementId>,
) {
    if let (Some(trigrama), Some(codigo_tup)) = (trigrama_porto, codigo_tup) {
        context.erro_com_detalhes(
            "<TrigramaPorto> e <CodigoTUP> são identificadores de instalação mutuamente exclusivos",
            regras::IDENTIFICADOR_INSTALACAO,
            [
                RotuloDiagnostico::na_tag_abertura(
                    trigrama,
                    "ajuda: remova esta tag se a instalação informante for o TUP indicado por <CodigoTUP>",
                ),
                RotuloDiagnostico::na_tag_abertura(
                    codigo_tup,
                    "ajuda: remova esta tag se a instalação informante for o porto indicado por <TrigramaPorto>",
                ),
            ],
        );
        return;
    }

    let incompatibilidade = match context.leiaute() {
        LeiauteSdp::Porto => codigo_tup.map(|elemento| {
            (
                LeiauteSdp::NAMESPACE_PORTO,
                LeiauteSdp::NAMESPACE_TUP,
                elemento,
                "<TrigramaPorto>",
            )
        }),
        LeiauteSdp::Tup => trigrama_porto.map(|elemento| {
            (
                LeiauteSdp::NAMESPACE_TUP,
                LeiauteSdp::NAMESPACE_PORTO,
                elemento,
                "<CodigoTUP>",
            )
        }),
    };

    if let Some((namespace, namespace_alternativo, identificador, tag_esperada)) = incompatibilidade
    {
        context.erro_com_detalhes(
            "o leiaute declarado e o identificador da instalação são incompatíveis",
            regras::IDENTIFICADOR_INSTALACAO,
            [
                RotuloDiagnostico::na_tag_abertura(
                    context.movimento(),
                    format!(
                        "ajuda: use o namespace `{namespace_alternativo}` se o identificador da instalação estiver correto"
                    ),
                ),
                RotuloDiagnostico::na_tag_abertura(
                    identificador,
                    format!(
                        "ajuda: substitua esta tag por {tag_esperada} se o namespace `{namespace}` estiver correto"
                    ),
                ),
            ],
        );
    }
}

/// Exige o identificador de instalação correspondente ao leiaute.
///
/// Como uma tag ausente não possui [`crate::xml::reader::ElementId`], o
/// diagnóstico relaciona o leiaute declarado na raiz ao ponto de inserção no
/// fim da `<Atracacao>`. A ausência não gera um segundo erro quando o
/// identificador do outro leiaute existe: essa incompatibilidade já é
/// explicada por
/// [`validar_adequacao_identificador_instalacao_ao_leiaute`].
fn validar_presenca_identificador_instalacao(
    context: &mut ValidationContext<'_, '_>,
    atracacao_id: ElementId,
    trigrama_porto: Option<ElementId>,
    codigo_tup: Option<ElementId>,
) {
    let missing = match context.leiaute() {
        LeiauteSdp::Porto if trigrama_porto.is_none() && codigo_tup.is_none() => Some((
            "<TrigramaPorto>",
            LeiauteSdp::NAMESPACE_PORTO,
            "nenhum identificador de instalação aparece nesta atracação",
        )),
        LeiauteSdp::Tup if codigo_tup.is_none() && trigrama_porto.is_none() => Some((
            "<CodigoTUP>",
            LeiauteSdp::NAMESPACE_TUP,
            "nenhum identificador de instalação aparece nesta atracação",
        )),
        _ => None,
    };

    if let Some((tag, namespace, mensagem_ausencia)) = missing {
        context.erro_com_detalhes(
            format!("o leiaute `{namespace}` e a ausência de {tag} são incompatíveis"),
            regras::IDENTIFICADOR_INSTALACAO,
            [
                RotuloDiagnostico::na_tag_abertura(
                    context.movimento(),
                    format!("o namespace desta tag declara o leiaute `{namespace}`"),
                ),
                RotuloDiagnostico::antes_tag_fechamento(
                    atracacao_id,
                    format!(
                        "ajuda: talvez você queira inserir {tag} antes deste ponto; {mensagem_ausencia}"
                    ),
                ),
            ],
        );
    }
}

/// Exige três letras ASCII maiúsculas no trigrama de porto existente.
///
/// Recebe somente um valor presente e seu elemento. A orquestração seleciona
/// os campos existentes; a ausência pertence a [`validar_presenca_identificador_instalacao`].
fn validar_trigrama_porto(
    context: &mut ValidationContext<'_, '_>,
    trigrama_porto: TextoId,
    trigrama_porto_id: ElementId,
) {
    if let Err(erro) = validar_codigo_alfabetico::<3>(context.texto(trigrama_porto)) {
        context.erro_nos_valores(
            "o valor de <TrigramaPorto> deve conter exatamente três caracteres alfabéticos em letra maiúscula",
            regras::TRIGRAMA_PORTO,
            [(trigrama_porto_id, erro.to_string())],
        );
    }
}

/// Executa as regras relativas à identificação da embarcação.
///
/// A função mantém essa classe de regras separada da conformidade do leiaute
/// da instalação informante. Ela coordena a relação entre os identificadores,
/// chama os validadores individuais e transforma os erros específicos
/// retornados por eles em diagnósticos associados aos elementos XML.
fn validar_identificacao_embarcacao(
    context: &mut ValidationContext<'_, '_>,
    atracacao: &Atracacao<'_>,
    source_map: &AtracacaoSourceMap,
) {
    if let Err(error) = validar_numero_imo(atracacao.imon) {
        context.erro_nos_valores(
            "<IMON> deve ser 0 ou conter exatamente 7 algarismos",
            regras::NUMERO_IMO,
            [(source_map.imon, error.to_string())],
        );
    }

    if let Some(numero) = atracacao.numero_inscricao_capitania {
        let element = source_map
            .numero_inscricao_capitania
            .expect("número da Capitania presente deve possuir ElementId");
        if let Err(error) = validar_numero_inscricao_capitania(numero) {
            context.erro_nos_valores(
                "<NumeroInscricaoCapitania> deve conter exatamente 10 algarismos",
                regras::NUMERO_INSCRICAO_CAPITANIA,
                [(element, error.to_string())],
            );
        }
    }

    let imon_is_zero =
        !atracacao.imon.is_empty() && atracacao.imon.bytes().all(|byte| byte == b'0');
    if imon_is_zero && atracacao.numero_inscricao_capitania.is_none() {
        context.erro_com_detalhes(
            "<IMON> e a ausência de <NumeroInscricaoCapitania> são incompatíveis",
            regras::IDENTIFICACAO_EMBARCACAO,
            [
                RotuloDiagnostico::no_valor(
                    source_map.imon,
                    "ajuda: informe aqui o número IMO da embarcação, se ela o possuir",
                ),
                RotuloDiagnostico::antes_tag_fechamento(
                    source_map.element,
                    "ajuda: talvez você queira inserir <NumeroInscricaoCapitania> antes deste ponto",
                ),
            ],
        );
    }
}

/// Motivo específico pelo qual o texto de um número IMO é inválido.
///
/// Os erros lexicais comuns são preservados em [`ErroFormatoTexto`]; esta enum
/// acrescenta apenas a regra exclusiva da sentinela zero de `<IMON>`.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ErroNumeroImo {
    /// O texto não contém exatamente sete algarismos ASCII.
    Formato(ErroFormatoTexto),
    /// Representação não canônica da sentinela zero de `<IMON>`.
    ZeroNaoCanonico,
}

impl std::fmt::Display for ErroNumeroImo {
    /// Descreve a falha lexical ou a grafia não canônica do IMO não informado.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Formato(erro) => erro.fmt(formatter),
            Self::ZeroNaoCanonico => {
                formatter.write_str("um IMO não informado deve ser representado exatamente por 0")
            }
        }
    }
}

/// Valida a representação textual de um número IMO.
///
/// `"0"` é a sentinela de IMO não informado. Os demais valores devem conter
/// exatamente sete algarismos ASCII. O Manual do SDP não estabelece uma regra
/// de dígito verificador para este campo.
///
/// # Errors
///
/// Retorna [`ErroNumeroImo::ZeroNaoCanonico`] para sequências não vazias de
/// zeros diferentes de `"0"`; nos demais casos, preserva o erro específico de
/// comprimento ou caractere inválido. Não consulta o cadastro de embarcações.
fn validar_numero_imo(value: &str) -> Result<(), ErroNumeroImo> {
    if value == "0" {
        return Ok(());
    }

    if !value.is_empty() && value.bytes().all(|byte| byte == b'0') {
        return Err(ErroNumeroImo::ZeroNaoCanonico);
    }

    validar_codigo_numerico::<7>(value).map_err(ErroNumeroImo::Formato)
}

/// Valida a representação textual do número de inscrição na Capitania.
///
/// Esta função é chamada somente quando a tag opcional existe. Seu valor deve
/// conter exatamente dez algarismos ASCII; a ausência da informação não usa
/// uma sentinela textual e é representada pela ausência da própria tag.
///
/// # Errors
///
/// Retorna comprimento incorreto ou o primeiro caractere não numérico. Não
/// verifica cadastro ou dígito verificador e não proíbe dez zeros nesta regra.
fn validar_numero_inscricao_capitania(value: &str) -> Result<(), ErroFormatoTexto> {
    validar_codigo_numerico::<10>(value)
}

/// Compara cada par de datas disponível, sem inventar datas para campos ausentes.
///
/// Percorre chegada, atracação, início, término e desatracação nessa ordem,
/// comparando datas consecutivas após descartar ausências. A chegada pode
/// coincidir com a atracação; os demais eventos são estritamente crescentes
/// conforme a revisão local. A data prevista não participa desta ordenação.
fn validar_cronologia(
    context: &mut ValidationContext<'_, '_>,
    atracacao: &Atracacao<'_>,
    source_map: &AtracacaoSourceMap,
) {
    let datas = [
        (
            "DataHoraChegada",
            Some(atracacao.data_hora_chegada),
            Some(source_map.data_hora_chegada),
        ),
        (
            "DataHoraAtracacao",
            Some(atracacao.data_hora_atracacao),
            Some(source_map.data_hora_atracacao),
        ),
        (
            "DataHoraInicioOperacao",
            atracacao.data_hora_inicio_operacao,
            source_map.data_hora_inicio_operacao,
        ),
        (
            "DataHoraTerminoOperacao",
            atracacao.data_hora_termino_operacao,
            source_map.data_hora_termino_operacao,
        ),
        (
            "DataHoraDesatracacao",
            Some(atracacao.data_hora_desatracacao),
            Some(source_map.data_hora_desatracacao),
        ),
    ];
    let mut anterior = None;
    for (nome, valor, elemento) in datas {
        let (Some(valor), Some(elemento)) = (valor, elemento) else {
            continue;
        };
        if let Some((nome_anterior, valor_anterior, elemento_anterior)) = anterior {
            let igualdade_permitida =
                nome_anterior == "DataHoraChegada" && nome == "DataHoraAtracacao";
            if valor_anterior > valor || (valor_anterior == valor && !igualdade_permitida) {
                context.erro_nos_valores(
                    format!(
                        "<{nome_anterior}> deve ser anterior{} a <{nome}>",
                        if igualdade_permitida { " ou igual" } else { "" }
                    ),
                    regras::pendentes::CRONOLOGIA_ESTRITA,
                    [
                        (elemento_anterior, "data anterior na sequência"),
                        (elemento, "data posterior na sequência"),
                    ],
                );
            }
        }
        anterior = Some((nome, valor, elemento));
    }
}

/// Aplica as condições de presença das datas operacionais previstas no Manual.
fn validar_presenca_datas_operacao(
    context: &mut ValidationContext<'_, '_>,
    atracacao: &Atracacao<'_>,
    source_map: &AtracacaoSourceMap,
) {
    let apoio = matches!(
        atracacao.tipo_operacao,
        TipoOperacao::Apoio | TipoOperacao::Abastecimento
    );
    let obrigatorias = matches!(
        atracacao.tipo_operacao,
        TipoOperacao::MovimentacaoDeCarga | TipoOperacao::Misto
    ) || (apoio && !atracacao.cargas.is_empty());
    for (nome, elemento) in [
        (
            "DataHoraInicioOperacao",
            source_map.data_hora_inicio_operacao,
        ),
        (
            "DataHoraTerminoOperacao",
            source_map.data_hora_termino_operacao,
        ),
    ] {
        match elemento {
            None if obrigatorias => context.erro_antes_tags_fechamento(
                format!("a operação exige <{nome}>"),
                regras::DATAS_E_CARGAS_POR_OPERACAO,
                [(source_map.element, format!("informe <{nome}>"))],
            ),
            Some(elemento) if apoio && atracacao.cargas.is_empty() => context
                .erro_nas_tags_abertura(
                    format!("<{nome}> exige registro de cargas em apoio ou abastecimento"),
                    regras::DATAS_E_CARGAS_POR_OPERACAO,
                    [(
                        elemento,
                        "confira as cargas movimentadas e a data operacional",
                    )],
                ),
            _ => (),
        }
    }
}

/// Executa as regras aplicáveis aos responsáveis informados na atracação.
///
/// As regras desta classe são independentes das regras de leiaute e, por isso,
/// continuam sendo executadas mesmo quando o identificador da instalação
/// informante é inválido.
fn validar_responsaveis(
    context: &mut ValidationContext<'_, '_>,
    atracacao: &Atracacao<'_>,
    source_map: &AtracacaoSourceMap,
) {
    validar_cnpj_agente(context, atracacao.cnpj_agente, source_map.cnpj_agente);
    validar_armador(
        context,
        atracacao.nacionalidade_armador,
        source_map.nacionalidade_armador,
        atracacao.cnpj_codigo_armador,
        source_map.cnpj_codigo_armador,
    );
}

/// Valida o CNPJ do agente marítimo informado em `<CnpjAgente>`.
///
/// A tag é estruturalmente obrigatória e já foi associada ao respectivo
/// [`crate::xml::reader::ElementId`] pelo reader. Esta regra delega apenas a
/// validação do valor ao
/// helper [`validar_cnpj`], preservando aqui o conhecimento de qual campo da
/// atracação contém o CNPJ do agente.
fn validar_cnpj_agente(
    context: &mut ValidationContext<'_, '_>,
    cnpj: TextoId,
    elemento: ElementId,
) {
    if let Err(erro) = super::helper::validar_comprimento_exato::<14>(context.texto(cnpj)) {
        context.erro_nos_valores(
            "<CnpjAgente> deve conter 14 caracteres sem máscara",
            regras::pendentes::CNPJ_AGENTE_SEM_MASCARA,
            [(elemento, erro.to_string())],
        );
        return;
    }
    let Err((_, label)) = validar_cnpj(context.texto(cnpj)) else {
        return;
    };

    context.erro_nos_valores(
        "o valor de <CnpjAgente> não representa um CNPJ válido",
        regras::FORMACAO_CNPJ,
        [(elemento, label)],
    );
}

/// Valida o identificador do armador de acordo com sua nacionalidade.
///
/// Um armador brasileiro deve possuir CNPJ válido; um armador estrangeiro não
/// pode ser identificado por um valor que também seja um CNPJ válido. Quando
/// há incompatibilidade, o diagnóstico relaciona os valores de
/// `<NacionalidadeArmador>` e `<CnpjCodigoArmador>` sem atribuir o problema a
/// apenas um deles.
///
/// Esta regra verifica somente a compatibilidade entre a nacionalidade e a
/// natureza do identificador. O formato próprio dos códigos estrangeiros será
/// tratado separadamente.
fn validar_armador(
    context: &mut ValidationContext<'_, '_>,
    nacionalidade: NacionalidadeArmador,
    nacionalidade_id: ElementId,
    codigo_armador: TextoId,
    codigo_armador_id: ElementId,
) {
    match (nacionalidade, validar_cnpj(context.texto(codigo_armador))) {
        (NacionalidadeArmador::Brasileiro, Err((_, reason))) => {
            context.erro_com_detalhes(
                "os valores de <NacionalidadeArmador> e <CnpjCodigoArmador> são incompatíveis",
                regras::IDENTIFICACAO_ARMADOR,
                [
                    RotuloDiagnostico::no_valor(
                        nacionalidade_id,
                        "ajuda: altere a nacionalidade se o identificador informado não for um CNPJ brasileiro",
                    ),
                    RotuloDiagnostico::no_valor(
                        codigo_armador_id,
                        format!(
                            "ajuda: corrija o CNPJ se a nacionalidade brasileira estiver certa; {reason}"
                        ),
                    ),
                ],
            );
        }
        (NacionalidadeArmador::Estrangeiro, Ok(())) => {
            context.erro_com_detalhes(
                "os valores de <NacionalidadeArmador> e <CnpjCodigoArmador> são incompatíveis",
                regras::IDENTIFICACAO_ARMADOR,
                [
                    RotuloDiagnostico::no_valor(
                        nacionalidade_id,
                        "ajuda: altere a nacionalidade se este CNPJ pertencer a um armador brasileiro",
                    ),
                    RotuloDiagnostico::no_valor(
                        codigo_armador_id,
                        "ajuda: informe o código estrangeiro do armador se a nacionalidade estiver certa",
                    ),
                ],
            );
        }
        _ => {}
    }
}

/// Exige carga nas operações de movimentação e mistas.
fn validar_presenca_carga(
    context: &mut ValidationContext<'_, '_>,
    operacao: TipoOperacaoAtracacao,
    possui_cargas: bool,
    atracacao_id: ElementId,
) {
    let tipo_operacao = operacao.tipo_operacao;
    match tipo_operacao {
        TipoOperacao::MovimentacaoDeCarga | TipoOperacao::Misto => {
            if !possui_cargas {
                context.erro_com_detalhes(
                    "<TipoOperacao> e a ausência de <Carga> são incompatíveis",
                    regras::DATAS_E_CARGAS_POR_OPERACAO,
                    [
                        RotuloDiagnostico::no_valor(
                            operacao.tipo_operacao_id,
                            "ajuda: altere o tipo da operação se esta atracação não movimentou cargas",
                        ),
                        RotuloDiagnostico::antes_tag_fechamento(
                            atracacao_id,
                            "ajuda: talvez você queira inserir uma <Carga> antes deste ponto",
                        ),
                    ],
                );
            }
        }
        _ => (),
    }

    return;
}

/// Relaciona presença e positividade das quantidades ao tipo de operação.
///
/// Passageiros e misto exigem pelo menos uma quantidade positiva; os outros
/// tipos proíbem as três tags, mesmo com zero. Registra uma incompatibilidade
/// conjunta e mantém labels nas quantidades presentes ou no ponto de inserção.
///
/// # Panics
///
/// Entra em pânico se uma quantidade presente não possuir ID no mapa paralelo.
fn validar_passageiros(
    context: &mut ValidationContext<'_, '_>,
    atracacao: &Atracacao<'_>,
    source_map: &AtracacaoSourceMap,
) {
    let tipo_operacao = atracacao.tipo_operacao;
    let campos_passageiros = [
        (
            atracacao.quantidade_passageiros_desembarcados,
            source_map.quantidade_passageiros_desembarcados,
            "<QuantidadePassageirosDesembarcados>",
        ),
        (
            atracacao.quantidade_passageiros_embarcados,
            source_map.quantidade_passageiros_embarcados,
            "<QuantidadePassageirosEmbarcados>",
        ),
        (
            atracacao.quantidade_passageiros_transito,
            source_map.quantidade_passageiros_transito,
            "<QuantidadePassageirosTransito>",
        ),
    ];

    match tipo_operacao {
        TipoOperacao::Passageiros | TipoOperacao::Misto => {
            let possui_passageiros = campos_passageiros
                .iter()
                .any(|(quantidade, _, _)| quantidade.is_some_and(|valor| valor > 0));

            if !possui_passageiros {
                let mut labels = vec![RotuloDiagnostico::no_valor(
                    source_map.tipo_operacao,
                    "ajuda: altere o tipo da operação se esta atracação não movimentou passageiros",
                )];

                for (quantidade, elemento, tag) in campos_passageiros {
                    if let Some(valor) = quantidade {
                        let elemento =
                            elemento.expect("campo de passageiros presente deve possuir ElementId");
                        labels.push(RotuloDiagnostico::no_valor(
                            elemento,
                            format!(
                                "ajuda: informe em {tag} uma quantidade maior que zero se o tipo da operação estiver certo; o valor atual é {valor}"
                            ),
                        ));
                    }
                }

                if labels.len() == 1 {
                    labels.push(RotuloDiagnostico::antes_tag_fechamento(
                        source_map.element,
                        "ajuda: talvez você queira inserir uma tag de quantidade de passageiros maior que zero antes deste ponto",
                    ));
                }

                context.erro_com_detalhes(
                    "<TipoOperacao> e as quantidades de passageiros são incompatíveis",
                    regras::QUANTIDADES_PASSAGEIROS,
                    labels,
                );
            }
        }
        _ => {
            let mut labels = vec![RotuloDiagnostico::no_valor(
                source_map.tipo_operacao,
                "ajuda: use o tipo Passageiros ou Misto se as quantidades de passageiros estiverem corretas",
            )];

            for (quantidade, elemento, tag) in campos_passageiros {
                if quantidade.is_some() {
                    let elemento =
                        elemento.expect("campo de passageiros presente deve possuir ElementId");
                    labels.push(RotuloDiagnostico::na_tag_abertura(
                        elemento,
                        format!("ajuda: remova {tag} se o tipo da operação estiver correto"),
                    ));
                }
            }

            if labels.len() > 1 {
                context.erro_com_detalhes(
                    "<TipoOperacao> e as tags de quantidade de passageiros são incompatíveis",
                    regras::QUANTIDADES_PASSAGEIROS,
                    labels,
                );
            }
        }
    }
}

/// Valida o formato do identificador de TUP informado.
///
/// Inclui o comprimento de sete caracteres; não há outra regra de comprimento
/// executada em paralelo, evitando duplicar o diagnóstico do mesmo formato.
/// A orquestração só chama esta regra quando o valor e seu ID estão presentes.
fn validar_codigo_tup(
    context: &mut ValidationContext<'_, '_>,
    codigo_tup: TextoId,
    codigo_tup_id: ElementId,
) {
    if let Err(erro) = validar_formato_codigo_tup(context.texto(codigo_tup)) {
        context.erro_nos_valores(
            "o valor em <CodigoTUP> deve seguir o padrão `BRLLNNN`, em que `L` é uma letra ASCII maiúscula e `N` é um algarismo",
            regras::CODIGO_TUP,
            [(codigo_tup_id, erro)]
        );
    }
}

/// Valida o formato lexical de um código de instalação autorizada.
///
/// Exige `BR`, duas letras ASCII maiúsculas e três algarismos. Não verifica
/// se as letras formam uma UF existente nem consulta o cadastro de instalações.
///
/// # Errors
///
/// Retorna o primeiro problema de comprimento, caracteres, prefixo ou posições
/// reservadas a letras e algarismos, em uma mensagem para a label do campo.
fn validar_formato_codigo_tup(valor: &str) -> Result<(), String> {
    validar_codigo_alfanumerico::<7>(valor).map_err(|erro| erro.to_string())?;

    let caracteres: Vec<_> = valor.chars().collect();
    if caracteres[..2] != ['B', 'R'] {
        return Err(String::from(
            "as duas primeiras posições devem formar o prefixo `BR`",
        ));
    }

    if let Some((indice, caractere)) = caracteres[2..4]
        .iter()
        .enumerate()
        .find(|(_, caractere)| !caractere.is_ascii_uppercase())
    {
        return Err(format!(
            "o caractere `{caractere}` na posição {} não é uma letra ASCII maiúscula",
            indice + 3
        ));
    }

    if let Some((indice, caractere)) = caracteres[4..]
        .iter()
        .enumerate()
        .find(|(_, caractere)| !caractere.is_ascii_digit())
    {
        return Err(format!(
            "o caractere `{caractere}` na posição {} não é um algarismo ASCII de 0 a 9",
            indice + 5
        ));
    }

    Ok(())
}

/// Confere o formato e o limite do identificador local da atracação.
fn validar_numero_atracacao(
    context: &mut ValidationContext<'_, '_>,
    numero_atracacao: TextoId,
    elemento: ElementId,
) {
    if let Err(erro) =
        validar_alfanumerico_com_comprimento_maximo::<15>(context.texto(numero_atracacao))
    {
        context.erro_nos_valores(
            "o valor em <NumeroAtracacao> deve ter no máximo 15 caracteres alfanuméricos ASCII",
            regras::NUMERO_ATRACACAO,
            [(elemento, erro.to_string())],
        );
    }
}

/// Exige quatro caracteres alfanuméricos no código do local.
fn validar_local_atracacao(
    context: &mut ValidationContext<'_, '_>,
    local_atracacao: TextoId,
    elemento: ElementId,
) {
    if let Err(erro) = validar_codigo_alfanumerico::<4>(context.texto(local_atracacao)) {
        context.erro_nos_valores(
            "o valor em <LocalAtracacao> deve ter exatamente 4 caracteres alfanuméricos ASCII",
            regras::LOCAL_ATRACACAO,
            [(elemento, erro.to_string())],
        );
    }
}

//! Decodificação da árvore XML intermediária para os modelos tipados.
//!
//! Este módulo verifica a forma estrutural esperada — tags permitidas,
//! cardinalidade e tipos lexicais — mas não aplica regras semânticas entre
//! campos, que pertencem ao módulo `validation`.
//!
//! Cada função de entidade produz simultaneamente:
//!
//! 1. o modelo XML tipado com valores emprestados do texto-fonte;
//! 2. o mapa tipado que associa cada campo a um `ElementId`;
//! 3. para relações um-para-muitos, entradas nas tabelas planas de
//!    `SdpSourceMap` e um `SourceMapRange` no mapa pai.
//!
//! Essa construção conjunta garante que o modelo e seu mapa permaneçam na
//! mesma ordem sem copiar spans ou conteúdo textual.
//! As buscas são por filhos diretos e não impõem a ordem de uma sequência XSD.
//! Todos os IDs recebidos pelos helpers devem pertencer ao documento informado.

use std::{fmt, str::FromStr};

use chrono::NaiveDateTime;

use crate::{
    interner::Interner,
    source::Span,
    xml::models::{
        atracacao::{Atracacao, TemposDescontos},
        carga::{Carga, CargaConteinerizada},
        documento::LeiauteSdp,
    },
};

use super::{
    ParsedAtracacoes, XmlParseError,
    document::{XmlDocument, read_document},
    source::{
        AtracacaoSourceMap, CargaConteinerizadaSourceMap, CargaSourceMap, ElementId, SdpSourceMap,
        SourceMapRange, TemposDescontosSourceMap,
    },
};

/// Converte um código numérico em um enum e rejeita discriminantes desconhecidos.
///
/// A implementação de `T::try_from` define o conjunto fechado; `expected`
/// serve apenas para compor a mensagem e não configura valores aceitos.
///
/// # Errors
///
/// Retorna erro se não houver texto, se ele não couber em `u32` ou se o valor
/// não tiver uma variante correspondente em `T`.
fn parse_enum<'xml, T>(
    document: &XmlDocument<'xml>,
    element: ElementId,
    expected: &str,
) -> Result<T, XmlParseError<'xml>>
where
    T: TryFrom<u32>,
{
    let raw = document.text(element)?;
    let value: u32 = document.parse(element, expected)?;
    T::try_from(value).map_err(|_| {
        document.error(
            document.value_span(element),
            format!("valor `{raw}` não pertence a {expected}"),
        )
    })
}

/// Converte uma data ISO, com segundos fracionários e fuso opcionais.
///
/// O formato é `AAAA-MM-DDThh:mm:ss[.fração]`, sem fuso horário. O retorno
/// [`NaiveDateTime`] preserva a hora local quando não há fuso. Datas RFC 3339
/// com `Z` ou offset são normalizadas para UTC antes das comparações. A leitura
/// não atribui um fuso implícito às datas locais nem valida a cronologia.
///
/// # Errors
///
/// Retorna erro de extração ou de data/hora quando nenhum formato for aceito.
fn parse_datetime<'xml>(
    document: &XmlDocument<'xml>,
    element: ElementId,
) -> Result<NaiveDateTime, XmlParseError<'xml>> {
    let text = document.text(element)?;
    ["%Y-%m-%dT%H:%M:%S", "%Y-%m-%dT%H:%M:%S%.f"]
        .into_iter()
        .find_map(|format| NaiveDateTime::parse_from_str(text, format).ok())
        .or_else(|| chrono::DateTime::parse_from_rfc3339(text).ok().map(|date| date.naive_utc()))
        .ok_or_else(|| {
            document.error(
                document.value_span(element),
                format!("data e hora `{text}` inválida; esperado AAAA-MM-DDThh:mm:ss, com fração e fuso opcionais"),
            )
        })
}

/// Converte as representações booleanas aceitas pelo formato SDP.
///
/// Aceita somente `1` ou `0`, conforme o campo `FlagPossuiColetaResiduo`.
///
/// # Errors
///
/// Retorna erro se o texto não puder ser extraído ou não for uma dessas grafias.
fn parse_bool<'xml>(
    document: &XmlDocument<'xml>,
    element: ElementId,
) -> Result<bool, XmlParseError<'xml>> {
    let text = document.text(element)?;
    match text {
        "1" => Ok(true),
        "0" => Ok(false),
        _ => Err(document.error(
            document.value_span(element),
            format!("booleano `{text}` inválido; esperado 0 ou 1"),
        )),
    }
}

/// Converte `C` em `true` e `V` em `false`.
///
/// # Errors
///
/// Retorna erro de extração ou quando o valor não for exatamente `C` ou `V`.
fn parse_cheio_vazio<'xml>(
    document: &XmlDocument<'xml>,
    element: ElementId,
) -> Result<bool, XmlParseError<'xml>> {
    let text = document.text(element)?;
    match text {
        "C" => Ok(true),
        "V" => Ok(false),
        _ => Err(document.error(
            document.value_span(element),
            format!("valor `{text}` inválido; esperado C (cheio) ou V (vazio)"),
        )),
    }
}

/// Extrai o texto de um elemento opcional já localizado.
///
/// # Errors
///
/// Propaga falhas de [`XmlDocument::text`] somente se o ID existir; `None`
/// produz `Ok(None)` sem procurar outras tags.
fn optional_text<'xml>(
    document: &XmlDocument<'xml>,
    element: Option<ElementId>,
) -> Result<Option<&'xml str>, XmlParseError<'xml>> {
    element.map(|element| document.text(element)).transpose()
}

/// Converte o valor de um elemento opcional já localizado.
///
/// # Errors
///
/// Propaga falhas de extração ou conversão para `T` quando o ID existir.
/// A ausência do ID produz `Ok(None)`.
fn optional_parse<'xml, T>(
    document: &XmlDocument<'xml>,
    element: Option<ElementId>,
    expected: &str,
) -> Result<Option<T>, XmlParseError<'xml>>
where
    T: FromStr,
    T::Err: fmt::Display,
{
    element
        .map(|element| document.parse(element, expected))
        .transpose()
}

/// Decodifica um elemento `<TemposDescontos>` e registra seus elementos-fonte.
///
/// Converte o código em `u32` e as datas, sem validar códigos normativos nem
/// duração ou relação cronológica com a atracação.
///
/// # Errors
///
/// Retorna a primeira falha de estrutura, cardinalidade ou conversão dos campos.
fn parse_tempos_descontos<'xml>(
    document: &XmlDocument<'xml>,
    element: ElementId,
) -> Result<(TemposDescontos, TemposDescontosSourceMap), XmlParseError<'xml>> {
    document.ensure_only_children(element, &["Codigo", "DataHoraInicio", "DataHoraTermino"])?;

    let codigo = document.required_child(element, &["Codigo"])?;
    let data_hora_inicio = document.required_child(element, &["DataHoraInicio"])?;
    let data_hora_termino = document.required_child(element, &["DataHoraTermino"])?;

    Ok((
        TemposDescontos {
            codigo: document.parse(codigo, "u32")?,
            data_hora_inicio: parse_datetime(document, data_hora_inicio)?,
            data_hora_termino: parse_datetime(document, data_hora_termino)?,
        },
        TemposDescontosSourceMap {
            element,
            codigo,
            data_hora_inicio,
            data_hora_termino,
        },
    ))
}

/// Decodifica `<CargaConteinerizada>` e registra a origem de todos os campos.
///
/// Somente `IDConteiner` é incondicionalmente obrigatório neste nível. As
/// parcelas própria e de terceiros são admitidas apenas no leiaute TUP;
/// exigências dependentes do estado cheio/vazio pertencem à validação.
///
/// # Errors
///
/// Retorna a primeira tag inesperada, ausência obrigatória, duplicidade ou
/// falha de conversão; não verifica formato do identificador nem soma de pesos.
fn parse_carga_conteinerizada<'xml>(
    document: &XmlDocument<'xml>,
    interner: &mut Interner<'xml>,
    element: ElementId,
    leiaute: LeiauteSdp,
) -> Result<(CargaConteinerizada<'xml>, CargaConteinerizadaSourceMap), XmlParseError<'xml>> {
    let mut allowed = vec![
        "IDConteiner",
        "CEMercante",
        "CodigoCargaConteinerizada",
        "PesoCargaLiquida",
        "ImdgCode",
    ];
    if leiaute == LeiauteSdp::Tup {
        allowed.extend(["PesoCargaLiquidaPropria", "PesoCargaLiquidaTerceiros"]);
    }
    document.ensure_only_children(element, &allowed)?;

    let id_conteiner = document.required_child(element, &["IDConteiner"])?;
    let ce_mercante = document.optional_child(element, &["CEMercante"])?;
    let codigo_carga_conteinerizada =
        document.optional_child(element, &["CodigoCargaConteinerizada"])?;
    let peso_carga_liquida = document.optional_child(element, &["PesoCargaLiquida"])?;
    let peso_carga_liquida_propria =
        document.optional_child(element, &["PesoCargaLiquidaPropria"])?;
    let peso_carga_liquida_terceiros =
        document.optional_child(element, &["PesoCargaLiquidaTerceiros"])?;
    let imdg_code = document.optional_child(element, &["ImdgCode"])?;

    Ok((
        CargaConteinerizada {
            id_conteiner: interner.entry(document.text(id_conteiner)?).or_insert(),
            ce_mercante: optional_text(document, ce_mercante)?,
            codigo_carga_conteinerizada: optional_text(document, codigo_carga_conteinerizada)?
                .map(|texto| interner.entry(texto).or_insert()),
            peso_carga_liquida: optional_parse(document, peso_carga_liquida, "peso em toneladas")?,
            peso_carga_liquida_propria: optional_parse(
                document,
                peso_carga_liquida_propria,
                "peso em toneladas",
            )?,
            peso_carga_liquida_terceiros: optional_parse(
                document,
                peso_carga_liquida_terceiros,
                "peso em toneladas",
            )?,
            imdg_code: optional_parse(document, imdg_code, "u8")?,
        },
        CargaConteinerizadaSourceMap {
            element,
            codigo_carga_conteinerizada,
            ce_mercante,
            peso_carga_liquida,
            peso_carga_liquida_propria,
            peso_carga_liquida_terceiros,
            id_conteiner,
            imdg_code,
        },
    ))
}

/// Decodifica `<Carga>` e acumula mapas planos de carga e contêiner.
///
/// Os mapas dos contêineres são anexados a `source_maps.conteineres`; o mapa da
/// carga guarda apenas o intervalo correspondente nesse vetor.
/// Nenhum detalhamento produz `None` no modelo, não `Some(Vec::new())`.
///
/// # Errors
///
/// Retorna a primeira falha estrutural ou de conversão da carga e de seus
/// detalhamentos. Uma falha pode deixar entradas parciais em `source_maps`;
/// o chamador deve descartar essa construção, como faz [`parse_atracacoes`].
fn parse_carga<'xml>(
    document: &XmlDocument<'xml>,
    interner: &mut Interner<'xml>,
    element: ElementId,
    leiaute: LeiauteSdp,
    source_maps: &mut SdpSourceMap,
) -> Result<(Carga<'xml>, CargaSourceMap), XmlParseError<'xml>> {
    let mut allowed = vec![
        "CnpjOperador",
        "CodigoCarga",
        "CodigoNCM",
        "CheioVazio",
        "CEMercante",
        "NaturezaCarga",
        "Sentido",
        "Navegacao",
        "CodigoTUPOrigem",
        "BigramaPaisOrigem",
        "TrigramaPortoOrigem",
        "CodigoAreaOrigem",
        "CodigoTUPDestino",
        "BigramaPaisDestino",
        "TrigramaPortoDestino",
        "CodigoAreaDestino",
        "Quantidade",
        "PesoCargaBruta",
        "TipoOperacaoCarga",
        "ImdgCode",
        "CargaConteinerizada",
    ];
    if leiaute == LeiauteSdp::Tup {
        allowed.extend(["PesoCargaBrutaPropria", "PesoCargaBrutaTerceiros"]);
    }
    document.ensure_only_children(element, &allowed)?;

    let cnpj_operador = document.required_child(element, &["CnpjOperador"])?;
    let codigo_carga = document.required_child(element, &["CodigoCarga"])?;
    let codigo_ncm = document.optional_child(element, &["CodigoNCM"])?;
    let cheio_vazio = document.optional_child(element, &["CheioVazio"])?;
    let ce_mercante = document.optional_child(element, &["CEMercante"])?;
    let natureza_carga = document.required_child(element, &["NaturezaCarga"])?;
    let sentido = document.optional_child(element, &["Sentido"])?;
    let navegacao = document.optional_child(element, &["Navegacao"])?;
    let codigo_tup_origem = document.optional_child(element, &["CodigoTUPOrigem"])?;
    let bigrama_pais_origem = document.optional_child(element, &["BigramaPaisOrigem"])?;
    let trigrama_porto_origem = document.optional_child(element, &["TrigramaPortoOrigem"])?;
    let codigo_area_origem = document.optional_child(element, &["CodigoAreaOrigem"])?;
    let codigo_tup_destino = document.optional_child(element, &["CodigoTUPDestino"])?;
    let bigrama_pais_destino = document.optional_child(element, &["BigramaPaisDestino"])?;
    let trigrama_porto_destino = document.optional_child(element, &["TrigramaPortoDestino"])?;
    let codigo_area_destino = document.optional_child(element, &["CodigoAreaDestino"])?;
    let quantidade = document.optional_child(element, &["Quantidade"])?;
    let peso_carga_bruta = document.required_child(element, &["PesoCargaBruta"])?;
    let peso_carga_bruta_propria = document.optional_child(element, &["PesoCargaBrutaPropria"])?;
    let peso_carga_bruta_terceiros =
        document.optional_child(element, &["PesoCargaBrutaTerceiros"])?;
    let tipo_operacao_carga = document.required_child(element, &["TipoOperacaoCarga"])?;
    let imdg_code = document.required_child(element, &["ImdgCode"])?;

    let container_start = source_maps.conteineres.len();
    let mut conteineres = Vec::new();
    for child in document.children_named_any(element, &["CargaConteinerizada"]) {
        let (conteiner, source_map) =
            parse_carga_conteinerizada(document, interner, child, leiaute)?;
        conteineres.push(conteiner);
        source_maps.conteineres.push(source_map);
    }
    let container_end = source_maps.conteineres.len();

    let carga = Carga {
        cnpj_operador: interner.entry(document.text(cnpj_operador)?).or_insert(),
        codigo_carga: interner.entry(document.text(codigo_carga)?).or_insert(),
        codigo_ncm: optional_text(document, codigo_ncm)?
            .map(|texto| interner.entry(texto).or_insert()),
        cheio_vazio: cheio_vazio
            .map(|element| parse_cheio_vazio(document, element))
            .transpose()?,
        ce_mercante: optional_text(document, ce_mercante)?,
        natureza_carga: parse_enum(document, natureza_carga, "NaturezaCarga (1..=3)")?,
        sentido: sentido
            .map(|id| parse_enum(document, id, "Sentido (0..=2)"))
            .transpose()?,
        navegacao: navegacao
            .map(|id| parse_enum(document, id, "Navegacao (0..=5)"))
            .transpose()?,
        codigo_tup_origem: optional_text(document, codigo_tup_origem)?
            .map(|texto| interner.entry(texto).or_insert()),
        bigrama_pais_origem: optional_text(document, bigrama_pais_origem)?
            .map(|texto| interner.entry(texto).or_insert()),
        trigrama_porto_origem: optional_text(document, trigrama_porto_origem)?
            .map(|texto| interner.entry(texto).or_insert()),
        codigo_area_origem: optional_text(document, codigo_area_origem)?
            .map(|texto| interner.entry(texto).or_insert()),
        codigo_tup_destino: optional_text(document, codigo_tup_destino)?
            .map(|texto| interner.entry(texto).or_insert()),
        bigrama_pais_destino: optional_text(document, bigrama_pais_destino)?
            .map(|texto| interner.entry(texto).or_insert()),
        trigrama_porto_destino: optional_text(document, trigrama_porto_destino)?
            .map(|texto| interner.entry(texto).or_insert()),
        codigo_area_destino: optional_text(document, codigo_area_destino)?
            .map(|texto| interner.entry(texto).or_insert()),
        quantidade: optional_parse(document, quantidade, "u32")?,
        peso_carga_bruta: document.parse(peso_carga_bruta, "peso em toneladas")?,
        peso_carga_bruta_propria: optional_parse(
            document,
            peso_carga_bruta_propria,
            "peso em toneladas",
        )?,
        peso_carga_bruta_terceiros: optional_parse(
            document,
            peso_carga_bruta_terceiros,
            "peso em toneladas",
        )?,
        tipo_operacao_carga: parse_enum(document, tipo_operacao_carga, "TipoOperacaoCarga")?,
        imdg_code: document.parse(imdg_code, "u8")?,
        carga_conteinerizada: (!conteineres.is_empty()).then_some(conteineres),
    };

    let source_map = CargaSourceMap {
        element,
        cnpj_operador,
        codigo_carga,
        codigo_ncm,
        cheio_vazio,
        ce_mercante,
        natureza_carga,
        sentido,
        navegacao,
        bigrama_pais_origem,
        trigrama_porto_origem,
        codigo_tup_origem,
        codigo_area_origem,
        bigrama_pais_destino,
        trigrama_porto_destino,
        codigo_tup_destino,
        codigo_area_destino,
        quantidade,
        peso_carga_bruta,
        peso_carga_bruta_propria,
        peso_carga_bruta_terceiros,
        tipo_operacao_carga,
        imdg_code,
        conteineres: SourceMapRange::new(container_start, container_end),
    };

    Ok((carga, source_map))
}

/// Decodifica um certificado e seus itens, sem avaliar cadastros ou relações.
///
/// # Errors
/// Retorna a primeira falha estrutural, de cardinalidade ou conversão lexical.
fn parse_coleta_residuo<'xml>(
    document: &XmlDocument<'xml>,
    element: ElementId,
    mapas: &mut SdpSourceMap,
) -> Result<
    (
        crate::xml::models::residuos::ColetaResiduo<'xml>,
        super::ColetaResiduoSourceMap,
    ),
    XmlParseError<'xml>,
> {
    use super::{ColetaResiduoSourceMap, ResiduoSourceMap};
    use crate::xml::models::residuos::{ColetaResiduo, Residuo};
    document.ensure_only_children(
        element,
        &[
            "NumeroCertificado",
            "CNPJColetor",
            "DataHoraInicioColeta",
            "DataHoraTerminoColeta",
            "DataHoraEntregaColeta",
            "CNPJReceptor",
            "Residuo",
        ],
    )?;
    let numero_certificado = document.required_child(element, &["NumeroCertificado"])?;
    let cnpj_coletor = document.required_child(element, &["CNPJColetor"])?;
    let data_hora_inicio_coleta = document.required_child(element, &["DataHoraInicioColeta"])?;
    let data_hora_termino_coleta = document.required_child(element, &["DataHoraTerminoColeta"])?;
    let data_hora_entrega_coleta = document.required_child(element, &["DataHoraEntregaColeta"])?;
    let cnpj_receptor = document.required_child(element, &["CNPJReceptor"])?;
    let inicio = mapas.residuos.len();
    let mut residuos = Vec::new();
    for child in document.children_named_any(element, &["Residuo"]) {
        document.ensure_only_children(
            child,
            &["TipoResiduo", "QTDSolicitado", "QTDColetado", "TipoUnidade"],
        )?;
        let tipo_residuo = document.required_child(child, &["TipoResiduo"])?;
        let qtd_solicitado = document.required_child(child, &["QTDSolicitado"])?;
        let qtd_coletado = document.required_child(child, &["QTDColetado"])?;
        let tipo_unidade = document.required_child(child, &["TipoUnidade"])?;
        residuos.push(Residuo {
            tipo_residuo: document.parse(tipo_residuo, "inteiro não negativo")?,
            qtd_solicitado: document
                .parse(qtd_solicitado, "quantidade decimal com até três casas")?,
            qtd_coletado: document.parse(qtd_coletado, "quantidade decimal com até três casas")?,
            tipo_unidade: document.parse(tipo_unidade, "inteiro não negativo")?,
        });
        mapas.residuos.push(ResiduoSourceMap {
            element: child,
            tipo_residuo,
            qtd_solicitado,
            qtd_coletado,
            tipo_unidade,
        });
    }
    let residuos_range = SourceMapRange::new(inicio, mapas.residuos.len());
    Ok((
        ColetaResiduo {
            numero_certificado: document.text(numero_certificado)?,
            cnpj_coletor: document.text(cnpj_coletor)?,
            data_hora_inicio_coleta: parse_datetime(document, data_hora_inicio_coleta)?,
            data_hora_termino_coleta: parse_datetime(document, data_hora_termino_coleta)?,
            data_hora_entrega_coleta: parse_datetime(document, data_hora_entrega_coleta)?,
            cnpj_receptor: document.text(cnpj_receptor)?,
            residuos,
        },
        ColetaResiduoSourceMap {
            element,
            numero_certificado,
            cnpj_coletor,
            data_hora_inicio_coleta,
            data_hora_termino_coleta,
            data_hora_entrega_coleta,
            cnpj_receptor,
            residuos: residuos_range,
        },
    ))
}

/// Decodifica `<Atracacao>` e acumula todos os mapas de origem descendentes.
///
/// Cargas e tempos de desconto são anexados às tabelas planas globais. O mapa
/// retornado contém intervalos para essas tabelas, mantendo a correspondência
/// com os vetores aninhados do modelo tipado.
///
/// # Errors
///
/// Retorna a primeira falha estrutural ou de conversão da atracação ou de seus
/// descendentes. Os mapas acumulados até a falha não sofrem rollback e devem
/// ser descartados junto à construção incompleta.
fn parse_atracacao<'xml>(
    document: &XmlDocument<'xml>,
    interner: &mut Interner<'xml>,
    element: ElementId,
    leiaute: LeiauteSdp,
    source_maps: &mut SdpSourceMap,
) -> Result<(Atracacao<'xml>, AtracacaoSourceMap), XmlParseError<'xml>> {
    document.ensure_only_children(
        element,
        &[
            "TrigramaPorto",
            "CodigoTUP",
            "NumeroAtracacao",
            "NREscala",
            "LocalAtracacao",
            "IMON",
            "NumeroInscricaoCapitania",
            "Navegacao",
            "TipoOperacao",
            "CnpjAgente",
            "NacionalidadeArmador",
            "CnpjCodigoArmador",
            "DataHoraChegada",
            "DataHoraPrevistaAtracacao",
            "DataHoraAtracacao",
            "DataHoraDesatracacao",
            "DataHoraInicioOperacao",
            "DataHoraTerminoOperacao",
            "TemposDescontos",
            "QuantidadePassageirosEmbarcados",
            "QuantidadePassageirosDesembarcados",
            "QuantidadePassageirosTransito",
            "FlagPossuiColetaResiduo",
            "ColetaResiduo",
            "Carga",
        ],
    )?;

    let trigrama_porto = document.optional_child(element, &["TrigramaPorto"])?;
    let codigo_tup = document.optional_child(element, &["CodigoTUP"])?;
    let numero_atracacao = document.required_child(element, &["NumeroAtracacao"])?;
    let nr_escala = document.optional_child(element, &["NREscala"])?;
    let local_atracacao = document.required_child(element, &["LocalAtracacao"])?;
    let imon = document.required_child(element, &["IMON"])?;
    let numero_inscricao_capitania =
        document.optional_child(element, &["NumeroInscricaoCapitania"])?;
    let navegacao = document.required_child(element, &["Navegacao"])?;
    let tipo_operacao = document.required_child(element, &["TipoOperacao"])?;
    let cnpj_agente = document.required_child(element, &["CnpjAgente"])?;
    let nacionalidade_armador = document.required_child(element, &["NacionalidadeArmador"])?;
    let cnpj_codigo_armador = document.required_child(element, &["CnpjCodigoArmador"])?;
    let data_hora_chegada = document.required_child(element, &["DataHoraChegada"])?;
    let data_hora_prevista_atracacao =
        document.required_child(element, &["DataHoraPrevistaAtracacao"])?;
    let data_hora_atracacao = document.required_child(element, &["DataHoraAtracacao"])?;
    let data_hora_desatracacao = document.required_child(element, &["DataHoraDesatracacao"])?;
    let data_hora_inicio_operacao =
        document.optional_child(element, &["DataHoraInicioOperacao"])?;
    let data_hora_termino_operacao =
        document.optional_child(element, &["DataHoraTerminoOperacao"])?;
    let quantidade_passageiros_embarcados =
        document.optional_child(element, &["QuantidadePassageirosEmbarcados"])?;
    let quantidade_passageiros_desembarcados =
        document.optional_child(element, &["QuantidadePassageirosDesembarcados"])?;
    let quantidade_passageiros_transito =
        document.optional_child(element, &["QuantidadePassageirosTransito"])?;
    let flag_possui_coleta_residuo =
        document.required_child(element, &["FlagPossuiColetaResiduo"])?;

    let desconto_start = source_maps.tempos_descontos.len();
    let mut descontos = Vec::new();
    for child in document.children_named_any(element, &["TemposDescontos"]) {
        let (desconto, source_map) = parse_tempos_descontos(document, child)?;
        descontos.push(desconto);
        source_maps.tempos_descontos.push(source_map);
    }
    let desconto_end = source_maps.tempos_descontos.len();

    let coleta_start = source_maps.coletas_residuos.len();
    let mut coletas_residuos = Vec::new();
    for child in document.children_named_any(element, &["ColetaResiduo"]) {
        let (coleta, mapa) = parse_coleta_residuo(document, child, source_maps)?;
        coletas_residuos.push(coleta);
        source_maps.coletas_residuos.push(mapa);
    }
    let coleta_end = source_maps.coletas_residuos.len();

    let carga_start = source_maps.cargas.len();
    let mut cargas = Vec::new();
    for child in document.children_named_any(element, &["Carga"]) {
        let (carga, source_map) = parse_carga(document, interner, child, leiaute, source_maps)?;
        cargas.push(carga);
        source_maps.cargas.push(source_map);
    }
    let carga_end = source_maps.cargas.len();

    let atracacao = Atracacao {
        trigrama_porto: optional_text(document, trigrama_porto)?
            .map(|texto| interner.entry(texto).or_insert()),
        codigo_tup: optional_text(document, codigo_tup)?
            .map(|texto| interner.entry(texto).or_insert()),
        numero_atracacao: interner.entry(document.text(numero_atracacao)?).or_insert(),
        nr_escala: optional_text(document, nr_escala)?,
        local_atracacao: interner.entry(document.text(local_atracacao)?).or_insert(),
        imon: document.text(imon)?,
        numero_inscricao_capitania: optional_text(document, numero_inscricao_capitania)?,
        navegacao: parse_enum(document, navegacao, "Navegacao (0..=5)")?,
        tipo_operacao: parse_enum(document, tipo_operacao, "TipoOperacao (1..=8)")?,
        cnpj_agente: interner.entry(document.text(cnpj_agente)?).or_insert(),
        nacionalidade_armador: parse_enum(
            document,
            nacionalidade_armador,
            "NacionalidadeArmador (1..=2)",
        )?,
        cnpj_codigo_armador: interner
            .entry(document.text(cnpj_codigo_armador)?)
            .or_insert(),
        data_hora_chegada: parse_datetime(document, data_hora_chegada)?,
        data_hora_prevista_atracacao: parse_datetime(document, data_hora_prevista_atracacao)?,
        data_hora_atracacao: parse_datetime(document, data_hora_atracacao)?,
        data_hora_desatracacao: parse_datetime(document, data_hora_desatracacao)?,
        data_hora_inicio_operacao: data_hora_inicio_operacao
            .map(|id| parse_datetime(document, id))
            .transpose()?,
        data_hora_termino_operacao: data_hora_termino_operacao
            .map(|id| parse_datetime(document, id))
            .transpose()?,
        tempos_descontos: (!descontos.is_empty()).then_some(descontos),
        quantidade_passageiros_embarcados: optional_parse(
            document,
            quantidade_passageiros_embarcados,
            "u32",
        )?,
        quantidade_passageiros_desembarcados: optional_parse(
            document,
            quantidade_passageiros_desembarcados,
            "u32",
        )?,
        quantidade_passageiros_transito: optional_parse(
            document,
            quantidade_passageiros_transito,
            "u32",
        )?,
        flag_possui_coleta_residuo: parse_bool(document, flag_possui_coleta_residuo)?,
        coletas_residuos,
        cargas,
    };

    let source_map = AtracacaoSourceMap {
        element,
        trigrama_porto,
        codigo_tup,
        numero_atracacao,
        nr_escala,
        local_atracacao,
        imon,
        numero_inscricao_capitania,
        navegacao,
        tipo_operacao,
        cnpj_agente,
        nacionalidade_armador,
        cnpj_codigo_armador,
        data_hora_chegada,
        data_hora_prevista_atracacao,
        data_hora_atracacao,
        data_hora_desatracacao,
        data_hora_inicio_operacao,
        data_hora_termino_operacao,
        tempos_descontos: SourceMapRange::new(desconto_start, desconto_end),
        quantidade_passageiros_embarcados,
        quantidade_passageiros_desembarcados,
        quantidade_passageiros_transito,
        flag_possui_coleta_residuo,
        coletas_residuos: SourceMapRange::new(coleta_start, coleta_end),
        cargas: SourceMapRange::new(carga_start, carga_end),
    };

    Ok((atracacao, source_map))
}

/// Lê o documento, preserva sua origem e converte as atracações.
///
/// A função verifica apenas sintaxe, estrutura e conversibilidade dos valores.
/// Regras entre campos devem ser executadas posteriormente por
/// [`crate::validation::Validador::validar_atracacoes`].
///
/// # Retorno e empréstimos
///
/// Retorna modelos, mapas de IDs, tokens, linhas lógicas e índice de linhas
/// físicas. Os textos são fatias de `xml`: o chamador deve conservar a string
/// original e o interner que registra seus identificadores repetitivos.
///
/// A fonte inteira já deve estar em memória e em UTF-8; não há leitura de
/// arquivo, consulta externa ou processamento incremental por esta API.
/// Requer o namespace padrão `SDPv2.xsd` ou `SDPTup.xsd` na raiz, sem carregar
/// esses XSDs. Preserva a ordem das atracações e aceita uma raiz sem atracações.
/// Os spans referem-se à grafia original; entidades textuais não são expandidas.
///
/// # Errors
///
/// Retorna [`XmlParseError`] se o documento for malformado, não possuir a raiz
/// `<Movimento>`, contiver uma estrutura inesperada ou algum valor não puder
/// ser convertido para o tipo correspondente. Também rejeita fontes maiores
/// que o limite de [`crate::source::SourceFile`], raízes adicionais e namespace
/// ausente ou desconhecido. Não há resultado parcial: retorna a primeira falha,
/// não necessariamente a de menor offset entre todas as regras estruturais.
pub fn parse_atracacoes<'xml>(
    xml: &'xml str,
    interner: &mut Interner<'xml>,
) -> Result<ParsedAtracacoes<'xml>, XmlParseError<'xml>> {
    let document = read_document(xml)?;

    let Some(&movimento) = document
        .roots
        .iter()
        .find(|&root| document.name(*root) == "Movimento")
    else {
        return Err(document.error(Span::START, "elemento raiz <Movimento> ausente"));
    };

    if let Some(&extra_root) = document.roots.iter().find(|&root| *root != movimento) {
        return Err(document.error(
            document.start_span(extra_root),
            format!("elemento raiz adicional <{}>", document.name(extra_root)),
        ));
    }

    let namespace = document.namespace(movimento).ok_or_else(|| {
        document.error(
            document.start_span(movimento),
            "namespace de <Movimento> ausente; esperado SDPv2.xsd ou SDPTup.xsd",
        )
    })?;
    let leiaute = LeiauteSdp::from_namespace(namespace).ok_or_else(|| {
        document.error(
            document.start_span(movimento),
            format!("namespace `{namespace}` não reconhecido; esperado SDPv2.xsd ou SDPTup.xsd"),
        )
    })?;

    document.ensure_only_children(movimento, &["Atracacao"])?;
    let mut source_maps = SdpSourceMap {
        movimento,
        atracacoes: Vec::new(),
        tempos_descontos: Vec::new(),
        cargas: Vec::new(),
        conteineres: Vec::new(),
        coletas_residuos: Vec::new(),
        residuos: Vec::new(),
    };
    let mut atracacoes = Vec::new();

    for element in document.children_named_any(movimento, &["Atracacao"]) {
        let (atracacao, source_map) =
            parse_atracacao(&document, interner, element, leiaute, &mut source_maps)?;
        atracacoes.push(atracacao);
        source_maps.atracacoes.push(source_map);
    }

    let (source, xml_source_map) = document.into_source_parts();

    Ok(ParsedAtracacoes {
        leiaute,
        atracacoes,
        source,
        xml_source_map,
        sdp_source_map: source_maps,
    })
}
